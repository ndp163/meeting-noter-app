// Example: Sử dụng các transcription engines khác nhau

use noter_lib::{AudioConfig, EngineType, Recorder};
use crossbeam_channel::unbounded;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Transcription Engine Example ===\n");

    // Example 1: Sử dụng FluidAudio (mặc định, nhanh và chính xác)
    println!("1. FluidAudio Engine (Recommended):");
    let config_fluid = AudioConfig::default()
        .with_engine(EngineType::FluidAudio)
        .with_timeout(120);
    
    let mut recorder_fluid = Recorder::new(config_fluid);
    
    // Tạo channel để nhận transcription events
    let (tx, rx) = unbounded();
    
    // Start recording với FluidAudio
    tokio::spawn(async move {
        if let Err(e) = recorder_fluid.start(Some(tx)).await {
            eprintln!("Error starting FluidAudio recorder: {}", e);
        }
    });

    // Nhận và in ra kết quả transcription
    for event in rx.iter() {
        println!("[{}] {}", event.source.as_str(), event.result.text);
    }

    println!("\n");

    // Example 2: Sử dụng WhisperKit với model cụ thể
    println!("2. WhisperKit Engine:");
    let config_whisper = AudioConfig::default()
        .with_engine(EngineType::WhisperKit)
        .with_model("small.en")  // Có thể dùng: tiny, base, small, medium, large
        .with_timeout(180);      // WhisperKit cần thời gian init lâu hơn

    let mut recorder_whisper = Recorder::new(config_whisper);
    
    let (tx2, rx2) = unbounded();
    
    tokio::spawn(async move {
        if let Err(e) = recorder_whisper.start(Some(tx2)).await {
            eprintln!("Error starting WhisperKit recorder: {}", e);
        }
    });

    for event in rx2.iter() {
        println!("[{}] {}", event.source.as_str(), event.result.text);
    }

    Ok(())
}

// Example 3: Dynamic switching (tạo recorder mới với engine khác)
async fn switch_engine_example() -> anyhow::Result<()> {
    let current_engine = EngineType::FluidAudio;
    
    println!("Current engine: {:?}", current_engine);
    
    // Tạo config với engine hiện tại
    let config = AudioConfig::default()
        .with_engine(current_engine);
    
    let mut recorder = Recorder::new(config);
    
    // ... sử dụng recorder ...
    
    // Để chuyển sang engine khác, tạo recorder mới
    let new_engine = EngineType::WhisperKit;
    let new_config = AudioConfig::default()
        .with_engine(new_engine)
        .with_model("base.en");
    
    let mut new_recorder = Recorder::new(new_config);
    
    println!("Switched to: {:?}", new_engine);
    
    Ok(())
}

// Example 4: Chỉ record audio mà không transcribe
async fn recording_only_example() -> anyhow::Result<()> {
    let config = AudioConfig::default();
    let mut recorder = Recorder::new(config);
    
    // Pass None để không nhận transcription events
    // Audio vẫn sẽ được record vào WAV file
    recorder.start(None).await?;
    
    Ok(())
}
