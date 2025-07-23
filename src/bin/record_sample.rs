use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use jarvis_code::logger::ConsoleLogger;
use jarvis_code::speech::audio::AudioRecorder;
use jarvis_code::speech::audio::format::PCMFormat;
use jarvis_code::speech::audio::format::SoundSpec;

// You can use this binary to record a sample using the AudioRecorder.
// Edit the std::thread::sleep(Duration::from_secs(5)); statement to change
// the recording length, or even better, add a CLI arg.

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let logger = Arc::new(Mutex::new(ConsoleLogger::new()));

    let mut rec = AudioRecorder::new(logger.clone(), None).unwrap();

    let sound_spec = SoundSpec::PCM {
        format: PCMFormat::S16LE,
        sample_rate_hz: 24000,
        num_channels: 1,
    };
    let (receiver, stop, _) = rec.listen(Some(sound_spec))?;
    std::thread::sleep(Duration::from_secs(5));
    stop.stop();

    let mut bytes: Vec<u8> = Vec::new();
    let mut total_bytes = 0;
    for chunk in receiver {
        total_bytes += chunk.len();
        bytes.extend(chunk);
    }

    #[allow(clippy::cast_precision_loss)]
    let total_mb = total_bytes as f32 / 1_000_000.0;

    println!("Total bytes received: {total_bytes} bytes ({total_mb:.2} MB)");

    // can be played back using
    //     ffplay  -autoexit -f s16le -ar 24000 -ac 1 output.pcm
    let output_path = "output.pcm";

    let mut file = File::create(output_path)?;
    file.write_all(&bytes)?;

    Ok(())
}
