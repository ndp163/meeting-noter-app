// Shared types used across the codebase

/// Audio source identifier for the dual-stream architecture
#[derive(Debug, Clone)]
pub enum AudioSource {
    Mic(Vec<f32>),
    System(Vec<f32>),
}
