use crossbeam_channel::Receiver;
use hound;
use std::io::{Seek, Write};

use crate::paths;
use crate::types::AudioSource;
use crate::audio::constants::{MIXER_BUFFER_CAPACITY, DEFAULT_MIC_GAIN, DEFAULT_SYSTEM_GAIN};

/// Mix microphone and system audio streams and write to WAV file.
///
/// `has_mic` tells the mixer whether to expect a microphone source. When there
/// is no mic, system audio is written on its own (otherwise the index-pairing
/// below would wait forever for mic samples and produce an empty file).
#[tracing::instrument(skip(rx), fields(meeting_id))]
pub fn mixer(
    rx: Receiver<AudioSource>,
    sample_rate: u32,
    meeting_id: Option<String>,
    has_mic: bool,
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
    
    tracing::info!(sample_rate, mic_gain, sys_gain, has_mic, "Mixer started for WAV recording");

    loop {
        match rx.recv() {
            Ok(AudioSource::Mic(data)) => mic_buf.extend(data),
            Ok(AudioSource::System(data)) => sys_buf.extend(data),
            Err(_) => {
                tracing::debug!("Audio channel closed, flushing mixer...");
                break;
            }
        }

        let result = if has_mic {
            write_mixed(&mut writer, &mut mic_buf, &mut sys_buf, mic_gain, sys_gain)
        } else {
            write_mono(&mut writer, &mut sys_buf, sys_gain)
        };
        if result.is_err() {
            tracing::error!("Failed to write sample - stopping mixer");
            return;
        }
    }

    // Flush whatever is left. With a mic, drain the paired part first, then the
    // trailing remainder of whichever source outlasted the other (the other is
    // silent for that span).
    if has_mic {
        let _ = write_mixed(&mut writer, &mut mic_buf, &mut sys_buf, mic_gain, sys_gain);
        let _ = write_mono(&mut writer, &mut mic_buf, mic_gain);
    }
    let _ = write_mono(&mut writer, &mut sys_buf, sys_gain);

    if let Err(e) = writer.finalize() {
        tracing::error!("Failed to finalize WAV file: {}", e);
    } else {
        tracing::info!(path = %output_path.display(), "Mixer completed, WAV file written");
    }
}

/// Mix the overlapping prefix of both buffers and drain it from each.
fn write_mixed<W: Write + Seek>(
    writer: &mut hound::WavWriter<W>,
    mic_buf: &mut Vec<f32>,
    sys_buf: &mut Vec<f32>,
    mic_gain: f32,
    sys_gain: f32,
) -> Result<(), hound::Error> {
    let len = mic_buf.len().min(sys_buf.len());
    for i in 0..len {
        let mixed = mic_buf[i] * mic_gain + sys_buf[i] * sys_gain;
        writer.write_sample(to_i16(mixed))?;
    }
    mic_buf.drain(..len);
    sys_buf.drain(..len);
    Ok(())
}

/// Write a single buffer (gain-applied) and clear it.
fn write_mono<W: Write + Seek>(
    writer: &mut hound::WavWriter<W>,
    buf: &mut Vec<f32>,
    gain: f32,
) -> Result<(), hound::Error> {
    for &sample in buf.iter() {
        writer.write_sample(to_i16(sample * gain))?;
    }
    buf.clear();
    Ok(())
}

#[inline]
fn to_i16(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
}
