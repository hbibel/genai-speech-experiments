// TODO
#![allow(dead_code)]

use std::marker::PhantomData;

use crate::session::{
    AIModel, Author, ConversationContext, InputModes, ModelInput, ModelOutput, OutputModes,
    TextImageMessage, TextMessage,
};

use super::Intent;

pub struct IntentClassifier<I: InputModes, O: OutputModes, M: AIModel<I, O>> {
    model: M,

    i: PhantomData<I>,
    o: PhantomData<O>,
}

// pub trait CanRender {
//     /*...*/
// }
// pub trait CanRenderToText {
//     /*...*/
// }
// pub trait CanRenderToImage {
//     /*...*/
// }
// pub trait CanRenderToTextOrImage {
//     /*...*/
// }
// pub trait Renderer<I: CanRender> {
//     fn render(&self, i: RenderInput<I>);
// }
// pub struct Foo<I, R>
// where
//     I: CanRender,
//     R: Renderer<I>,
// {
//     renderer: R,
//     i: PhantomData<I>,
// }
// impl<I, R> Foo<I, R>
// where
//     I: CanRender,
//     R: Renderer<I>,
// {
//     pub fn do_sth(&self, my_input: MyInput<I>) {
//         /*...*/
//     }
// }

// TODO is it an issue that RenderForExcerpt is more private than IntentClassifier?
#[allow(private_bounds)]
impl<I, O, M> IntentClassifier<I, O, M>
where
    I: InputModes + RenderForExcerpt + Clone,
    O: OutputModes + HasResponseText<O>,
    M: AIModel<I, O>,
{
    pub fn new(model: M) -> Self {
        Self {
            model,
            i: PhantomData,
            o: PhantomData,
        }
    }

    pub async fn classify_intent(
        &self,
        recent_context: &ConversationContext<I>,
    ) -> anyhow::Result<Intent> {
        let intents = Intent::variants()
            .iter()
            .map(|intent| match intent {
                Intent::Unclear => {
                    "- unclear: It's not clear if or what the user wants the \
                    assistant to do"
                }
                Intent::Nothing => {
                    "- nothing: The user does not want the AI to do anything, \
                    for example they just made a general remark, or talked to \
                    themselves while thinking"
                }
                Intent::Ask => {
                    "- ask: The user expects the assistant to tell or explain \
                    something to the user"
                }
                Intent::Brainstorm => {
                    "- brainstorm: The user has asked an open-ended question \
                    and wants to gather ideas"
                }
                Intent::Change => {
                    "- change: The user told the assistant to \
                    edit the code"
                }
            })
            .fold(String::default(), |l, r| l + "\n" + r);

        let excerpt = recent_context
            .log
            .iter()
            .map(RenderForExcerpt::render)
            .fold(String::default(), |l, r| l + "\n" + &r);

        let instructions = format!(
            "The following is an excerpt from a conversation between a user \
            and an AI coding assistant. \
            Given this excerpt, Your job is to classify based on the the last \
            user interaction(s) what they intent for the AI assistant to do \
            out of the following possible intentions:\n\
            {intents}\n\
            \n\
            Here is the excerpt: \"\"\"\n\
            {excerpt}\n\
            \"\"\"
            \n\
            Return only the name of the classification.\n\
            \n\
            Example:\n\
            Excerpt: <assistant>We can use XYZ<assistant><user>What is XYZ?</user>\n\
            Your response: ask\n\
            "
        )
        .to_owned();

        let model_input = ModelInput {
            instructions,
            log: recent_context.log.clone(),
        };

        let output = self.model.send(model_input).await?;
        let response = O::get_assistant_text_response(&output)?;

        println!("Response: {response}");
        todo!()
    }
}

trait RenderForExcerpt: InputModes {
    fn render(&self) -> String;
}

impl RenderForExcerpt for TextMessage {
    fn render(&self) -> String {
        let author = match self.author {
            Author::User => "user",
            Author::Assistant => "assistant",
        };
        let text = self.parts.join("\n\n");
        format!("<{author}>{text}</{author}>")
    }
}
impl RenderForExcerpt for TextImageMessage {
    fn render(&self) -> String {
        let author = match self.author {
            Author::User => "user",
            Author::Assistant => "assistant",
        };
        let text = self
            .parts
            .iter()
            .fold(String::new(), |acc, part| match part {
                crate::session::TextImage::Text(t) => acc + "\n\n" + t,
                crate::session::TextImage::Base64Image(_)
                | crate::session::TextImage::ImageUrl(_) => acc + "\n\n<image>",
            });
        format!("<{author}>{text}</{author}>")
    }
}

trait HasResponseText<O: OutputModes> {
    fn get_assistant_text_response(output: &ModelOutput<O>) -> anyhow::Result<String>;
}

impl HasResponseText<TextMessage> for TextMessage {
    fn get_assistant_text_response(output: &ModelOutput<TextMessage>) -> anyhow::Result<String> {
        output
            .items
            .last()
            .ok_or(anyhow::anyhow!("The model output appears to be empty"))
            .map(|text_item| text_item.parts.join("\n\n"))
    }
}
