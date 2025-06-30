use std::fmt::Display;

#[derive(Clone, PartialEq, Eq)]
pub enum AudioFormat {
    S16LE,
    S16BE,
    U16LE,
    U16BE,
    S24_32LE,
    S24_32BE,
    U24_32LE,
    U24_32BE,
    S32LE,
    S32BE,
    U32LE,
    U32BE,
    S24LE,
    S24BE,
    U24LE,
    U24BE,
    S20LE,
    S20BE,
    U20LE,
    U20BE,
    S18LE,
    S18BE,
    U18LE,
    U18BE,
    F32LE,
    F32BE,
    F64LE,
    F64BE,
}

impl Display for AudioFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fmt_str = match self {
            AudioFormat::S16LE => "s16le",
            AudioFormat::S16BE => "s16be",
            AudioFormat::U16LE => "u16le",
            AudioFormat::U16BE => "u16be",
            AudioFormat::S24_32LE => "s24_32le",
            AudioFormat::S24_32BE => "s24_32be",
            AudioFormat::U24_32LE => "u24_32le",
            AudioFormat::U24_32BE => "u24_32be",
            AudioFormat::S32LE => "s32le",
            AudioFormat::S32BE => "s32be",
            AudioFormat::U32LE => "u32le",
            AudioFormat::U32BE => "u32be",
            AudioFormat::S24LE => "s24le",
            AudioFormat::S24BE => "s24be",
            AudioFormat::U24LE => "u24le",
            AudioFormat::U24BE => "u24be",
            AudioFormat::S20LE => "s20le",
            AudioFormat::S20BE => "s20be",
            AudioFormat::U20LE => "u20le",
            AudioFormat::U20BE => "u20be",
            AudioFormat::S18LE => "s18le",
            AudioFormat::S18BE => "s18be",
            AudioFormat::U18LE => "u18le",
            AudioFormat::U18BE => "u18be",
            AudioFormat::F32LE => "f32le",
            AudioFormat::F32BE => "f32be",
            AudioFormat::F64LE => "f64le",
            AudioFormat::F64BE => "f64be",
        };
        f.write_str(fmt_str)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum SoundSpec {
    PCM {
        format: AudioFormat,
        sample_rate_hz: u32,
        num_channels: u32,
    },
}

impl Display for SoundSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Audio format [")?;
        match self {
            Self::PCM {
                format,
                sample_rate_hz,
                num_channels,
            } => {
                f.write_str(&format!("{format}, "))?;
                f.write_str(&format!("{sample_rate_hz} Hz, "))?;
                f.write_str(&format!("{num_channels} channels"))?;
            }
        }
        f.write_str("]")
    }
}
