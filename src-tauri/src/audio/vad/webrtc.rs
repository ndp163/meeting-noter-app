use webrtc_vad::Vad;

pub struct WebRtcVAD {
    vad: Vad,
}

impl WebRtcVAD {
    pub fn new() -> Result<Self, String> {
        let mut vad = Vad::new();
        // Set aggressiveness mode (0-3)
        // 0: least aggressive, 3: most aggressive
        vad.set_mode(webrtc_vad::VadMode::Aggressive);
        
        Ok(Self { vad })
    }

    /// Detect speech in audio chunk (must be 10, 20, or 30ms at 16kHz)
    /// Returns true if speech detected
    pub fn has_speech(&mut self, audio: &[i16]) -> Result<bool, String> {
        // WebRTC VAD requires specific frame sizes
        // For 16kHz: 160 samples (10ms), 320 samples (20ms), or 480 samples (30ms)
        
        if audio.len() != 160 && audio.len() != 320 && audio.len() != 480 {
            return Err(format!(
                "Invalid frame size: {}. WebRTC VAD requires 160, 320, or 480 samples at 16kHz",
                audio.len()
            ));
        }
        
        match self.vad.is_voice_segment(audio) {
            Ok(result) => Ok(result),
            Err(e) => Err(format!("VAD error: {:?}", e)),
        }
    }

    /// Analyze audio in chunks and return overall speech probability
    pub fn analyze(&mut self, audio: &[i16]) -> VadResult {
        // Split audio into 30ms chunks (480 samples at 16kHz)
        const CHUNK_SIZE: usize = 480;
        
        let mut speech_chunks = 0;
        let mut total_chunks = 0;
        
        for chunk in audio.chunks(CHUNK_SIZE) {
            if chunk.len() == CHUNK_SIZE {
                if let Ok(true) = self.has_speech(chunk) {
                    speech_chunks += 1;
                }
                total_chunks += 1;
            }
        }
        
        let confidence = if total_chunks > 0 {
            speech_chunks as f32 / total_chunks as f32
        } else {
            0.0
        };
        
        // Consider speech if > 30% of chunks contain speech
        let has_speech = confidence > 0.3;
        
        VadResult {
            has_speech,
            confidence,
        }
    }
}

pub struct VadResult {
    pub has_speech: bool,
    pub confidence: f32,
}
