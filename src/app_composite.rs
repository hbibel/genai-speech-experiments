use crate::{
    config::Config,
    logger::{ConsoleLogger, Logger},
    speech::{audio::AudioRecorder, input::SpeechListener},
};
use std::sync::{Arc, Mutex};

pub struct AppComposite {
    pub speech_listener: SpeechListener,
    pub logger: Arc<Mutex<dyn Logger>>,
}

impl AppComposite {
    pub fn new(config: &Config) -> anyhow::Result<Self> {
        let logger = ConsoleLogger::new();
        let logger = Arc::new(Mutex::new(logger));

        let audio_recorder = match config.recording_file.clone() {
            Some(f) => AudioRecorder::new(logger.clone(), Some(&f)),
            None => AudioRecorder::new(logger.clone(), None),
        }?;

        Ok(Self {
            speech_listener: SpeechListener::new(config, audio_recorder),
            logger,
        })
    }
}
