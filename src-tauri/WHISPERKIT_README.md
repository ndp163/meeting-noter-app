# WhisperKit Real-Time Transcription Integration

This project integrates WhisperKit 0.15.0 for real-time audio transcription.

## Prerequisites

### macOS Requirements

- macOS 13.0 or later
- Xcode 15.0 or later
- Swift 5.9 or later

### Installing WhisperKit

1. **Build the Swift bridge:**

   ```bash
   cd src-tauri
   ./build-swift.sh
   ```

2. **Download WhisperKit models:**
   WhisperKit will automatically download the required models on first run. You can also pre-download specific models:
   ```bash
   # Using WhisperKit CLI (optional)
   swift run whisperkit-cli download --model tiny
   ```

## Building the Project

### Development Build

```bash
# From the src-tauri directory
./build-swift.sh
cargo build
```

### Production Build

```bash
# From the src-tauri directory
./build-swift.sh
cargo build --release
```

## How It Works

1. **Audio Capture**: The application captures audio from both:

   - Microphone input
   - System audio (speaker output)

2. **Audio Processing**:

   - Audio streams are mixed in real-time
   - Mixed audio is saved to `mixed.wav`
   - Audio chunks are buffered for transcription

3. **Real-Time Transcription**:
   - Every 3 seconds of audio is sent to WhisperKit
   - WhisperKit processes the audio using on-device ML models
   - Transcriptions are printed to console in real-time
   - Format: `📝 Transcription: [text]`

## Configuration

### Transcription Parameters

You can adjust these constants in `src/lib.rs`:

```rust
// Buffer size for transcription (30 seconds at 16kHz)
const TRANSCRIPTION_BUFFER_SIZE: usize = 16000 * 30;

// Process every 3 seconds
const TRANSCRIPTION_CHUNK_SIZE: usize = 16000 * 3;
```

### WhisperKit Model Selection

To use a specific model, modify the initialization in `src/lib.rs`:

```rust
// Default (base model)
whisper.initialize(None).await

// Specific model
whisper.initialize(Some("tiny")).await
whisper.initialize(Some("base")).await
whisper.initialize(Some("small")).await
```

Available models:

- `tiny`: Fastest, less accurate (~75MB)
- `base`: Balanced (~145MB)
- `small`: More accurate, slower (~490MB)

## Troubleshooting

### Swift Bridge Build Fails

If the Swift bridge fails to build:

1. Check Swift version:

   ```bash
   swift --version
   ```

   Should be 5.9 or later

2. Verify Xcode installation:

   ```bash
   xcode-select -p
   ```

3. Clean and rebuild:
   ```bash
   cd src-tauri
   swift package clean
   ./build-swift.sh
   ```

### WhisperKit Initialization Fails

If WhisperKit fails to initialize:

1. Check if models are downloaded:

   ```bash
   ls ~/Library/Caches/whisperkit/
   ```

2. Try downloading models manually:
   ```bash
   # The app will download automatically on first run
   # Just wait for the download to complete
   ```

### No Transcription Output

If you don't see transcription output:

1. Check that audio is being captured:

   - Look for "Recording mic + system → mixed.wav" message
   - Verify `mixed.wav` file is being created

2. Check WhisperKit initialization:

   - Look for "WhisperKit initialized successfully" message
   - If you see an error, transcription will be disabled

3. Speak clearly or play audio for at least 3 seconds

## Architecture

```
┌─────────────────┐     ┌──────────────────┐
│   Microphone    │────▶│                  │
└─────────────────┘     │   Audio Mixer    │──┬──▶ mixed.wav
                        │                  │  │
┌─────────────────┐     │                  │  │
│ System Audio    │────▶│                  │  │
└─────────────────┘     └──────────────────┘  │
                                               │
                                               ▼
                        ┌──────────────────────────┐
                        │ Transcription Buffer     │
                        │ (Rolling 30s window)     │
                        └──────────────────────────┘
                                   │
                                   ▼
                        ┌──────────────────────────┐
                        │   WhisperKit Bridge      │
                        │   (Swift/Rust FFI)       │
                        └──────────────────────────┘
                                   │
                                   ▼
                        ┌──────────────────────────┐
                        │   WhisperKit ML Model    │
                        │   (On-device inference)  │
                        └──────────────────────────┘
                                   │
                                   ▼
                        📝 Real-time Transcription
```

## Performance Tips

1. **Model Selection**: Use `tiny` model for fastest real-time performance
2. **Chunk Size**: Smaller chunks = more frequent updates but higher CPU usage
3. **Buffer Size**: Larger buffer provides more context but uses more memory

## License

This integration uses:

- WhisperKit (MIT License) - https://github.com/argmaxinc/WhisperKit
- OpenAI Whisper (MIT License)
