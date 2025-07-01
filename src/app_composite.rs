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
    #[must_use]
    pub fn new(config: &Config) -> Self {
        let logger = ConsoleLogger::new();
        let logger = Arc::new(Mutex::new(logger));

        let audio_recorder = AudioRecorder::new(logger.clone());

        Self {
            speech_listener: SpeechListener::new(config, audio_recorder),
            logger,
        }
    }
}
