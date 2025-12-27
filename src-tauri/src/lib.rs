use anyhow::Result;
use futures_util::StreamExt;
use std::sync::Arc;

mod audio;
mod bridges;
mod types;

use audio::{
    mic::Mic, 
    speaker::Speaker,
    processing::mixer,
    transcription::transcription_task,
};
use bridges::whisperkit::WhisperKit;
use types::AudioSource;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() -> Result<()> {
    let (tx, rx) = crossbeam_channel::unbounded();

    // Initialize WhisperKit for real-time transcription
    let whisper = Arc::new(WhisperKit::new());
    println!("Initializing WhisperKit (model: small.en) with 120s timeout...");
    
    // Try to initialize with timeout - use "small.en" for better English transcription
    let init_result = tokio::time::timeout(
        std::time::Duration::from_secs(120),
        whisper.initialize(Some("small.en"))
    ).await;
    
    match init_result {
        Ok(Ok(_)) => println!("✓ WhisperKit initialized successfully"),
        Ok(Err(e)) => {
            eprintln!("✗ Failed to initialize WhisperKit: {}", e);
            eprintln!("Continuing without transcription...");
        }
        Err(_) => {
            eprintln!("✗ WhisperKit initialization timed out after 120s");
            eprintln!("Continuing without transcription...");
        }
    }

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

    // Channel for transcription
    let (transcription_tx, transcription_rx) = crossbeam_channel::unbounded::<Vec<f32>>();
    
    println!("Starting audio mixer...");
    
    // Run transcription task in spawn_blocking to avoid Send issue with FFI
    let whisper_clone = whisper.clone();
    tokio::task::spawn_blocking(move || {
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async move {
            transcription_task(transcription_rx, whisper_clone).await;
        });
    });

    // Run mixer in blocking task since it blocks indefinitely
    tokio::task::spawn_blocking(move || {
        mixer(rx, output_sample_rate, transcription_tx);
    });

    println!("Starting mic and speaker streams...");
    
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
    speaker: Speaker,
) {
    let mut stream = match speaker.stream() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to create speaker stream: {}", e);
            return;
        }
    };

    while let Some(chunk) = stream.next().await {
        if tx.send(AudioSource::System(chunk)).is_err() {
            eprintln!("Speaker receiver dropped");
            break;
        }
    }
}
