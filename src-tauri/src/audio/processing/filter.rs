use regex::Regex;

/// Filter out non-speech content that transcription might output
/// Removes tags like [Music], [Breathing], etc.
pub fn filter_non_speech(text: &str) -> String {
    // Remove common non-speech tags that Whisper outputs
    let non_speech_patterns = [
        "silence", "chanting", "crying", "crow cawing", "crashing",
        "squeaking", "clipper sounds", "chiming",
        "BLANK_AUDIO", "Music", "music", "Applause", "applause",
        "Laughter", "laughter", "Breathing", "breathing",
        "Birds chirping", "birds chirping", "Bell", "bell",
        "Dog barking", "dog barking", "Dramatic music", "dramatic music",
        "cow mooing", "chewing", "Sounds of a bird", "sounds of a bird calling",
        "sounds of pain", "sneezes",
    ];
    
    let mut result = text.to_string();
    
    // Remove brackets and parentheses with non-speech patterns
    for pattern in &non_speech_patterns {
        let lower_pattern = pattern.to_lowercase();
        result = result.replace(&format!("[{}]", pattern), "");
        result = result.replace(&format!("({})", pattern), "");
        result = result.replace(&format!("[{}]", lower_pattern), "");
        result = result.replace(&format!("({})", lower_pattern), "");
    }
    
    // Remove any remaining brackets/parentheses with typical non-speech indicators
    let re = Regex::new(r"[\[\(][^\]\)]*(?:music|sound|breathing|chirping|barking|bell|applause|laughter|blank|silence|crying|crashing|squeaking|chanting|chiming)[^\]\)]*[\]\)]").unwrap();
    result = re.replace_all(&result, "").to_string();
    
    // Clean up multiple spaces
    let re_spaces = Regex::new(r"\s+").unwrap();
    result = re_spaces.replace_all(&result, " ").to_string();
    
    result.trim().to_string()
}
