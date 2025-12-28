use webrtc_vad::Vad;

// Constants
const SAMPLE_RATE: u32 = 16000;
const FRAME_10MS: usize = 160;  // 10ms at 16kHz
const FRAME_20MS: usize = 320;  // 20ms at 16kHz
const FRAME_30MS: usize = 480;  // 30ms at 16kHz
const DEFAULT_CONFIDENCE_THRESHOLD: f32 = 0.3;

#[derive(Debug, Clone, Copy)]
pub struct VadResult {
    pub has_speech: bool,
    pub confidence: f32,
    pub speech_chunks: usize,
    pub total_chunks: usize,
}

pub struct WebRtcVAD {
    vad: Vad,
    confidence_threshold: f32,
}

impl Default for WebRtcVAD {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

impl WebRtcVAD {
    /// Creates a new WebRTC VAD with Aggressive mode
    pub fn new() -> Result<Self, String> {
        Self::with_mode(webrtc_vad::VadMode::Aggressive)
    }
    
    /// Creates a new WebRTC VAD with specified mode
    pub fn with_mode(mode: webrtc_vad::VadMode) -> Result<Self, String> {
        let mut vad = Vad::new();
        vad.set_mode(mode);
        
        Ok(Self {
            vad,
            confidence_threshold: DEFAULT_CONFIDENCE_THRESHOLD,
        })
    }
    
    /// Set confidence threshold (0.0 - 1.0)
    pub fn set_threshold(&mut self, threshold: f32) {
        self.confidence_threshold = threshold.clamp(0.0, 1.0);
    }

    /// Detect speech in audio chunk
    /// Frame must be 10ms (160), 20ms (320), or 30ms (480) samples at 16kHz
    pub fn has_speech(&mut self, audio: &[i16]) -> Result<bool, String> {
        // Use match for cleaner validation
        match audio.len() {
            FRAME_10MS | FRAME_20MS | FRAME_30MS => {
                self.vad
                    .is_voice_segment(audio)
                    .map_err(|e| format!("VAD error: {:?}", e))
            }
            len => Err(format!(
                "Invalid frame size: {}. Expected {}, {}, or {} samples at {}Hz",
                len, FRAME_10MS, FRAME_20MS, FRAME_30MS, SAMPLE_RATE
            )),
        }
    }

    /// Analyze audio buffer and return detailed results
    pub fn analyze(&mut self, audio: &[i16]) -> VadResult {
        self.analyze_with_chunk_size(audio, FRAME_30MS)
    }
    
    /// Analyze with custom chunk size
    pub fn analyze_with_chunk_size(&mut self, audio: &[i16], chunk_size: usize) -> VadResult {
        debug_assert!(
            chunk_size == FRAME_10MS || chunk_size == FRAME_20MS || chunk_size == FRAME_30MS,
            "Invalid chunk size"
        );
        
        let mut speech_chunks = 0;
        let mut total_chunks = 0;
        
        // Process all complete chunks
        for chunk in audio.chunks_exact(chunk_size) {
            if let Ok(true) = self.has_speech(chunk) {
                speech_chunks += 1;
            }
            total_chunks += 1;
        }
        
        let confidence = if total_chunks > 0 {
            speech_chunks as f32 / total_chunks as f32
        } else {
            0.0
        };
        
        VadResult {
            has_speech: confidence > self.confidence_threshold,
            confidence,
            speech_chunks,
            total_chunks,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_sizes() {
        let mut vad = WebRtcVAD::new().unwrap();
        
        // Valid frame sizes
        assert!(vad.has_speech(&vec![0i16; FRAME_10MS]).is_ok());
        assert!(vad.has_speech(&vec![0i16; FRAME_20MS]).is_ok());
        assert!(vad.has_speech(&vec![0i16; FRAME_30MS]).is_ok());
        
        // Invalid frame size
        assert!(vad.has_speech(&vec![0i16; 100]).is_err());
    }
}
