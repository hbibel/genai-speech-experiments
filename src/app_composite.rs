use crate::{
    config::Config,
    logger::Logger,
    speech::{audio::AudioRecorder, input::SpeechListener},
};

pub struct AppComposite {
    pub speech_listener: SpeechListener,
    pub logger: Logger, // pub intent_classifier: Box<dyn IntentClassifier>,
}

impl AppComposite {
    pub fn new(config: &Config) -> anyhow::Result<Self> {
        let logger = Logger::new();

        let audio_recorder = match config.recording_file.clone() {
            Some(f) => AudioRecorder::new(logger, Some(&f)),
            None => AudioRecorder::new(logger, None),
        }?;

        Ok(Self {
            speech_listener: SpeechListener::new(config, audio_recorder),
            logger,
        })
    }
}
