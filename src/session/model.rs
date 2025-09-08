// TODO session/model is not a good place for all the LLM related models

use std::future::Future;

pub struct ConversationContext<I: InputModes> {
    pub log: Vec<I>,
    // current code change
    // code vicinity (current and related files or parts of it?)
}

// TODO this implementation is kind of useless, but right now I don't have a
// better method to send text messages to a model that allows texts or images
// as input
impl From<ConversationContext<TextMessage>> for ConversationContext<TextImageMessage> {
    fn from(ctx: ConversationContext<TextMessage>) -> Self {
        let log = ctx
            .log
            .iter()
            .map(|msg| {
                let parts = msg
                    .parts
                    .iter()
                    .map(|s| TextImage::Text(s.clone()))
                    .collect();
                TextImageMessage {
                    author: msg.author.clone(),
                    parts,
                }
            })
            .collect();
        ConversationContext { log }
    }
}

pub trait AIModel<I: InputModes, O: OutputModes> {
    fn send(&self, i: ModelInput<I>) -> impl Future<Output = anyhow::Result<ModelOutput<O>>>;
}

pub struct ModelInput<I: InputModes> {
    pub instructions: String,
    pub log: Vec<I>,
}

pub struct ModelOutput<O: OutputModes> {
    pub items: Vec<O>,
}

pub trait InputModes {}
pub trait OutputModes {}

pub trait TextMode {}
pub trait ImageMode {}

pub struct TextMessage {
    pub author: Author,
    pub parts: Vec<String>,
}
impl InputModes for TextMessage {}
impl OutputModes for TextMessage {}
impl TextMode for TextMessage {}

#[derive(Clone)]
pub struct TextImageMessage {
    pub author: Author,
    pub parts: Vec<TextImage>,
}
#[derive(Clone)]
pub enum TextImage {
    Text(String),
    Base64Image(String),
    ImageUrl(String),
}
impl InputModes for TextImageMessage {}
impl OutputModes for TextImageMessage {}
impl TextMode for TextImageMessage {}
impl ImageMode for TextImageMessage {}

#[derive(Clone)]
pub enum Author {
    User,
    Assistant,
}
