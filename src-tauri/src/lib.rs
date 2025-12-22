use anyhow::Result;
use crossbeam_channel::Receiver;
use futures_util::StreamExt;

mod audio;
use audio::{mic::Mic, speaker::Speaker};

enum AudioSource {
    Mic(Vec<f32>),
    System(Vec<f32>),
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() -> Result<()> {
    let (tx, rx) = crossbeam_channel::unbounded();

    // Initialize microphone
    let mic = Mic::new().expect("Failed to initialize microphone");
    let mic_sample_rate = mic.sample_rate();

    // Initialize system audio (speaker)
    let speaker = Speaker::new().expect("Failed to initialize speaker");
    let speaker_sample_rate = speaker.sample_rate();

    println!(
        "Recording mic ({}Hz) + system ({}Hz) → mixed.wav",
        mic_sample_rate, speaker_sample_rate
    );

    // Use the higher sample rate for output
    let output_sample_rate = mic_sample_rate.max(speaker_sample_rate);

    // Run mixer in blocking task since it blocks indefinitely
    tokio::task::spawn_blocking(move || {
        mixer(rx, output_sample_rate);
    });

    // Run both streams concurrently
    let tx_mic = tx.clone();
    let tx_sys = tx.clone();

    tokio::join!(
        spawn_mic_stream(tx_mic, mic),
        spawn_speaker_stream(tx_sys, speaker)
    );

    Ok(())
}

async fn spawn_mic_stream(tx: crossbeam_channel::Sender<AudioSource>, mic: Mic) {
    let mut stream = match mic.stream() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to create mic stream: {}", e);
            return;
        }
    };

    while let Some(chunk) = stream.next().await {
        if tx.send(AudioSource::Mic(chunk)).is_err() {
            eprintln!("Mic receiver dropped");
            break;
        }
    }
}

async fn spawn_speaker_stream(
    tx: crossbeam_channel::Sender<AudioSource>,
    speaker: audio::speaker::Speaker,
) {
    let mut stream = speaker.stream();

    while let Some(chunk) = stream.next().await {
        if tx.send(AudioSource::System(chunk)).is_err() {
            eprintln!("Speaker receiver dropped");
            break;
        }
    }
}

fn mixer(rx: Receiver<AudioSource>, sample_rate: u32) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create("mixed.wav", spec).unwrap();

    let mut mic_buf = Vec::<f32>::new();
    let mut sys_buf = Vec::<f32>::new();

    let mic_gain = 1.0;
    let sys_gain = 1.0;

    loop {
        match rx.recv().unwrap() {
            AudioSource::Mic(data) => mic_buf.extend(data),
            AudioSource::System(data) => sys_buf.extend(data),
        }

        let len = mic_buf.len().min(sys_buf.len());
        if len == 0 {
            continue;
        }

        for i in 0..len {
            let mixed = mic_buf[i] * mic_gain + sys_buf[i] * sys_gain;

            let s = (mixed.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            writer.write_sample(s).unwrap();
        }

        mic_buf.drain(..len);
        sys_buf.drain(..len);
    }
}
