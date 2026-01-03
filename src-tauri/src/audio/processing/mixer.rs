use crossbeam_channel::Receiver;
use hound;
use std::path::PathBuf;

use crate::types::AudioSource;

/// Mix microphone and system audio streams and write to WAV file
pub fn mixer(
    rx: Receiver<AudioSource>,
    sample_rate: u32,
) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    // Create recordings directory in project root (one level up from src-tauri)
    let project_root = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().and_then(|p| p.parent()).map(|p| p.to_path_buf()))
        .unwrap_or_else(|| {
            // Fallback: try to go up from current_dir
            std::env::current_dir()
                .ok()
                .and_then(|d| d.parent().map(|p| p.to_path_buf()))
                .unwrap_or_else(|| PathBuf::from(".."))
        });
    
    let recordings_dir = project_root.join("recordings");
    if !recordings_dir.exists() {
        std::fs::create_dir_all(&recordings_dir).ok();
    }
    
    eprintln!("📁 Recordings directory: {}", recordings_dir.display());
    
    // Generate filename with timestamp
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|_| std::time::Duration::from_secs(0))
        .as_secs();
    let output_path = recordings_dir.join(format!("noter_mixed_{}.wav", timestamp));
    
    let mut writer = match hound::WavWriter::create(&output_path, spec) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("❌ Failed to create WAV file: {}", e);
            return;
        }
    };
    eprintln!("Recording to: {}", output_path.display());

    // Pre-allocate buffers with reasonable capacity
    const BUFFER_CAPACITY: usize = 48000; // 1 second at 48kHz
    let mut mic_buf = Vec::<f32>::with_capacity(BUFFER_CAPACITY);
    let mut sys_buf = Vec::<f32>::with_capacity(BUFFER_CAPACITY);

    let mic_gain = 1.0;
    let sys_gain = 1.0;
    
    eprintln!("Mixer started for WAV recording at {}Hz...", sample_rate);

    loop {
        match rx.recv() {
            Ok(AudioSource::Mic(data)) => mic_buf.extend(data),
            Ok(AudioSource::System(data)) => sys_buf.extend(data),
            Err(_) => {
                // Channel closed, flush remaining data and exit
                eprintln!("Audio channel closed, flushing mixer...");
                break;
            }
        }

        let len = mic_buf.len().min(sys_buf.len());
        if len == 0 {
            continue;
        }

        for i in 0..len {
            let mixed = mic_buf[i] * mic_gain + sys_buf[i] * sys_gain;

            let s = (mixed.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            if let Err(e) = writer.write_sample(s) {
                eprintln!("❌ Failed to write sample: {} - stopping mixer", e);
                return; // Exit mixer on write error
            }
        }

        mic_buf.drain(..len);
        sys_buf.drain(..len);
    }

    // Flush remaining samples
    let len = mic_buf.len().min(sys_buf.len());
    for i in 0..len {
        let mixed = mic_buf[i] * mic_gain + sys_buf[i] * sys_gain;
        let s = (mixed.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        if let Err(e) = writer.write_sample(s) {
            eprintln!("⚠️ Failed to write final sample: {}", e);
            break;
        }
    }
    
    // Finalize WAV file
    if let Err(e) = writer.finalize() {
        eprintln!("❌ Failed to finalize WAV file: {}", e);
    } else {
        eprintln!("✅ Mixer completed, WAV file written to {}", output_path.display());
    }
}
