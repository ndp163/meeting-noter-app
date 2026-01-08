use crossbeam_channel::Receiver;
use hound;

use crate::paths;
use crate::types::AudioSource;
use crate::audio::constants::{MIXER_BUFFER_CAPACITY, DEFAULT_MIC_GAIN, DEFAULT_SYSTEM_GAIN};

/// Mix microphone and system audio streams and write to WAV file
#[tracing::instrument(skip(rx), fields(meeting_id))]
pub fn mixer(
    rx: Receiver<AudioSource>,
    sample_rate: u32,
    meeting_id: Option<String>,
) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    // Determine output path based on meeting_id
    let output_path = if let Some(ref id) = meeting_id {
        // Save directly to meeting folder
        match paths::ensure_meeting_dir(id) {
            Ok(meeting_dir) => meeting_dir.join("audio.wav"),
            Err(e) => {
                tracing::error!("Failed to create meeting directory: {}", e);
                return;
            }
        }
    } else {
        // Fallback: save with timestamp (legacy behavior)
        let recordings_dir = match paths::ensure_recordings_dir() {
            Ok(dir) => dir,
            Err(e) => {
                tracing::error!("Failed to create recordings directory: {}", e);
                return;
            }
        };
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to get system time: {}, using 0", e);
                std::time::Duration::from_secs(0)
            })
            .as_secs();
        recordings_dir.join(format!("noter_mixed_{}.wav", timestamp))
    };
    
    tracing::info!(path = %output_path.display(), "Audio output path configured");
    
    let mut writer = match hound::WavWriter::create(&output_path, spec) {
        Ok(w) => w,
        Err(e) => {
            tracing::error!("Failed to create WAV file: {}", e);
            return;
        }
    };
    tracing::debug!(path = %output_path.display(), "Recording started");

    // Pre-allocate buffers with reasonable capacity
    let mut mic_buf = Vec::<f32>::with_capacity(MIXER_BUFFER_CAPACITY);
    let mut sys_buf = Vec::<f32>::with_capacity(MIXER_BUFFER_CAPACITY);

    let mic_gain = DEFAULT_MIC_GAIN;
    let sys_gain = DEFAULT_SYSTEM_GAIN;
    
    tracing::info!(sample_rate, mic_gain, sys_gain, "Mixer started for WAV recording");

    loop {
        match rx.recv() {
            Ok(AudioSource::Mic(data)) => mic_buf.extend(data),
            Ok(AudioSource::System(data)) => sys_buf.extend(data),
            Err(_) => {
                // Channel closed, flush remaining data and exit
                tracing::debug!("Audio channel closed, flushing mixer...");
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
                tracing::error!("Failed to write sample: {} - stopping mixer", e);
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
            tracing::warn!("Failed to write final sample: {}", e);
            break;
        }
    }
    
    // Finalize WAV file
    if let Err(e) = writer.finalize() {
        tracing::error!("Failed to finalize WAV file: {}", e);
    } else {
        tracing::info!(path = %output_path.display(), "Mixer completed, WAV file written");
    }
}
