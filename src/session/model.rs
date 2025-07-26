pub struct ConversationContext<I: InputMode> {
    pub log: Vec<I>,
    // current code change
    // code vicinity (current and related files or parts of it?)
}

pub trait AiModel<I: InputMode, O: OutputMode> {
    fn send(&self, i: ModelInput<I>) -> anyhow::Result<O>;
}

pub struct ModelInput<I: InputMode> {
    pub instructions: String,
    pub log: Vec<I>,
}

pub trait InputMode {}
pub trait OutputMode {}

pub trait TextMode: InputMode + OutputMode {}
pub trait ImageMode: InputMode + OutputMode {}

pub enum TextImage {
    Text(TextMessage),
    Base64Image(String),
    ImageUrl(String),
}
impl InputMode for TextImage {}
impl OutputMode for TextImage {}
impl TextMode for TextImage {}
impl ImageMode for TextImage {}

pub struct TextMessage {
    pub author: Author,
    pub text: String,
}

pub enum Author {
    User,
    Developer, // aka system
    Assistant,
}
