# FluidAudio Parakeet v2 Test Suite

Test suite for the **Parakeet TDT v2** model from [FluidInference/FluidAudio](https://github.com/FluidInference/FluidAudio) with support for VAD (Voice Activity Detection) and real-time streaming.

## 📋 Features

This repo provides 3 test programs:

1. **BatchTranscribeTest** - Simple batch transcription
2. **VadStreamTest** - Real-time VAD streaming for speech detection
3. **StreamingTranscribeTest** - Streaming transcription with VAD-guided chunking

## 🔧 System Requirements

- **macOS 13.0+** (FluidAudio only supports macOS/iOS)
- **Xcode 15.0+** or Swift 5.9+
- **Apple Silicon (M1/M2/M3)** recommended to take advantage of the Apple Neural Engine

## 📦 Installation

### 1. Clone or download this repository

```bash
cd /Users/phuc.nguyendinh/personal/meeting-noter-app/model-test
```

### 2. Build the project

```bash
swift build
```

The first time it will:

- Download the FluidAudio SDK from GitHub
- Download the CoreML models from HuggingFace (parakeet-v2, silero-vad)
- Compile the models

## 🚀 Usage

### Test 1: Batch Transcription (Simplest)

Transcribe the entire audio file at once:

```bash
swift run BatchTranscribeTest ~/audio/meeting.wav
```

**Sample output:**

```
=== FluidAudio Parakeet v2 Batch Transcription Test ===

📁 Audio file: /Users/phuc/audio/meeting.wav
📥 Downloading and loading Parakeet v2 models...
✅ Models loaded in 3.45s
✅ ASR Manager initialized
🎵 Converting audio to 16kHz mono...
✅ Audio converted in 0.12s
   Duration: 45.30s
   Samples: 724800

🎤 Transcribing...

============================================================
TRANSCRIPTION RESULTS
============================================================

📝 Text: Hello everyone, welcome to today's meeting...

📊 Statistics:
   • Processing time: 0.238s
   • Audio duration: 45.30s
   • RTFx: 190.3x (faster than real-time)
   • Confidence: 94.50%

⏱️  Word Timings:
   • 0.00s - 0.50s: Hello
   • 0.50s - 1.20s: everyone
   ...

============================================================
✅ Test completed successfully!
```

### Test 2: VAD Streaming (Speech Detection)

Simulate real-time streaming with Voice Activity Detection:

```bash
swift run VadStreamTest ~/audio/meeting.wav
```

**Sample output:**

```
=== FluidAudio VAD Real-time Streaming Test ===

📁 Audio file: /Users/phuc/audio/meeting.wav
📥 Initializing VAD...
✅ VAD initialized
🎵 Loading audio...
✅ Audio loaded: 45.30s, 724800 samples

🔴 Starting real-time VAD simulation...
   Chunk size: 4096 samples (~256ms)
------------------------------------------------------------
[  0.00s] ⚪️ [░░░░░░░░░░░░░░░░░░░░] 0.023
[  0.26s] ⚪️ [░░░░░░░░░░░░░░░░░░░░] 0.045
[  0.51s] 🟢 [████████████░░░░░░░░] 0.612 🎙️  SPEECH START
[  0.77s] 🟢 [███████████████████░] 0.956
[  1.02s] 🟢 [████████████████████] 0.987
...
[  8.19s] ⚪️ [███░░░░░░░░░░░░░░░░░] 0.167 🔇 SPEECH END (duration: 7.68s)
...

============================================================
VAD ANALYSIS SUMMARY
============================================================

📊 Statistics:
   • Audio duration: 45.30s
   • Processing time: 0.037s
   • RTFx: 1224.3x faster than real-time
   • Chunks processed: 177

🎙️  Speech Segments Detected: 12
   1.   0.51s →   8.19s (duration: 7.68s)
   2.  10.24s →  15.87s (duration: 5.63s)
   ...

   Total speech time: 38.42s (84.8%)

============================================================
✅ Test completed successfully!
```

### Test 3: Streaming Transcription with VAD (Real-time)

Combine VAD and ASR to transcribe by segments:

```bash
# Use the v2 model (English-only, highest recall)
swift run StreamingTranscribeTest ~/audio/meeting.wav --model-version v2

# Or the v3 model (Multilingual, 25 European languages)
swift run StreamingTranscribeTest ~/audio/meeting.wav --model-version v3
```

**Sample output:**

```
=== FluidAudio Real-time Streaming Transcription Test ===
    (with VAD-guided speech detection)

📁 Audio file: /Users/phuc/audio/meeting.wav
🤖 Model version: v2 (English-only)

📥 Step 1/3: Initializing VAD...
✅ VAD ready

📥 Step 2/3: Loading Parakeet models...
✅ Models loaded in 2.87s
✅ ASR ready

📥 Step 3/3: Loading audio...
✅ Audio loaded: 45.30s

🔍 Running VAD to detect speech segments...
✅ VAD detected 12 speech segments in 0.04s

📊 Speech Segments:
------------------------------------------------------------
   Segment 1: 0.51s → 8.19s (duration: 7.68s)
   Segment 2: 10.24s → 15.87s (duration: 5.63s)
   ...

🎤 Transcribing speech segments...
============================================================

[1/12] Processing segment 0.51s - 8.19s...
   📝 "Hello everyone, welcome to today's meeting..."
   ✓ Confidence: 95.20%

[2/12] Processing segment 10.24s - 15.87s...
   📝 "Let's start with the project updates..."
   ✓ Confidence: 93.80%

...

============================================================
FULL TRANSCRIPTION
============================================================

[0.51s - 8.19s]
Hello everyone, welcome to today's meeting...

[10.24s - 15.87s]
Let's start with the project updates...

...

============================================================
PERFORMANCE SUMMARY
============================================================

📊 Statistics:
   • Audio duration: 45.30s
   • VAD processing: 0.037s
   • ASR processing: 0.238s
   • Total processing: 0.275s
   • RTFx: 164.7x faster than real-time
   • Speech segments: 12
   • Successful transcriptions: 12

============================================================
✅ Test completed successfully!
```

## 📝 Model Details

### Parakeet TDT v2 (English-only)

- **Model**: `FluidInference/parakeet-tdt-0.6b-v2-coreml`
- **Language**: English
- **Advantage**: Highest recall for English
- **Performance**: ~190x RTF on M4 Pro
- **Size**: ~600MB

### Parakeet TDT v3 (Multilingual)

- **Model**: `FluidInference/parakeet-tdt-0.6b-v3-coreml`
- **Languages**: 25 European languages
- **Performance**: ~190x RTF on M4 Pro
- **Size**: ~600MB

### Silero VAD

- **Model**: `FluidInference/silero-vad-coreml`
- **Feature**: Voice Activity Detection
- **Performance**: ~1200x RTF on M2
- **Size**: ~5MB

## 🎯 Use Cases

### 1. Meeting Transcription

```bash
# Transcribe a meeting with detailed timestamps
swift run StreamingTranscribeTest ~/Recordings/meeting.wav --model-version v2
```

### 2. Podcast Processing

```bash
# Batch transcription for a podcast
swift run BatchTranscribeTest ~/Podcasts/episode-123.mp3
```

### 3. Voice Activity Analysis

```bash
# Analyze when someone is speaking
swift run VadStreamTest ~/Audio/conversation.wav
```

### 4. Real-time Dictation

```bash
# Record from the microphone and transcribe in real-time
# (needs extra code to capture audio from the mic)
swift run StreamingTranscribeTest ~/Recordings/dictation.wav --model-version v2
```

## 🔧 Tuning Parameters

### VAD Configuration

The VAD parameters can be adjusted in code:

```swift
var segmentConfig = VadSegmentationConfig.default
segmentConfig.minSpeechDuration = 0.5  // At least 0.5s to count as speech
segmentConfig.minSilenceDuration = 0.3  // 0.3s of silence to split segments
segmentConfig.speechPadding = 0.1      // Add 0.1s of padding
segmentConfig.threshold = 0.5          // Probability threshold (0-1)
```

**Recommendations:**

- **Meeting transcription**: minSpeechDuration=0.5, minSilenceDuration=0.3
- **Dictation**: minSpeechDuration=0.25, minSilenceDuration=0.5
- **Noisy environment**: threshold=0.6 or 0.7

### ASR Configuration

```swift
let config = AsrConfig(
    computeUnits: .cpuAndNeuralEngine,  // Use the ANE for optimization
    maxAudioLength: 30.0                // Max 30s per chunk
)
```

## 📊 Performance Benchmarks

### M4 Pro (2024)

- **Parakeet v2**: ~190x RTF (1 hour of audio → ~19 seconds)
- **VAD**: ~1200x RTF
- **Combined (VAD + ASR)**: ~165x RTF

### M2 (2022)

- **Parakeet v2**: ~120x RTF (1 hour of audio → ~30 seconds)
- **VAD**: ~1220x RTF
- **Combined (VAD + ASR)**: ~110x RTF

### Memory Usage

- **VAD**: ~50MB
- **Parakeet v2/v3**: ~1.2GB
- **Combined**: ~1.3GB

## 🐛 Troubleshooting

### Error: "Models not found"

```bash
# Models are downloaded automatically on first run
# If the network is blocked, set a proxy:
export https_proxy=http://your-proxy:port
swift run BatchTranscribeTest audio.wav
```

### Error: "File not found"

```bash
# Check the file path
ls -la ~/audio/meeting.wav

# Or use an absolute path
swift run BatchTranscribeTest /Users/phuc/audio/meeting.wav
```

### Error: "Cannot load model"

```bash
# Clear the cache and re-download
rm -rf ~/.cache/fluidaudio/Models
swift run BatchTranscribeTest audio.wav
```

### Performance too slow

```bash
# Check whether it's running on the ANE
# Open Activity Monitor > CPU > Find the process
# If CPU usage is high → the ANE is not being used

# Solution: Make sure the model compiled correctly
swift build -c release
swift run -c release BatchTranscribeTest audio.wav
```

## 🧠 Real-time Transcription Processing Logic in Detail

### Overview

The system uses **VAD (Voice Activity Detection)** to detect voice → **ASR (Automatic Speech Recognition)** to transcribe, with 2 modes:

- **Streaming**: Transcribe continuously while speaking (real-time feedback)
- **Final**: Transcribe everything after speaking ends (most accurate)

### State Machine

**States:**

1. **IDLE**: `isSpeaking=false` - Waiting, not speaking
2. **SPEAKING**: `isSpeaking=true` - In a speech session
3. **SILENCE_IN_SPEECH**: `isSpeaking=true` + `silenceFrameCount>0` - Temporarily silent

**Transitions:**

```
IDLE
  → [detect voice prob>0.25]
  → SPEAKING

SPEAKING
  → [strong voice prob>0.35 + buffer≥0.25s]
  → Launch Streaming Transcription (do not clear buffer)
  → Stay in SPEAKING

SPEAKING
  → [no voice detected]
  → SILENCE_IN_SPEECH

SILENCE_IN_SPEECH
  → [silence < 4s]
  → Continue buffering + count silence

SILENCE_IN_SPEECH
  → [silence ≥ 4s]
  → Launch Final Transcription (clear buffer)
  → IDLE
```

### Audio Processing Pipeline

**1. Audio Capture**

```swift
bufferSize: 4096 frames
Native rate: 48kHz (Mac) → ~85ms/chunk
```

**2. Conversion**

```swift
48kHz → 16kHz mono Float32
4096 frames → ~1365 samples @ 16kHz
```

**3. VAD Detection**

```swift
processStreamingChunk() → VadResult
- probability: 0.0-1.0 (probability of voice)
- state: Streaming state for context
```

**Thresholds:**

- `probability > 0.35` → Strong voice ✅ Trigger streaming transcription
- `probability > 0.25` → Weak voice 🟡 Buffer only (no transcription)
- `probability ≤ 0.25` → No voice ⚪️ Silence

### Buffer Management

**Thread-Safety:**

```swift
bufferLock: NSLock
- Lock every append/copy/clear operation
- Avoid race conditions between the audio thread and transcribe tasks
```

**Buffer Operations:**

| Scenario          | Action                          | Clear Buffer?               | Lock |
| ----------------- | ------------------------------- | --------------------------- | ---- |
| Speech start      | `speechBuffer = []`             | ✅                          | ✅   |
| Voice detected    | `append(samples)`               | ❌                          | ✅   |
| Silence in speech | `append(samples)` (if count≤16) | ❌                          | ✅   |
| Streaming trigger | `copy()` buffer                 | ❌ (keep to keep transcribing) | ✅   |
| Final trigger     | `copy()` + `clear()`            | ✅ (prepare a new segment)   | ✅   |

**Buffer Limits:**

```swift
maxBufferSize = 16000 × 15 = 240,000 samples (15s)
minChunkSize = 16000 ÷ 4 = 4,000 samples (0.25s)
```

### Streaming Transcription

**Trigger Conditions:**

```swift
if isStrongVoice
   && currentBufferSize >= minChunkSize
   && activeTranscriptions < 3
```

**Process:**

1. 🔒 Lock buffer
2. 📋 Copy buffer (all audio from the start until now)
3. 🔓 Unlock (do NOT clear - keep accumulating)
4. 🎯 Enqueue to serial `transcriptionQueue`
5. 🔄 ASR transcribe async
6. 📝 Update UI with partial result

**Characteristics:**

- **Frequency**: Every time there is strong voice + enough buffer
- **Latency**: ~50-100ms
- **Purpose**: Real-time feedback for the user
- **Accuracy**: Good but may change as more context is added

### Final Transcription

**Trigger Conditions:**

```swift
if isSpeaking
   && !hasVoice
   && silenceFrameCount > 16 (~4 seconds)
```

**Process:**

1. 🔒 Lock buffer
2. 📋 Copy the entire buffer (complete speech segment)
3. 🗑️ Clear the buffer immediately
4. 🔓 Unlock
5. 🎯 Enqueue to serial `transcriptionQueue`
6. 🔄 ASR transcribe async
7. 📊 Show final result + stats
8. 🔄 Reset all state (VAD state, counters, flags)

**Characteristics:**

- **Frequency**: Once when speech ends
- **Latency**: ~60-80ms
- **Purpose**: Most accurate result
- **Accuracy**: Highest (full context from start to end)

### Silence Detection

**Counter Logic:**

```swift
silenceFrameCount = 0                    // Init
hasVoice → silenceFrameCount = 0         // Reset when voice is present
!hasVoice + isSpeaking → silenceFrameCount++ // Count silence within speech
```

**Timeout:**

```
silenceThreshold = 16 chunks
@ 48kHz native: 16 × ~85ms ≈ 1.4s (but VAD output slower)
Effective timeout: ~4 seconds
```

**During Silence:**

- **Count ≤ 16**: Continue buffering (allow pauses within a sentence)
- **Count > 16**: Trigger Final + reset all state

### Concurrency Control

**1. Buffer Protection:**

```swift
bufferLock: NSLock
- Serialize all buffer access
- Avoid race conditions
```

**2. Transcription Queue:**

```swift
transcriptionQueue: Serial DispatchQueue
- QoS: .userInitiated
- Process transcriptions sequentially
- Avoid model contention
```

**3. Active Transcription Limit:**

```swift
maxConcurrentTranscriptions = 3
activeTranscriptions counter
- Prevent overload
- Graceful degradation
```

### State Reset (After Final)

```swift
isSpeaking = false                    // Exit speech mode
silenceFrameCount = 0                 // Reset silence counter
currentTranscript = ""                // Clear display
speechBuffer = []                     // Clear buffer (already cleared earlier)
skippedShortAudio = 0                 // Reset debug counters
skippedLowVAD = 0
vadState = makeStreamState()          // Fresh VAD state
```

**Purpose:** Start completely clean for the next speech segment, avoiding stale state interference.

### Complete Timeline Example

```
T=0.0s:  ⚪️ Silence, isSpeaking=false
T=0.5s:  🟡 Weak voice (0.28) → isSpeaking=true, buffer starts
T=1.0s:  🟢 Strong voice (0.45) → buffer=16000 samples (1s)
         → Launch Streaming #1 (copy buffer, NO clear)
T=1.5s:  📝 Streaming result: "Hello"
T=2.0s:  🟢 Strong voice → buffer=32000 samples (2s)
         → Launch Streaming #2 (copy buffer, NO clear)
T=2.5s:  📝 Streaming result: "Hello how are"
T=3.0s:  🟢 Strong voice → buffer=48000 (3s)
         → Launch Streaming #3
T=3.5s:  ⚪️ Silence detected, silenceCount=1, continue buffer
T=4.0s:  ⚪️ silenceCount=2, continue buffer
...
T=7.0s:  ⚪️ silenceCount=16 > threshold
         → Copy buffer (56000 samples = 3.5s full speech)
         → Clear buffer immediately
         → Launch Final transcription
T=7.1s:  📊 Final: "Hello how are you today"
         → Reset VAD state, counters, flags
         → Back to IDLE state
T=7.5s:  ⚪️ Silence, isSpeaking=false (ready for next speech)
```

### Key Design Decisions

1. **Streaming does NOT clear the buffer**: To have full context for the next transcription, increasing accuracy
2. **Final clears the buffer immediately**: Avoid reusing stale data for a new segment
3. **Serial queue**: Avoid model conflicts, guarantee ordering
4. **VAD state reset**: Avoid old state interfering with a new segment
5. **Lock protection**: Thread-safe for the buffer shared between the audio thread and transcribe tasks
6. **4s silence threshold**: Balance between natural (allowing pauses) and responsive

### Critical Bug Fixes

**Original problem:** A 0.9s buffer (14400 samples) caused an "Invalid audio data" error after Final transcription

**Cause:**

1. The buffer was not protected by a lock → race condition
2. VAD state was not reset after Final → state corruption
3. Counters were not reset → wrong logic on the next segment

**Solution implemented:**

1. ✅ Added an NSLock for all buffer operations
2. ✅ Reset VAD state after each Final
3. ✅ Reset all counters (skippedShortAudio, skippedLowVAD)
4. ✅ Serial queue instead of Task.detached
5. ✅ Clear the buffer immediately after copy in Final mode

## 🔗 References

- **FluidAudio GitHub**: https://github.com/FluidInference/FluidAudio
- **Documentation**: https://github.com/FluidInference/FluidAudio/tree/main/Documentation
- **HuggingFace Models**: https://huggingface.co/FluidInference
- **Discord Community**: https://discord.gg/WNsvaCtmDe

## 📄 License

These code examples use the FluidAudio SDK (Apache 2.0 License).

## 🙏 Credits

- **FluidInference Team** - FluidAudio SDK
- **Silero Team** - VAD models
- **NVIDIA NeMo** - Parakeet TDT models

---

**Note**: This is a test implementation. For production you also need:

- Better error handling
- Support for more audio formats
- Real-time microphone capture
- Streaming output to file/API
- GPU acceleration options
