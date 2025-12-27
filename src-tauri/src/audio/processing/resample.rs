/// Resample audio from 48kHz to 16kHz using simple decimation
/// For production, consider using rubato for better quality
pub fn resample_to_16khz(audio: &[f32]) -> Vec<f32> {
    const INPUT_RATE: usize = 48000;
    const OUTPUT_RATE: usize = 16000;
    
    if audio.is_empty() {
        return Vec::new();
    }
    
    // Calculate ratio - assuming input is 48kHz
    let ratio = INPUT_RATE as f32 / OUTPUT_RATE as f32; // 3.0
    let output_len = (audio.len() as f32 / ratio) as usize;
    let mut output = Vec::with_capacity(output_len);
    
    for i in 0..output_len {
        let src_idx = (i as f32 * ratio) as usize;
        if src_idx < audio.len() {
            output.push(audio[src_idx]);
        }
    }
    
    output
}

/// Ultra-fast resampling: just take every 3rd sample
/// 48kHz -> 16kHz (3:1 ratio)
pub fn resample_to_16khz_fast(audio: &[f32]) -> Vec<f32> {
    audio.iter().step_by(3).copied().collect()
}
