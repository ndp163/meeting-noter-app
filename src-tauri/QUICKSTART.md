# Quick Start Guide - WhisperKit Real-Time Transcription

## 🚀 Get Started in 3 Steps

### Step 1: Setup WhisperKit

```bash
cd src-tauri
./setup-whisperkit.sh
```

### Step 2: Build the Project

```bash
cargo build
```

### Step 3: Run

```bash
cargo run
```

## ✅ What You'll See

When the app starts:

```
Initializing WhisperKit...
WhisperKit initialized successfully
Recording mic (48000Hz) + system (48000Hz) → mixed.wav
```

When you speak or play audio:

```
📝 Transcription: Hello, this is a test...
📝 Transcription: The quick brown fox jumps over the lazy dog...
```

## 🎯 Key Features

- ✅ **Real-time transcription** - Updates every 3 seconds
- ✅ **Mic + System audio** - Captures both input and output
- ✅ **Offline processing** - No internet required
- ✅ **WAV recording** - Saves to `mixed.wav`

## ⚙️ Configuration

### Change Model (in lib.rs)

```rust
// Faster, less accurate
whisper.initialize(Some("tiny")).await

// Balanced (default)
whisper.initialize(None).await  // uses "base"

// More accurate, slower
whisper.initialize(Some("small")).await
```

### Adjust Transcription Frequency (in lib.rs)

```rust
// Process every 5 seconds instead of 3
const TRANSCRIPTION_CHUNK_SIZE: usize = 16000 * 5;
```

## 🔧 Troubleshooting

### Issue: Setup script fails

```bash
# Install Xcode Command Line Tools
xcode-select --install

# Then retry
./setup-whisperkit.sh
```

### Issue: No transcription output

- Speak clearly for at least 3 seconds
- Check System Preferences > Security & Privacy > Microphone
- Look for "WhisperKit initialized successfully" message

### Issue: Build errors

```bash
# Clean and rebuild
cargo clean
cd src-tauri
swift package clean
./setup-whisperkit.sh
cargo build
```

## 📚 More Info

- **Detailed Guide**: [WHISPERKIT_README.md](WHISPERKIT_README.md)
- **Implementation Details**: [IMPLEMENTATION_SUMMARY.md](IMPLEMENTATION_SUMMARY.md)
- **WhisperKit Docs**: https://github.com/argmaxinc/WhisperKit

## 💡 Tips

1. **First Run**: May take a few minutes to download models (~150MB)
2. **Best Performance**: Use "tiny" model for fastest real-time transcription
3. **Accuracy**: Use "small" model for best accuracy (requires more resources)
4. **Privacy**: All processing happens on-device, no cloud services

---

Need help? Check the error messages in the console - they usually explain what's wrong!
