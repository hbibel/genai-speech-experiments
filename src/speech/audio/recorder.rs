mod file;
mod pipewire;

use std::{
    path::Path,
    sync::{Arc, Mutex, mpsc::Receiver},
};

use file::FileAudioRecorder;
use pipewire::PipewireAudioRecorder;

use crate::logger::Logger;

use super::{StopTrigger, format::SoundSpec};

type ListenResult = anyhow::Result<(Receiver<Vec<u8>>, StopTrigger, Option<SoundSpec>)>;

pub struct AudioRecorder(AudioRecorderImpl);

impl AudioRecorder {
    pub fn new(logger: Arc<Mutex<dyn Logger>>, from_file: Option<&Path>) -> anyhow::Result<Self> {
        match from_file {
            Some(path) => Ok(Self(AudioRecorderImpl::SampleFile(FileAudioRecorder(
                path.to_path_buf(),
            )))),
            None => Ok(Self(AudioRecorderImpl::Pipewire(
                PipewireAudioRecorder::new(logger),
            ))),
        }
    }

    pub fn listen(&mut self, request_format: Option<SoundSpec>) -> ListenResult {
        self.0.listen(request_format)
    }
}

enum AudioRecorderImpl {
    Pipewire(PipewireAudioRecorder),
    SampleFile(FileAudioRecorder),
}

impl AudioRecorderImpl {
    fn listen(&mut self, request_format: Option<SoundSpec>) -> ListenResult {
        match self {
            Self::Pipewire(rec) => rec.listen(request_format),
            Self::SampleFile(rec) => rec.listen(request_format),
        }
    }
}
