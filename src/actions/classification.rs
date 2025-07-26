// TODO
#![allow(dead_code)]

use std::marker::PhantomData;

use crate::session::{AiModel, ConversationContext, InputMode, ModelInput, OutputMode};

use super::Intent;

pub struct IntentClassifier<I: InputMode, O: OutputMode, M: AiModel<I, O>> {
    model: M,

    i: PhantomData<I>,
    o: PhantomData<O>,
}

impl<I, O, M> IntentClassifier<I, O, M>
where
    I: InputMode + Clone,
    O: OutputMode,
    M: AiModel<I, O>,
{
    pub fn classify_intent(
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
                Intent::Change => "- change: The user told the assistant to edit the code",
            })
            .fold(String::default(), |l, r| l + "\n" + r);

        let excerpt = "todo".to_owned();

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
            Classification: ask\n\
            "
        )
        .to_owned();
        let model_input = ModelInput {
            instructions,
            log: recent_context.log.clone(),
        };
        let TODO = self.model.send(model_input);
        todo!()
    }
}
