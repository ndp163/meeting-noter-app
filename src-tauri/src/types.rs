/// Audio source type for the mixer
#[derive(Debug, Clone)]
pub enum AudioSource {
    Mic(Vec<f32>),
    System(Vec<f32>),
}
