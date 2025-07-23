use std::io::{BufReader, Read};
use std::sync::mpsc::sync_channel;
use std::thread;
use std::{fs::File, path::PathBuf};

use crate::speech::audio::StopTrigger;
use crate::speech::audio::format::SoundSpec;

use super::ListenResult;

/// An audio "recorder" that simply plays back audio from a file. Useful for
/// testing purposes.
pub struct FileAudioRecorder(pub PathBuf);

impl FileAudioRecorder {
    pub fn listen(&mut self, _request_format: Option<SoundSpec>) -> ListenResult {
        let f = File::open(self.0.clone())?;
        let (sender, receiver) = sync_channel(0);

        thread::spawn(move || {
            let mut buf: [u8; 4096] = [0; 4096];
            let mut reader = BufReader::new(f);

            let mut read_count = reader.read(&mut buf);
            while let Ok(c) = read_count {
                if c == 0 {
                    break;
                }
                if sender.send(buf[..c].into()).is_err() {
                    break;
                }
                read_count = reader.read(&mut buf);
            }
        });

        let stop_trigger = StopTrigger::new();

        Ok((receiver, stop_trigger, None))
    }
}
