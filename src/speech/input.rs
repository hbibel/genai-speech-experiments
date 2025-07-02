//! A [`SpeechListener`] is able to listen to spoken words and transcribe them
//! to written text.
//! This module contains a [`SpeechListener`] struct which abstracts over the
//! different possible implementations.

mod openai;

use crate::config::Config;

use super::audio::AudioRecorder;

use openai::SpeechListener as OpenAISpeechListener;

#[derive(Clone)]
pub struct RecognizedSpeech {
    pub text: String,
}

pub struct SpeechListener(SpeechListenerImpl);

impl SpeechListener {
    #[must_use]
    pub fn new(config: &Config, audio_recorder: AudioRecorder) -> Self {
        // If more speech listener backends are to be implemented, use the
        // config to decide which one to use at runtime.
        Self(SpeechListenerImpl::OpenAI(OpenAISpeechListener::new(
            config,
            audio_recorder,
        )))
    }

    pub async fn listen_to_input(&mut self) -> anyhow::Result<RecognizedSpeech> {
        self.0.listen_to_input().await
    }
}

enum SpeechListenerImpl {
    OpenAI(OpenAISpeechListener),
}

impl SpeechListenerImpl {
    async fn listen_to_input(&mut self) -> anyhow::Result<RecognizedSpeech> {
        match self {
            SpeechListenerImpl::OpenAI(l) => l.listen_to_input().await,
        }
    }
}
