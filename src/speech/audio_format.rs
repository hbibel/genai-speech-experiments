use std::fmt::Display;

#[derive(Clone, PartialEq, Eq)]
pub enum SoundSpec {
    PCM {
        format: PCMFormat,
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

#[derive(Clone, PartialEq, Eq)]
pub enum PCMFormat {
    S16LE,
}

impl Display for PCMFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fmt_str = match self {
            PCMFormat::S16LE => "s16le",
        };
        f.write_str(fmt_str)
    }
}
