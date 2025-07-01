//! Using the OpenAI realtime transcription API here:
//! https://platform.openai.com/docs/guides/realtime?use-case=transcription

use std::str::FromStr;
use std::thread;

use anyhow::{Context, Ok, bail};
use base64::prelude::*;
use futures_util::{SinkExt, StreamExt, TryFutureExt};
use serde::{Deserialize, Serialize};
use serde_json;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{http, protocol::Message},
};

use crate::speech::audio::AudioRecorder;
use crate::{
    config::Config,
    speech::audio_format::{AudioFormat, SoundSpec},
};

use super::RecognizedSpeech;

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

    pub async fn listen_to_input(&mut self) -> anyhow::Result<RecognizedSpeech> {
        let desired_format = SoundSpec::PCM {
            format: AudioFormat::S16LE,
            sample_rate_hz: 24000,
            num_channels: 1,
        };
        let (sound_receiver, _stop, actual_format) =
            self.audio_recorder.listen(Some(desired_format.clone()));

        if desired_format != actual_format {
            anyhow::bail!(
                "Could not record audio in the required format {desired_format}. Your device instead records in format {actual_format}"
            )
        }

        let url = http::Uri::from_str("wss://api.openai.com/v1/realtime?intent=transcription")?;
        // into_client_request for Uri will set headers required for websockets
        let mut req = url.into_client_request()?;
        let headers = req.headers_mut();
        headers.try_insert(
            "Authorization",
            HeaderValue::from_str(&("Bearer ".to_owned() + &self.api_key))?,
        )?;
        headers.try_insert("OpenAI-Beta", HeaderValue::from_str("realtime=v1")?)?;

        let (ws_stream, _res) = connect_async(req)
            .await
            .expect("Failed to connect to OpenAI realtime API");

        Self::run_speech_session(ws_stream, sound_receiver).await?;

        // pin_mut!(stdin_to_ws, ws_to_stdout);
        // future::select(stdin_to_ws, ws_to_stdout).await;

        Ok(RecognizedSpeech {
            text: "todo".to_string(),
        })
    }

    async fn run_speech_session(
        ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
        sound_stream: std::sync::mpsc::Receiver<Vec<u8>>,
    ) -> anyhow::Result<()> {
        let (mut write, mut read) = ws_stream.split();

        let session_configuration = TranscriptionSessionUpdate {
            type_: TranscriptionSessionUpdateType::Update,
            session: TranscriptionSessionUpdateSession {
                input_audio_format: TranscriptionAudioFormat::PCM16,
                input_audio_noise_reduction: TranscriptionNoiseReduction {
                    type_: NoiseReductionType::NearField,
                },
                input_audio_transcription: InputAudioTranscription {
                    language: Some("en".into()),
                    model: Some("gpt-4o-mini-transcribe".into()),
                    prompt: Some("expect words related to technology".into()),
                },
                turn_detection: TranscriptionTurnDetection {
                    // Current regression in OpenAI: Semantic VAD never sends
                    // a stop event. See
                    // https://community.openai.com/t/semantic-vad-might-not-be-working-with-transcription-mode/1151522/3
                    // type_: TurnDetectionType::SemanticVad,
                    type_: TurnDetectionType::ServerVad,
                },
            },
        };
        let msg = serde_json::to_string(&session_configuration)?;
        let msg = Message::Text(msg.into());

        write
            .feed(msg.clone())
            .await
            .context(format!("Failed to send message {msg}"))?;

        let msg = read
            .next()
            .await
            .ok_or(anyhow::format_err!(
                "Stream closed while waiting for transcription_session.updated event"
            ))
            .and_then(|it| it.context("Failed to receive transcription_session.updated event"))?;
        let json_msg = if let Message::Text(msg) = msg {
            msg.as_str().to_owned()
        } else {
            bail!("Message {msg} from transcription API does not appear to contain valid text data")
        };
        let msg = serde_json::from_str::<TranscriptionMessage>(&json_msg)
            .context(format!("Failed to parse transcription message {json_msg}"))?;
        match msg {
            TranscriptionMessage::SessionCreated(_) => println!("Session created"),
            _ => bail!(
                "Expected transcription_session.updated message from transcription API, but got {json_msg}"
            ),
        }

        // TODO Can we wait for sound_stream and read concurrently using async
        // code? Then we could also send Pongs to the ping events ...
        thread::spawn(move || {
            for chunk in &sound_stream {
                let json = "{\"type\": \"input_audio_buffer.append\",\"audio\": \"".to_owned();
                let json = json + &BASE64_STANDARD.encode(chunk);
                let json = json + "\"}";
                let rt = tokio::runtime::Builder::new_current_thread()
                    .build()
                    .expect("Building runtime failed");
                rt.block_on(
                    write
                        .feed(Message::Text(json.into()))
                        .map_err(|err| eprintln!("Could not send audio data: {err}")),
                )
                .unwrap();
            }
        });

        read.for_each(|message| async {
            match message.unwrap() {
                Message::Ping(_) => println!("Ping received"),
                Message::Text(msg) => {
                    let msg = msg.as_str();
                    let msg: TranscriptionMessage = serde_json::from_str(msg).unwrap();
                    println!("msg received: {msg:#?}");
                }
                _ => println!("Other message received"),
            }
            // todo
        })
        .await;

        Ok(())
    }
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
