use crossbeam_channel::Receiver;
use hound;
use std::io::{Seek, Write};
use std::sync::Arc;

use crate::audio::alignment::{StreamAlignment, StreamKind};
use crate::paths;
use crate::types::AudioSource;
use crate::audio::processing::Resampler;
use crate::audio::constants::{MIXER_BUFFER_CAPACITY, DEFAULT_MIC_GAIN, DEFAULT_SYSTEM_GAIN};

/// How far one source may run ahead of the other before the lagging one is
/// treated as stalled. Well above normal chunk jitter.
const MAX_SOURCE_LAG_SECS: u32 = 2;

/// Mix microphone and system audio streams and write to WAV file.
///
/// `target_rate` is the WAV's sample rate; mic and system audio are resampled
/// to it first. They can arrive at different rates — a Bluetooth mic forced
/// into the 16kHz HFP call profile while system audio stays 48kHz — and the
/// index-pairing mix below assumes both share one rate, so the resample is
/// what keeps the mic from playing back too fast and the two tracks aligned.
///
/// `has_mic` tells the mixer whether to expect a microphone source. When there
/// is no mic, system audio is written on its own (otherwise the index-pairing
/// below would wait forever for mic samples and produce an empty file).
#[tracing::instrument(skip(rx, alignment), fields(meeting_id))]
pub fn mixer(
    rx: Receiver<AudioSource>,
    target_rate: u32,
    mic_rate: u32,
    meeting_id: Option<String>,
    has_mic: bool,
    alignment: Arc<StreamAlignment>,
) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: target_rate,
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

    // Per-source tracks for offline diarization: speaker.wav (remote audio only)
    // and mic.wav (your voice only). These let the diarization pass separate
    // remote speakers cleanly and treat the mic as a known "You".
    let parent = output_path.parent().map(|p| p.to_path_buf());
    let mut speaker_writer = parent
        .as_ref()
        .and_then(|dir| create_track_writer(&dir.join("speaker.wav"), spec));
    let mut mic_writer = if has_mic {
        parent
            .as_ref()
            .and_then(|dir| create_track_writer(&dir.join("mic.wav"), spec))
    } else {
        None
    };

    // Pre-allocate buffers with reasonable capacity
    let mut mic_buf = Vec::<f32>::with_capacity(MIXER_BUFFER_CAPACITY);
    let mut sys_buf = Vec::<f32>::with_capacity(MIXER_BUFFER_CAPACITY);

    // System audio is captured at the target rate already; only the mic can
    // arrive at a different rate (e.g. a Bluetooth HFP mic at 16kHz).
    let mut mic_resampler = Resampler::new(mic_rate, target_rate);

    let mic_gain = DEFAULT_MIC_GAIN;
    let sys_gain = DEFAULT_SYSTEM_GAIN;
    let max_lag = (MAX_SOURCE_LAG_SECS * target_rate) as usize;

    // Shared t=0 for the two sources, measured once in the stream handlers
    // (see `audio::alignment`). The later-starting source is padded with
    // leading silence — in its track file and its mix buffer — so mic.wav,
    // speaker.wav and audio.wav all live on the same timeline as the
    // transcription timestamps. Reading the same registry the pipelines use
    // keeps WAV time and transcript/word time from ever disagreeing.
    let mut mic_padded = !has_mic;
    let mut sys_padded = false;
    let pad_samples = |kind: StreamKind| -> usize {
        (f64::from(alignment.offset_sec(kind)) * f64::from(target_rate)) as usize
    };

    tracing::info!(target_rate, mic_rate, mic_gain, sys_gain, has_mic, "Mixer started for WAV recording");

    loop {
        match rx.recv() {
            Ok(AudioSource::Mic(data)) => {
                if !mic_padded {
                    mic_padded = true;
                    let pad = pad_samples(StreamKind::Mic);
                    if pad > 0 {
                        tracing::info!(samples = pad, "Padding mic track to the shared start");
                        write_silence(&mut mic_writer, pad);
                        mic_buf.resize(pad, 0.0);
                    }
                }
                let data = mic_resampler.process(&data);
                write_track(&mut mic_writer, &data, mic_gain);
                mic_buf.extend(data);
            }
            Ok(AudioSource::System(data)) => {
                if !sys_padded {
                    sys_padded = true;
                    let pad = pad_samples(StreamKind::Speaker);
                    if pad > 0 {
                        tracing::info!(samples = pad, "Padding speaker track to the shared start");
                        write_silence(&mut speaker_writer, pad);
                        sys_buf.resize(pad, 0.0);
                    }
                }
                write_track(&mut speaker_writer, &data, sys_gain);
                sys_buf.extend(data);
            }
            Err(_) => {
                tracing::debug!("Audio channel closed, flushing mixer...");
                break;
            }
        }

        let result = if has_mic {
            // After pairing, at most one buffer is non-empty. If it has run
            // far ahead, the other source has stalled (mic unplugged, device
            // switched): write the lead out against silence instead of
            // buffering it forever, which would freeze audio.wav and grow RAM.
            // Only once both have started: a slow-starting source (e.g. a
            // Bluetooth mic) is lined up by the start padding above instead.
            let started = mic_padded && sys_padded;
            write_mixed(&mut writer, &mut mic_buf, &mut sys_buf, mic_gain, sys_gain).and_then(|_| {
                if !started {
                    return Ok(());
                }
                drain_stalled(&mut writer, &mut mic_buf, mic_gain, &mut speaker_writer, max_lag)?;
                drain_stalled(&mut writer, &mut sys_buf, sys_gain, &mut mic_writer, max_lag)
            })
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

    if let Some(w) = speaker_writer.take() {
        if let Err(e) = w.finalize() {
            tracing::error!("Failed to finalize speaker.wav: {}", e);
        }
    }
    if let Some(w) = mic_writer.take() {
        if let Err(e) = w.finalize() {
            tracing::error!("Failed to finalize mic.wav: {}", e);
        }
    }
}

type TrackWriter = hound::WavWriter<std::io::BufWriter<std::fs::File>>;

/// Create a per-source WAV writer, logging and returning `None` on failure so a
/// missing side track never aborts the main recording.
fn create_track_writer(path: &std::path::Path, spec: hound::WavSpec) -> Option<TrackWriter> {
    match hound::WavWriter::create(path, spec) {
        Ok(w) => Some(w),
        Err(e) => {
            tracing::error!(path = %path.display(), "Failed to create track WAV: {}", e);
            None
        }
    }
}

/// Write gain-applied samples to an optional per-source track writer.
fn write_track(writer: &mut Option<TrackWriter>, data: &[f32], gain: f32) {
    if let Some(w) = writer.as_mut() {
        for &sample in data {
            if w.write_sample(to_i16(sample * gain)).is_err() {
                return;
            }
        }
    }
}

/// Prepend silence to an optional per-source track writer.
fn write_silence(writer: &mut Option<TrackWriter>, samples: usize) {
    if let Some(w) = writer.as_mut() {
        for _ in 0..samples {
            if w.write_sample(0i16).is_err() {
                return;
            }
        }
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

/// If `lead` holds more than `max_lag` unpaired samples, the other source has
/// stalled: write `lead` out mixed against silence, and pad the stalled
/// source's own track by the same amount so it stays on the shared timeline.
fn drain_stalled<W: Write + Seek>(
    writer: &mut hound::WavWriter<W>,
    lead: &mut Vec<f32>,
    gain: f32,
    stalled_track: &mut Option<TrackWriter>,
    max_lag: usize,
) -> Result<(), hound::Error> {
    if lead.len() <= max_lag {
        return Ok(());
    }
    write_silence(stalled_track, lead.len());
    write_mono(writer, lead, gain)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn wav_writer() -> hound::WavWriter<std::io::Cursor<Vec<u8>>> {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 16_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        hound::WavWriter::new(std::io::Cursor::new(Vec::new()), spec).unwrap()
    }

    #[test]
    fn stalled_source_does_not_hold_back_the_mix() {
        let mut writer = wav_writer();
        let mut mic_buf = Vec::new(); // mic died: nothing arrives
        let mut sys_buf = vec![0.5; 5_000];

        write_mixed(&mut writer, &mut mic_buf, &mut sys_buf, 1.0, 1.0).unwrap();
        drain_stalled(&mut writer, &mut sys_buf, 1.0, &mut None, 4_000).unwrap();

        assert!(sys_buf.is_empty(), "lead past the lag cap must be written out");
        assert_eq!(writer.len(), 5_000);
    }

    #[test]
    fn normal_jitter_waits_for_the_other_source() {
        let mut writer = wav_writer();
        let mut sys_buf = vec![0.5; 3_000];

        drain_stalled(&mut writer, &mut sys_buf, 1.0, &mut None, 4_000).unwrap();

        assert_eq!(sys_buf.len(), 3_000, "within the cap, keep buffering to pair");
        assert_eq!(writer.len(), 0);
    }
}
