use anyhow::Context;
use reqwest::Client as ReqwestClient;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::session::{
    AIModel, Author, InputModes, ModelInput, ModelOutput, OutputModes, TextImageMessage,
    TextMessage,
};

use responses_api::model_response::request::{
    AssistantMessageContentItem, AssistantMessageTextItem, Body as ResponseApiRequestBody,
    InputData, UserMessageContentItem, UserMessageTextImageItem,
};
use responses_api::model_response::response::{
    Body as ResponsesApiResponseBody, MessageOutputContent, Output,
};

pub struct Gpt4_1Nano {
    api_key: String,
}

impl Gpt4_1Nano {
    #[must_use]
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }
}

impl AIModel<TextImageMessage, TextMessage> for Gpt4_1Nano {
    async fn send(
        &self,
        input: ModelInput<TextImageMessage>,
    ) -> anyhow::Result<ModelOutput<TextMessage>> {
        let api_input: InputData<Modality_TextImage_Text> =
            InputData::Multiple(input.log.iter().map(Clone::clone).map(Into::into).collect());
        let api_input = ResponseApiRequestBody {
            include: None,
            input: Some(api_input),
            instructions: Some(input.instructions),
            model: OpenAIModelName::Gpt4_1Nano,
        };
        let client = ReqwestClient::new();
        let resp: ResponsesApiResponseBody<Modality_TextImage_Text> = client
            .post("https://api.openai.com/v1/responses")
            .bearer_auth(self.api_key.clone())
            .json(&api_input)
            .send()
            .await
            .context("Failed to reach OpenAI API")?
            .json()
            .await
            .context("Failed to gather response body as text")?;

        let items = resp
            .output
            .iter()
            .filter_map(|it| match it {
                Output::Message(msg) => Some(TextMessage {
                    author: Author::Assistant,
                    parts: msg
                        .content
                        .iter()
                        .map(|c| match c {
                            MessageOutputContent::OutputText(txt) => txt.text.clone(),
                            MessageOutputContent::Refusal(_no) => todo!(),
                        })
                        .collect(),
                }),
                _ => None,
            })
            .collect();

        Ok(ModelOutput { items })
    }
}

trait Modality {
    type In: InputModes;
    type Out: OutputModes;
    type UserMsg: UserMessageContentItem<Self::In> + DeserializeOwned + Serialize + std::fmt::Debug;
    type AssistantMsg: AssistantMessageContentItem<Self::Out>
        + DeserializeOwned
        + Serialize
        + std::fmt::Debug;
}

// Allowing non_camel_case_types because the underscores bear semantics
#[allow(non_camel_case_types)]
// Text + Image in, Text out
#[derive(Debug, Serialize)]
struct Modality_TextImage_Text();
impl Modality for Modality_TextImage_Text {
    type In = TextImageMessage;
    type Out = TextMessage;
    type UserMsg = UserMessageTextImageItem;
    type AssistantMsg = AssistantMessageTextItem;
}

#[derive(Debug, Serialize, Deserialize)]
enum OpenAIModelName {
    // TODO this string should probably come from config
    #[serde(rename = "gpt-4.1-nano-2025-04-14")]
    Gpt4_1Nano,
}

mod responses_api {
    pub mod model_response {
        pub mod request {
            use std::collections::HashMap;

            use serde::{Deserialize, Serialize};

            use crate::session::{
                Author, InputModes, OutputModes, TextImage, TextImageMessage, TextMessage,
            };

            use super::super::super::{Modality, OpenAIModelName};

            #[derive(Debug, Serialize, Deserialize)]
            #[serde(rename_all = "snake_case")]
            pub struct Body<M: Modality> {
                // #[serde(default = "some_false")] // implement fn some_false if needed
                // background: Option<bool>,
                //
                pub include: Option<Vec<OutputOptions>>,

                #[serde(default)]
                pub input: Option<InputData<M>>,

                pub instructions: Option<String>,

                // max_output_tokens: Option<u32>,
                //
                // max_tool_calls: Option<u32>,
                //
                // metadata: Option<HashMap<String, String>>,
                //
                pub model: OpenAIModelName,
                //
                // #[serde(default)]
                // parallel_tool_calls: Option<bool>, // Defaults to true if None
                //
                // previous_response_id: Option<String>,
                //
                // prompt: Option<HashMap<String, serde_json::Value>>,
                //
                // prompt_cache_key: Option<String>,
                //
                // reasoning: Option<HashMap<String, serde_json::Value>>,
                //
                // safety_identifier: Option<String>,
                //
                // service_tier: Option<String>, // "auto", "default", "flex", "priority"
                //
                // #[serde(default)]
                // store: Option<bool>, // Defaults to true
                //
                // #[serde(default)]
                // stream: Option<bool>, // Defaults to false
                //
                // temperature: Option<f32>, // 0.0 to 2.0
                //
                // text: Option<HashMap<String, serde_json::Value>>,
                //
                // tool_choice: Option<ToolChoice>, // String or object
                //
                // tools: Option<Vec<serde_json::Value>>,
                //
                // top_logprobs: Option<u8>, // 0–20
                //
                // top_p: Option<f32>, // 0.0–1.0
                //
                // truncation: Option<String>, // "auto" or "disabled"
                //
                // #[serde(rename = "user", skip_serializing_if = "Option::is_none")]
                // deprecated_user: Option<String>, // Deprecated
            }

            #[derive(Debug, Serialize, Deserialize)]
            #[serde(untagged)]
            pub enum InputData<M: Modality> {
                Single(String),
                Multiple(Vec<InputItem<M>>),
            }

            #[derive(Debug, Serialize, Deserialize)]
            #[serde(tag = "type")]
            pub enum InputItem<M: Modality> {
                #[serde(rename = "item_reference")]
                Reference(InputItemReference),
                #[serde(rename = "message")]
                Message(Message<M>),
            }

            impl<M> From<TextImageMessage> for InputItem<M>
            where
                M: Modality<
                        In = TextImageMessage,
                        UserMsg = UserMessageTextImageItem,
                        AssistantMsg = AssistantMessageTextItem,
                    >,
            {
                fn from(message: TextImageMessage) -> Self {
                    let api_message = match message.author {
                        Author::User => {
                            let content = message
                                .parts
                                .iter()
                                .map(|text_image| match text_image {
                                    TextImage::Text(text) => {
                                        UserMessageTextImageItem::InputText { text: text.clone() }
                                    }
                                    TextImage::Base64Image(_) => todo!(),
                                    TextImage::ImageUrl(_) => todo!(),
                                })
                                .collect();
                            Message::UserMessage {
                                // role: Role::User,
                                status: None,
                                content,
                            }
                        }
                        Author::Assistant => {
                            let content = message
                                .parts
                                .iter()
                                .map(|text_image| match text_image {
                                    TextImage::Text(text) => AssistantMessageTextItem::OutputText {
                                        text: text.clone(),
                                        annotations: Vec::new(),
                                    },
                                    // TODO this shouldn't be required!
                                    TextImage::Base64Image(_) => todo!(),
                                    TextImage::ImageUrl(_) => todo!(),
                                })
                                .collect();
                            // TODO fields like status and ID are not tracked in the
                            // application model. I could add provider-specific metadata to
                            // the application model, but I may also want to send a
                            // conversation first to OpenAI, then continue the conversation
                            // at Anthropic and there is no guarantee that they will have
                            // the same fields.
                            Message::AssistantMessage {
                                // role: Role::Assistant,
                                status: MessageStatus::Completed,
                                content,
                                id: MessageId(String::new()),
                            }
                        }
                    };

                    InputItem::Message(api_message)
                }
            }

            #[derive(Debug, Serialize, Deserialize)]
            #[serde(tag = "role")]
            pub enum Message<M: Modality> {
                #[serde(rename = "user")]
                UserMessage {
                    status: Option<MessageStatus>,
                    content: Vec<M::UserMsg>,
                },
                #[serde(rename = "assistant")]
                AssistantMessage {
                    status: MessageStatus,
                    content: Vec<M::AssistantMsg>,
                    id: MessageId,
                },
            }

            #[derive(Debug, Serialize, Deserialize)]
            pub struct MessageId(pub String);

            #[derive(Debug, Serialize, Deserialize)]
            #[serde(rename_all = "snake_case")]
            pub enum Role {
                // Leaving out "System", as the "instructions' field in the request body
                // model makes more sense I believe
                User,
                Assistant,
            }

            #[derive(Debug, Serialize, Deserialize)]
            #[serde(rename_all = "snake_case")]
            pub enum MessageStatus {
                InProgress,
                Completed,
                Incomplete,
            }

            // ???
            #[allow(dead_code)]
            pub trait UserMessageContentItem<I: InputModes> {}

            #[derive(Debug, Serialize, Deserialize)]
            #[serde(tag = "type", rename_all = "snake_case")]
            pub enum UserMessageTextImageItem {
                InputText {
                    text: String,
                },
                InputImage {
                    #[serde(default = "ImageDetail::Auto")]
                    detail: ImageDetail,
                    file_id: Option<String>,
                    image_url: Option<String>,
                },
            }

            #[derive(Debug, Serialize, Deserialize)]
            #[serde(rename_all = "snake_case")]
            pub enum ImageDetail {
                High,
                Low,
                Auto(), // parentheses for serde default annotation
            }

            impl UserMessageContentItem<TextImageMessage> for UserMessageTextImageItem {}

            // ???
            #[allow(dead_code)]
            pub trait AssistantMessageContentItem<O: OutputModes> {}

            #[derive(Debug, Serialize, Deserialize)]
            #[serde(tag = "type", rename_all = "snake_case")]
            pub enum AssistantMessageTextItem {
                OutputText {
                    text: String,
                    annotations: Vec<Annotation>,
                },
                Refusal {
                    refusal: String,
                },
            }
            impl AssistantMessageContentItem<TextMessage> for AssistantMessageTextItem {}

            #[derive(Debug, Serialize, Deserialize)]
            #[serde(rename_all = "snake_case")]
            pub enum LiteralOutputText {
                OutputText,
            }

            #[derive(Debug, Serialize, Deserialize)]
            #[serde(tag = "type", rename_all = "snake_case")]
            pub enum Annotation {
                FileCitation {
                    file_id: String,
                    filename: String,
                    index: i32,
                },
                UrlCitation {
                    start_index: i32,
                    end_index: i32,
                    title: String,
                    url: String,
                },
                ContainerFileCitation {
                    container_id: String,
                    file_id: String,
                    filename: String,
                    start_index: i32,
                    end_index: i32,
                },
                FilePath {
                    file_id: String,
                    index: i32,
                },
            }

            #[derive(Debug, Serialize, Deserialize)]
            pub struct InputItemReference {
                pub id: String,
            }

            #[derive(Debug, Serialize, Deserialize)]
            #[serde(untagged)]
            pub enum ToolChoice {
                Choice(String),
                Config(HashMap<String, serde_json::Value>),
            }

            #[derive(Debug, Serialize, Deserialize)]
            pub enum OutputOptions {
                #[serde(rename = "code_interpreter_call.outputs")]
                CodeInterpreterCallOutputs,
                #[serde(rename = "computer_call_output.output.image_url")]
                ComputerCallOutputOutputImageUrl,
                #[serde(rename = "file_search_call.results")]
                FileSearchCallResults,
                #[serde(rename = "message.input_image.image_url")]
                MessageInputImageImageUrl,
                #[serde(rename = "message.output_text.logprobs")]
                MessageOutputTextLogprobs,
                #[serde(rename = "reasoning.encrypted_content")]
                ReasoningEncryptedContent,
            }
        }

        pub mod response {
            use std::marker::PhantomData;

            use serde::{Deserialize, Serialize};

            use super::super::super::Modality;

            #[derive(Debug, Serialize, Deserialize)]
            #[serde(rename_all = "snake_case")]
            pub struct Body<M: Modality> {
                pub id: String,
                pub output: Vec<Output>,
                pub status: Status,
                pub usage: Usage,

                #[serde(skip)]
                m: PhantomData<M>,
            }

            #[derive(Debug, Serialize, Deserialize)]
            #[serde(rename_all = "snake_case", tag = "type")]
            pub enum Output {
                Message(MessageOutput),
                FileSearchToolCall(FileSearchToolCallOutput),
                FunctionToolCall(FunctionToolCallOutput),
                WebSearchToolCall(WebSearchToolCallOutput),
                ComputerToolCall(ComputerToolCallOutput),
                Reasoning(ReasoningOutput),
                CodeInterpreterToolCall(CodeInterpreterToolCallOutput),
                LocalShellCall(LocalShellCallOutput),
                MCPToolCall(MCPToolCallOutput),
                MCPToolList(MCPToolListOutput),
                MCPApprovalRequest(MCPApprovalRequestOutput),
                CustomToolCall(CustomToolCallOutput),
            }

            #[derive(Debug, Serialize, Deserialize)]
            #[serde(rename_all = "snake_case")]
            pub struct MessageOutput {
                pub id: String,
                pub content: Vec<MessageOutputContent>,
            }

            #[derive(Debug, Serialize, Deserialize)]
            #[serde(rename_all = "snake_case", tag = "type")]
            pub enum MessageOutputContent {
                OutputText(OutputTextContent),
                Refusal(RefusalContent),
            }

            #[derive(Debug, Serialize, Deserialize)]
            #[serde(rename_all = "snake_case")]
            pub struct OutputTextContent {
                pub annotations: Vec<Annotation>,
                pub text: String,
            }

            #[derive(Debug, Serialize, Deserialize)]
            #[serde(rename_all = "snake_case", tag = "type")]
            pub enum Annotation {
                FileCitation(FileCitation),
                UrlCitation(URLCitation),
                ContainerFileCitation(ContainerFileCitation),
                FilePath(FilePath),
            }

            #[derive(Debug, Serialize, Deserialize)]
            #[serde(rename_all = "snake_case")]
            pub struct FileCitation {
                pub file_id: String,
                pub filename: String,
                pub index: i32,
            }

            #[derive(Debug, Serialize, Deserialize)]
            #[serde(rename_all = "snake_case")]
            pub struct URLCitation {
                pub end_index: i32,
                pub start_index: i32,
                pub title: String,
                pub url: String,
            }

            #[derive(Debug, Serialize, Deserialize)]
            #[serde(rename_all = "snake_case")]
            pub struct ContainerFileCitation {
                pub container_id: String,
                pub end_index: i32,
                pub file_id: String,
                pub filename: String,
                pub start_index: i32,
            }

            #[derive(Debug, Serialize, Deserialize)]
            #[serde(rename_all = "snake_case")]
            pub struct FilePath {
                pub file_id: String,
                pub index: i32,
            }

            #[derive(Debug, Serialize, Deserialize)]
            #[serde(rename_all = "snake_case")]
            pub struct RefusalContent {
                pub refusal: String,
            }

            #[derive(Debug, Serialize, Deserialize)]
            #[serde(rename_all = "snake_case")]
            pub struct FileSearchToolCallOutput {
                // TODO
            }

            #[derive(Debug, Serialize, Deserialize)]
            #[serde(rename_all = "snake_case")]
            pub struct FunctionToolCallOutput {
                // TODO
            }

            #[derive(Debug, Serialize, Deserialize)]
            #[serde(rename_all = "snake_case")]
            pub struct WebSearchToolCallOutput {
                // TODO
            }

            #[derive(Debug, Serialize, Deserialize)]
            #[serde(rename_all = "snake_case")]
            pub struct ComputerToolCallOutput {
                // TODO
            }

            #[derive(Debug, Serialize, Deserialize)]
            #[serde(rename_all = "snake_case")]
            pub struct ReasoningOutput {
                // TODO
            }

            #[derive(Debug, Serialize, Deserialize)]
            #[serde(rename_all = "snake_case")]
            pub struct CodeInterpreterToolCallOutput {
                // TODO
            }

            #[derive(Debug, Serialize, Deserialize)]
            #[serde(rename_all = "snake_case")]
            pub struct LocalShellCallOutput {
                // TODO
            }

            #[derive(Debug, Serialize, Deserialize)]
            #[serde(rename_all = "snake_case")]
            pub struct MCPToolCallOutput {
                // TODO
            }

            #[derive(Debug, Serialize, Deserialize)]
            #[serde(rename_all = "snake_case")]
            pub struct MCPToolListOutput {
                // TODO
            }

            #[derive(Debug, Serialize, Deserialize)]
            #[serde(rename_all = "snake_case")]
            pub struct MCPApprovalRequestOutput {
                // TODO
            }

            #[derive(Debug, Serialize, Deserialize)]
            #[serde(rename_all = "snake_case")]
            pub struct CustomToolCallOutput {
                // TODO
            }

            #[derive(Debug, Serialize, Deserialize)]
            #[serde(rename_all = "snake_case")]
            pub enum Status {
                Completed,
                Failed,
                InProgress,
                Cancelled,
                Queued,
                Incomplete,
            }

            #[derive(Debug, Serialize, Deserialize)]
            #[serde(rename_all = "snake_case")]
            pub struct InputTokenDetails {
                pub cached_tokens: i32,
            }

            #[derive(Debug, Serialize, Deserialize)]
            #[serde(rename_all = "snake_case")]
            pub struct OutputTokenDetails {
                pub reasoning_tokens: i32,
            }

            #[derive(Debug, Serialize, Deserialize)]
            #[serde(rename_all = "snake_case")]
            pub struct Usage {
                pub input_tokens: i32,
                pub input_tokens_details: InputTokenDetails,
                pub output_tokens: i32,
                pub output_tokens_details: OutputTokenDetails,
                pub total_tokens: i32,
            }
        }
    }
}
