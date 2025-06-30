use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::thread;

use anyhow::Context as AnyhowContext;
use pipewire::spa::param::audio::{AudioFormat as PwAudioFormat, AudioInfoRaw};
use pipewire::{context::Context, main_loop::MainLoop, spa, stream::StreamRef};
use spa::pod::serialize::PodSerializer;

use crate::logger::Logger;

use crate::speech::audio_format::{AudioFormat, SoundSpec};

#[derive(Clone)]
pub struct StopTrigger {
    has_triggered: Arc<AtomicBool>,
}

impl Default for StopTrigger {
    fn default() -> Self {
        Self::new()
    }
}

impl StopTrigger {
    #[must_use]
    pub fn new() -> Self {
        let has_triggered = Arc::new(AtomicBool::new(false));

        Self { has_triggered }
    }

    pub fn stop(self) {
        self.has_triggered.store(true, Ordering::Relaxed);
    }

    fn has_stopped(&self) -> bool {
        self.has_triggered.load(Ordering::Relaxed)
    }
}

struct StreamUserData {}

pub struct AudioRecorder {
    audio_data_sender: mpsc::Sender<Vec<u8>>,
    audio_data_receiver: Arc<Mutex<mpsc::Receiver<Vec<u8>>>>,
    logger: Arc<Mutex<dyn Logger>>,
}

impl AudioRecorder {
    pub fn new(logger: Arc<Mutex<dyn Logger>>) -> AudioRecorder {
        let (audio_data_sender, audio_data_receiver) = std::sync::mpsc::channel::<Vec<u8>>();
        let audio_data_receiver = Arc::new(Mutex::new(audio_data_receiver));

        Self {
            audio_data_sender,
            audio_data_receiver,
            logger,
        }
    }

    pub fn listen(
        &mut self,
        request_format: Option<SoundSpec>,
    ) -> (mpsc::Receiver<Vec<u8>>, StopTrigger, SoundSpec) {
        let negotiated_spec =
            start_pipewire_loop(&self.audio_data_sender, self.logger.clone(), request_format);

        let (tx, rx) = mpsc::channel();
        let trigger = StopTrigger::new();
        let main_receiver = Arc::clone(&self.audio_data_receiver);

        let trigger_for_thread = trigger.clone();
        thread::spawn(move || {
            let receiver = main_receiver.lock().unwrap();
            while !trigger_for_thread.has_stopped() {
                match receiver.try_recv() {
                    Ok(data) => {
                        if tx.send(data).is_err() {
                            break;
                        }
                    }
                    Err(mpsc::TryRecvError::Empty) => {
                        thread::sleep(std::time::Duration::from_millis(1));
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        break;
                    }
                }
            }

            // TODO stop the pipewire loop!

            drop(receiver); // TODO necessary?
        });

        (rx, trigger, negotiated_spec)
    }
}

fn start_pipewire_loop(
    audio_data_sender: &mpsc::Sender<Vec<u8>>,
    logger: Arc<Mutex<dyn Logger>>,
    request_format: Option<SoundSpec>,
) -> SoundSpec {
    let audio_data_sender = audio_data_sender.clone();

    let (sound_spec_sender, sound_spec_receiver) = mpsc::channel::<SoundSpec>();

    thread::spawn(move || {
        // TODO error handling; Maybe pass a channel to this function, that
        // we'll send to a single "OK" or "ERROR" just before `mainloop.run()`.
        // AudioRecorder.create then would wait for that "OK" or "ERROR"
        let mainloop = MainLoop::new(None)
            .context("Failed to initialize Pipewire main loop")
            .unwrap();
        let context = Context::new(&mainloop).unwrap();
        let core = context.connect(None).unwrap();

        /* Make one parameter with the supported formats. The SPA_PARAM_EnumFormat
         * id means that this is a format enumeration (of 1 value).
         * We leave the channels and rate empty to accept the native graph
         * rate and channels. */
        let mut audio_info = spa::param::audio::AudioInfoRaw::new();
        if let Some(format) = request_format {
            match format {
                SoundSpec::PCM {
                    format,
                    sample_rate_hz,
                    num_channels,
                } => {
                    let pw_format = match format {
                        AudioFormat::S16LE => PwAudioFormat::S16LE,
                        AudioFormat::S16BE => PwAudioFormat::S16BE,
                        AudioFormat::U16LE => PwAudioFormat::U16LE,
                        AudioFormat::U16BE => PwAudioFormat::U16BE,
                        AudioFormat::S24_32LE => PwAudioFormat::S24_32LE,
                        AudioFormat::S24_32BE => PwAudioFormat::S24_32BE,
                        AudioFormat::U24_32LE => PwAudioFormat::U24_32LE,
                        AudioFormat::U24_32BE => PwAudioFormat::U24_32BE,
                        AudioFormat::S32LE => PwAudioFormat::S32LE,
                        AudioFormat::S32BE => PwAudioFormat::S32BE,
                        AudioFormat::U32LE => PwAudioFormat::U32LE,
                        AudioFormat::U32BE => PwAudioFormat::U32BE,
                        AudioFormat::S24LE => PwAudioFormat::S24LE,
                        AudioFormat::S24BE => PwAudioFormat::S24BE,
                        AudioFormat::U24LE => PwAudioFormat::U24LE,
                        AudioFormat::U24BE => PwAudioFormat::U24BE,
                        AudioFormat::S20LE => PwAudioFormat::S20LE,
                        AudioFormat::S20BE => PwAudioFormat::S20BE,
                        AudioFormat::U20LE => PwAudioFormat::U20LE,
                        AudioFormat::U20BE => PwAudioFormat::U20BE,
                        AudioFormat::S18LE => PwAudioFormat::S18LE,
                        AudioFormat::S18BE => PwAudioFormat::S18BE,
                        AudioFormat::U18LE => PwAudioFormat::U18LE,
                        AudioFormat::U18BE => PwAudioFormat::U18BE,
                        AudioFormat::F32LE => PwAudioFormat::F32LE,
                        AudioFormat::F32BE => PwAudioFormat::F32BE,
                        AudioFormat::F64LE => PwAudioFormat::F64LE,
                        AudioFormat::F64BE => PwAudioFormat::F64BE,
                    };
                    audio_info.set_format(pw_format);
                    audio_info.set_rate(sample_rate_hz);
                    audio_info.set_channels(num_channels);
                }
            }
        }
        let obj = spa::pod::Object {
            type_: spa::utils::SpaTypes::ObjectParamFormat.as_raw(),
            id: spa::param::ParamType::EnumFormat.as_raw(),
            properties: audio_info.into(),
        };
        let values = PodSerializer::serialize(
            std::io::Cursor::new(Vec::new()),
            &spa::pod::Value::Object(obj),
        );
        let values: Vec<u8> = values.unwrap().0.into_inner();

        let mut params = [spa::pod::Pod::from_bytes(&values).unwrap()];

        let properties = pipewire::properties::properties! {
            *pipewire::keys::MEDIA_TYPE => "Audio",
            *pipewire::keys::MEDIA_CATEGORY => "Capture",
            *pipewire::keys::MEDIA_ROLE => "Music",
        };
        let stream = pipewire::stream::Stream::new(&core, "audio-capture", properties).unwrap();

        stream
            .connect(
                spa::utils::Direction::Input,
                None,
                pipewire::stream::StreamFlags::AUTOCONNECT
                    | pipewire::stream::StreamFlags::MAP_BUFFERS
                    | pipewire::stream::StreamFlags::RT_PROCESS,
                &mut params,
            )
            .unwrap();

        let listener = stream.add_local_listener_with_user_data(StreamUserData {});
        let listener = add_param_changed_callback(listener, sound_spec_sender, logger.clone());
        let listener = add_process_callback(listener, &audio_data_sender, logger);
        // listener must outlive the main loop
        let _listener = listener.register();

        // Note: If I need to quit the main loop, here's an example on how to
        // do that:
        // https://pipewire.pages.freedesktop.org/pipewire-rs/pipewire/channel/index.html
        mainloop.run();
    });

    sound_spec_receiver.recv().unwrap()
}

fn add_param_changed_callback(
    listener: pipewire::stream::ListenerLocalBuilder<'_, StreamUserData>,
    sound_spec_sender: mpsc::Sender<SoundSpec>,
    logger: Arc<Mutex<dyn Logger>>,
) -> pipewire::stream::ListenerLocalBuilder<'_, StreamUserData> {
    listener.param_changed(move |_stream, _user_data, id, param| {
        // TODO is this called again if we switch our Audio device
        // mid-recording? If yes, we may want to notify outside code

        // param == None means to clear the format
        let Some(param) = param else {
            return;
        };
        if id != spa::param::ParamType::Format.as_raw() {
            return;
        }

        let Ok((media_type, media_subtype)) = spa::param::format_utils::parse_format(param) else {
            return;
        };

        // only accept raw audio
        if media_type != spa::param::format::MediaType::Audio
            || media_subtype != spa::param::format::MediaSubtype::Raw
        {
            return;
        }

        let mut audio_info = AudioInfoRaw::default();
        audio_info
            .parse(param)
            .expect("Failed to parse param changed to AudioInfoRaw");

        let logger = &*logger.lock().unwrap();
        logger.debug(&format!("audio format: {:?}", audio_info.format()));

        let format = match audio_info.format() {
            PwAudioFormat::S16LE => AudioFormat::S16LE,
            PwAudioFormat::S16BE => AudioFormat::S16BE,
            PwAudioFormat::U16LE => AudioFormat::U16LE,
            PwAudioFormat::U16BE => AudioFormat::U16BE,
            PwAudioFormat::S24_32LE => AudioFormat::S24_32LE,
            PwAudioFormat::S24_32BE => AudioFormat::S24_32BE,
            PwAudioFormat::U24_32LE => AudioFormat::U24_32LE,
            PwAudioFormat::U24_32BE => AudioFormat::U24_32BE,
            PwAudioFormat::S32LE => AudioFormat::S32LE,
            PwAudioFormat::S32BE => AudioFormat::S32BE,
            PwAudioFormat::U32LE => AudioFormat::U32LE,
            PwAudioFormat::U32BE => AudioFormat::U32BE,
            PwAudioFormat::S24LE => AudioFormat::S24LE,
            PwAudioFormat::S24BE => AudioFormat::S24BE,
            PwAudioFormat::U24LE => AudioFormat::U24LE,
            PwAudioFormat::U24BE => AudioFormat::U24BE,
            PwAudioFormat::S20LE => AudioFormat::S20LE,
            PwAudioFormat::S20BE => AudioFormat::S20BE,
            PwAudioFormat::U20LE => AudioFormat::U20LE,
            PwAudioFormat::U20BE => AudioFormat::U20BE,
            PwAudioFormat::S18LE => AudioFormat::S18LE,
            PwAudioFormat::S18BE => AudioFormat::S18BE,
            PwAudioFormat::U18LE => AudioFormat::U18LE,
            PwAudioFormat::U18BE => AudioFormat::U18BE,
            PwAudioFormat::F32LE => AudioFormat::F32LE,
            PwAudioFormat::F32BE => AudioFormat::F32BE,
            PwAudioFormat::F64LE => AudioFormat::F64LE,
            PwAudioFormat::F64BE => AudioFormat::F64BE,
            _ => panic!("Unsupported format for now"),
        };
        let sample_rate_hz = audio_info.rate();
        let num_channels = audio_info.channels();
        sound_spec_sender
            .send(SoundSpec::PCM {
                format,
                sample_rate_hz,
                num_channels,
            })
            .unwrap();
    })
}

fn add_process_callback<'a>(
    listener: pipewire::stream::ListenerLocalBuilder<'a, StreamUserData>,
    audio_data_sender: &mpsc::Sender<Vec<u8>>,
    logger: Arc<Mutex<dyn Logger>>,
) -> pipewire::stream::ListenerLocalBuilder<'a, StreamUserData> {
    let audio_data_sender = audio_data_sender.clone();

    listener.process(move |stream: &StreamRef, _user_data: &mut StreamUserData| {
        let buf = stream.dequeue_buffer();

        if buf.is_none() {
            // TODO check what the None value means so that I can create a
            // better error message
            logger.lock().unwrap().error("No buffer");
            return;
        }
        let mut buf = buf.unwrap();

        for data in buf.datas_mut() {
            let chunk = data.chunk();
            let data_from = chunk.offset() as usize;
            let data_to = data_from + chunk.size() as usize;

            let data: &[u8] = data.data().unwrap_or(&mut []);
            let data: &[u8] = &data[data_from..data_to];

            audio_data_sender
                .send(data.to_vec())
                .unwrap_or_else(|_| logger.lock().unwrap().error("Failed to send audio data"));
        }
    })
}
