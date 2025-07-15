//! Using the [Open AI realtime transcription API](https://platform.openai.com/docs/guides/realtime?use-case=transcription)

use std::str::FromStr;
use std::sync::mpsc::Receiver;

use anyhow::{Context, Ok, bail};
use base64::prelude::*;
use futures_util::{SinkExt, Stream, StreamExt, TryStreamExt, future};
use serde::{Deserialize, Serialize};
use serde_json;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{Receiver as TokioReceiver, channel};
use tokio_tungstenite::tungstenite;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{http, protocol::Message},
};

use crate::speech::audio::AudioRecorder;
use crate::{
    config::Config,
    speech::audio_format::{PCMFormat, SoundSpec},
};

use super::Transcription;

pub struct SpeechListener {
    api_key: String,
    audio_recorder: AudioRecorder,
}

impl SpeechListener {
    #[must_use]
    pub fn new(config: &Config, audio_recorder: AudioRecorder) -> Self {
        Self {
            api_key: config.openai_key.clone(),
            audio_recorder,
        }
    }

    pub async fn listen_to_input(&mut self) -> anyhow::Result<Transcription> {
        let desired_format = SoundSpec::PCM {
            format: PCMFormat::S16LE,
            sample_rate_hz: 24000,
            num_channels: 1,
        };
        let (sound_receiver, stop, actual_format) =
            self.audio_recorder.listen(Some(desired_format.clone()));

        if desired_format != actual_format {
            anyhow::bail!(
                "Could not record audio in the required format {desired_format}. Your device instead records in format {actual_format}"
            )
        }

        let ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>> =
            create_ws(&self.api_key).await?;
        let (mut ws_write, ws_read) = ws_stream.split();

        let transcription_events = to_event_stream(ws_read);
        let transcription_fut = tokio::spawn(async move {
            let result = transcription_events
                .try_fold(Transcription::Empty, async |acc, event| {
                    match (acc, event) {
                        (_, TranscriptionMessage::Error(err)) => Err(anyhow::Error::msg(format!(
                            "Transcription failed with an error from the API: {}",
                            err.error.message
                        ))),
                        (_, TranscriptionMessage::TranscriptionCompleted(transcription)) => {
                            Ok(Transcription::Some {
                                text: transcription.transcript,
                            })
                        }
                        (acc, _) => Ok(acc),
                    }
                })
                .await;
            stop.stop();
            result
        });

        let mut audio_receiver = to_async_receiver(sound_receiver);
        let consume_audio = tokio::spawn(async move {
            let mut next_msg = audio_receiver.recv().await;
            while let Some(chunk) = next_msg {
                let json = "{\"type\": \"input_audio_buffer.append\",\"audio\": \"".to_owned();
                let json = json + &BASE64_STANDARD.encode(chunk);
                let json = json + "\"}";
                match ws_write.feed(Message::Text(json.into())).await {
                    Result::Ok(()) => (),
                    Err(err) => eprintln!("Could not send audio data: {err}"),
                }

                next_msg = audio_receiver.recv().await;
            }
        });

        let (transcription, sink_result) = future::join(transcription_fut, consume_audio).await;
        transcription
            .context("Failed to run transcription")
            .and_then(|res| res)
            .and_then(|res| {
                sink_result
                    .context("Failed to send audio data")
                    .map(|()| res)
            })
    }
}

fn to_event_stream<S: StreamExt<Item = Result<tungstenite::Message, tungstenite::Error>> + Send>(
    ws_stream: S,
) -> impl Stream<Item = anyhow::Result<TranscriptionMessage>> + Send {
    ws_stream.filter_map(async move |try_msg| match try_msg {
        Result::Err(_err) => Some(Result::Err(anyhow::Error::msg(
            "Failed to consume websocket stream",
        ))),
        Result::Ok(msg) => {
            if let Message::Text(msg) = msg {
                let msg = msg.as_str();
                println!("Received message {msg}");
                Some(
                    serde_json::from_str::<TranscriptionMessage>(msg)
                        .context("Failed to serialize message {msg}"),
                )
            } else {
                None
            }
        }
    })
}

fn to_async_receiver<T: Send + 'static>(receiver: Receiver<T>) -> TokioReceiver<T> {
    // channel size chosen arbitrarily; note that an unbounded channel here
    // can lead to an issue we block all threads on the Tokio runtime.
    let (tx, rx) = channel(1024);
    tokio::spawn(async move {
        for x in receiver {
            match tx.send(x).await {
                Result::Ok(()) => (),
                Result::Err(err) => println!("Failed to send: {err}"),
            }
        }
    });
    rx
}

async fn create_ws(api_key: &str) -> anyhow::Result<WebSocketStream<MaybeTlsStream<TcpStream>>> {
    let url = http::Uri::from_str("wss://api.openai.com/v1/realtime?intent=transcription")?;
    // into_client_request for Uri will set headers required for websockets
    let mut req = url.into_client_request()?;
    let headers = req.headers_mut();
    let header_val = "Bearer ".to_owned() + api_key;
    let header_val = HeaderValue::from_str(&header_val)
        .context("Could not create header from OpenAI API key")?;
    headers
        .try_insert("Authorization", header_val)
        .context("Failed to modify transcription websocket request headers")?;
    let header_val = "realtime=v1";
    let header_val = HeaderValue::from_str("realtime=v1")
        .context(format!("Could not create header from '{header_val}'"))?;
    headers
        .try_insert("OpenAI-Beta", header_val)
        .context("Failed to modify transcription websocket request headers")?;

    let (ws_stream, _res) = connect_async(req).await?;
    Ok(ws_stream)
}

async fn expect_event<F>(
    read: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    name: &str,
    event_matcher: F,
) -> anyhow::Result<()>
where
    F: FnOnce(TranscriptionMessage) -> bool,
{
    let msg = read
        .next()
        .await
        .ok_or(anyhow::format_err!(
            "Stream closed while waiting for {name} event"
        ))
        .and_then(|it| it.context(format!("Failed to receive {name} event")))?;
    let json_msg = if let Message::Text(msg) = msg {
        msg.as_str().to_owned()
    } else {
        bail!(
            "Message '{msg}' from the OpenAI transcription API does not appear to contain valid text data"
        )
    };
    let msg = serde_json::from_str::<TranscriptionMessage>(&json_msg)
        .context(format!("Failed to parse transcription message {json_msg}"))?;

    if let TranscriptionMessage::Error(err) = msg {
        bail!(
            "The OpenAI transition API responded with an error: {}: {}",
            err.error.type_,
            err.error.message,
        )
    }
    if !event_matcher(msg) {
        bail!("Expected {name} message from transcription API, but got {json_msg}");
    }

    Ok(())
}

pub enum TranscriptionSessionUpdateType {
    Update,
}

impl serde::Serialize for TranscriptionSessionUpdateType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Update => serializer.serialize_str("transcription_session.update"),
        }
    }
}

#[derive(serde::Serialize)]
pub struct TranscriptionSessionUpdate {
    session: TranscriptionSessionUpdateSession,
    #[serde(rename = "type")]
    type_: TranscriptionSessionUpdateType,
}

#[allow(dead_code)]
pub enum TranscriptionAudioFormat {
    PCM16,
    G711ulaw,
    G711alaw,
}

impl serde::Serialize for TranscriptionAudioFormat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let str_val = match self {
            Self::PCM16 => "pcm16",
            Self::G711ulaw => "g711_ulaw",
            Self::G711alaw => "g711_alaw",
        };
        serializer.serialize_str(str_val)
    }
}

#[allow(dead_code)]
pub enum NoiseReductionType {
    NearField,
    FarField,
}

impl serde::Serialize for NoiseReductionType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let str_val = match self {
            Self::NearField => "near_field",
            Self::FarField => "far_field",
        };
        serializer.serialize_str(str_val)
    }
}

#[derive(serde::Serialize)]
pub struct TranscriptionNoiseReduction {
    #[serde(rename = "type")]
    type_: NoiseReductionType,
}

#[derive(serde::Serialize)]
pub struct InputAudioTranscription {
    // TODO: Attributes could be modeled with enums
    language: Option<String>,
    model: Option<String>,
    prompt: Option<String>,
}

#[allow(dead_code)]
pub enum TurnDetectionType {
    ServerVad,
    SemanticVad,
}

impl serde::Serialize for TurnDetectionType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let str_val = match self {
            Self::ServerVad => "server_vad",
            Self::SemanticVad => "semantic_vad",
        };
        serializer.serialize_str(str_val)
    }
}

#[derive(serde::Serialize)]
pub struct TranscriptionTurnDetection {
    #[serde(rename = "type")]
    type_: TurnDetectionType,
}

#[derive(serde::Serialize)]
pub struct TranscriptionSessionUpdateSession {
    // client_secret
    // include
    // modalities

    // input audio must be 16-bit PCM at a 24kHz sample rate, single channel, little-endian
    input_audio_format: TranscriptionAudioFormat,

    input_audio_noise_reduction: TranscriptionNoiseReduction,

    input_audio_transcription: InputAudioTranscription,

    turn_detection: TranscriptionTurnDetection,
}

/* Transcription messages */

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum TranscriptionMessage {
    #[serde(rename = "error")]
    Error(ErrorEvent),

    #[serde(rename = "transcription_session.created")]
    SessionCreated(SessionEvent),

    #[serde(rename = "transcription_session.updated")]
    SessionUpdated(SessionEvent),

    #[serde(rename = "input_audio_buffer.speech_started")]
    SpeechStarted(SpeechBoundaryEvent),

    #[serde(rename = "input_audio_buffer.speech_stopped")]
    SpeechStopped(SpeechBoundaryEvent),

    #[serde(rename = "input_audio_buffer.committed")]
    SpeechCommitted(SpeechCommittedEvent),

    #[serde(rename = "conversation.item.created")]
    ConversationItemCreated(ConversationItemCreatedEvent),

    #[serde(rename = "conversation.item.input_audio_transcription.delta")]
    TranscriptionDelta(TranscriptionDeltaEvent),

    #[serde(rename = "conversation.item.input_audio_transcription.completed")]
    TranscriptionCompleted(TranscriptionCompletedEvent),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionEvent {
    pub event_id: String,
    pub session: SessionDetail,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionDetail {
    id: String,
    input_audio_format: String,
    input_audio_noise_reduction: Option<InputAudioNoiseReduction>,
    input_audio_transcription: Option<InputAudioTranscriptionData>,
    instructions: Option<String>,
    max_response_output_tokens: Option<u32>, // TODO or "inf"
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum InputAudioNoiseReduction {
    #[serde(rename = "near_field")]
    NearField,
    #[serde(rename = "far_field")]
    FarField,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InputAudioTranscriptionData {
    // TODO language and model could also be enums
    language: String,
    model: String,
    prompt: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorEvent {
    pub event_id: String,
    pub error: ErrorEventDetail,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorEventDetail {
    #[serde(rename = "type")]
    pub type_: String,
    pub event_id: Option<String>,
    pub code: Option<String>,
    pub message: String,
    pub param: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpeechBoundaryEvent {
    pub event_id: Option<String>,
    pub item_id: Option<String>,
    pub audio_start_ms: Option<u32>,
}

#[allow(clippy::struct_field_names)]
#[derive(Debug, Serialize, Deserialize)]
pub struct SpeechCommittedEvent {
    pub event_id: Option<String>,
    pub item_id: Option<String>,
    pub previous_item_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConversationItemCreatedEvent {
    pub event_id: Option<String>,
    pub previous_item_id: Option<String>,
    pub item: ConversationItem,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ConversationItem {
    #[serde(rename = "message")]
    Message {
        id: String,
        role: MessageRole,
        content: Vec<MessageContent>,
        object: Option<String>,
        status: Option<String>,
    },

    #[serde(rename = "function_call")]
    FunctionCall {
        id: String,
        name: String,
        arguments: String,
        call_id: String,
        object: Option<String>,
        status: Option<String>,
    },

    #[serde(rename = "function_call_output")]
    FunctionCallOutput {
        id: String,
        call_id: String,
        object: Option<String>,
        status: Option<String>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MessageContent {
    #[serde(rename = "input_text")]
    InputText { text: String },
    #[serde(rename = "input_audio")]
    InputAudio {
        audio: Option<String>,
        transcript: Option<String>,
    },
    #[serde(rename = "item_reference")]
    ItemReference { id: String },
    #[serde(rename = "text")]
    Text { text: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MessageRole {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
    #[serde(rename = "system")]
    System,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TranscriptionDeltaEvent {
    pub event_id: Option<String>,
    pub item_id: Option<String>,
    pub content_index: Option<i32>,
    pub delta: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TranscriptionCompletedEvent {
    pub event_id: Option<String>,
    pub item_id: Option<String>,
    pub content_index: Option<i32>,
    pub transcript: String,
    // TODO maybe add usage
}
