# FluidAudio Parakeet v2 Test Suite

Bộ test cho model **Parakeet TDT v2** từ [FluidInference/FluidAudio](https://github.com/FluidInference/FluidAudio) với hỗ trợ VAD (Voice Activity Detection) và streaming real-time.

## 📋 Tính năng

Repo này cung cấp 3 chương trình test:

1. **BatchTranscribeTest** - Batch transcription đơn giản
2. **VadStreamTest** - Real-time VAD streaming để phát hiện giọng nói
3. **StreamingTranscribeTest** - Streaming transcription với VAD-guided chunking

## 🔧 Yêu cầu hệ thống

- **macOS 13.0+** (FluidAudio chỉ hỗ trợ macOS/iOS)
- **Xcode 15.0+** hoặc Swift 5.9+
- **Apple Silicon (M1/M2/M3)** được khuyến nghị để tận dụng Apple Neural Engine

## 📦 Cài đặt

### 1. Clone hoặc download repository này

```bash
cd /Users/phuc.nguyendinh/personal/meeting-noter-app/model-test
```

### 2. Build project

```bash
swift build
```

Lần đầu tiên sẽ:

- Download FluidAudio SDK từ GitHub
- Download các model CoreML từ HuggingFace (parakeet-v2, silero-vad)
- Compile models

## 🚀 Sử dụng

### Test 1: Batch Transcription (Đơn giản nhất)

Transcribe toàn bộ file audio một lần:

```bash
swift run BatchTranscribeTest ~/audio/meeting.wav
```

**Output mẫu:**

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

### Test 2: VAD Streaming (Phát hiện giọng nói)

Simulate real-time streaming với Voice Activity Detection:

```bash
swift run VadStreamTest ~/audio/meeting.wav
```

**Output mẫu:**

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

### Test 3: Streaming Transcription với VAD (Real-time)

Kết hợp VAD và ASR để transcribe theo segments:

```bash
# Sử dụng model v2 (English-only, recall cao nhất)
swift run StreamingTranscribeTest ~/audio/meeting.wav --model-version v2

# Hoặc model v3 (Multilingual, 25 ngôn ngữ châu Âu)
swift run StreamingTranscribeTest ~/audio/meeting.wav --model-version v3
```

**Output mẫu:**

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

## 📝 Chi tiết về Models

### Parakeet TDT v2 (English-only)

- **Model**: `FluidInference/parakeet-tdt-0.6b-v2-coreml`
- **Ngôn ngữ**: Tiếng Anh
- **Ưu điểm**: Recall cao nhất cho tiếng Anh
- **Performance**: ~190x RTF trên M4 Pro
- **Kích thước**: ~600MB

### Parakeet TDT v3 (Multilingual)

- **Model**: `FluidInference/parakeet-tdt-0.6b-v3-coreml`
- **Ngôn ngữ**: 25 ngôn ngữ châu Âu
- **Performance**: ~190x RTF trên M4 Pro
- **Kích thước**: ~600MB

### Silero VAD

- **Model**: `FluidInference/silero-vad-coreml`
- **Tính năng**: Voice Activity Detection
- **Performance**: ~1200x RTF trên M2
- **Kích thước**: ~5MB

## 🎯 Use Cases

### 1. Meeting Transcription

```bash
# Transcribe cuộc họp với timestamps chi tiết
swift run StreamingTranscribeTest ~/Recordings/meeting.wav --model-version v2
```

### 2. Podcast Processing

```bash
# Batch transcription cho podcast
swift run BatchTranscribeTest ~/Podcasts/episode-123.mp3
```

### 3. Voice Activity Analysis

```bash
# Phân tích khi nào có người nói
swift run VadStreamTest ~/Audio/conversation.wav
```

### 4. Real-time Dictation

```bash
# Ghi âm từ microphone và transcribe real-time
# (cần code thêm để capture audio từ mic)
swift run StreamingTranscribeTest ~/Recordings/dictation.wav --model-version v2
```

## 🔧 Tuning Parameters

### VAD Configuration

Trong code có thể điều chỉnh các tham số VAD:

```swift
var segmentConfig = VadSegmentationConfig.default
segmentConfig.minSpeechDuration = 0.5  // Tối thiểu 0.5s mới coi là speech
segmentConfig.minSilenceDuration = 0.3  // 0.3s im lặng để tách segments
segmentConfig.speechPadding = 0.1      // Thêm 0.1s padding
segmentConfig.threshold = 0.5          // Ngưỡng probability (0-1)
```

**Recommendations:**

- **Meeting transcription**: minSpeechDuration=0.5, minSilenceDuration=0.3
- **Dictation**: minSpeechDuration=0.25, minSilenceDuration=0.5
- **Noisy environment**: threshold=0.6 hoặc 0.7

### ASR Configuration

```swift
let config = AsrConfig(
    computeUnits: .cpuAndNeuralEngine,  // Sử dụng ANE để tối ưu
    maxAudioLength: 30.0                // Max 30s per chunk
)
```

## 📊 Performance Benchmarks

### M4 Pro (2024)

- **Parakeet v2**: ~190x RTF (1 giờ audio → ~19 giây)
- **VAD**: ~1200x RTF
- **Combined (VAD + ASR)**: ~165x RTF

### M2 (2022)

- **Parakeet v2**: ~120x RTF (1 giờ audio → ~30 giây)
- **VAD**: ~1220x RTF
- **Combined (VAD + ASR)**: ~110x RTF

### Memory Usage

- **VAD**: ~50MB
- **Parakeet v2/v3**: ~1.2GB
- **Combined**: ~1.3GB

## 🐛 Troubleshooting

### Error: "Models not found"

```bash
# Models sẽ tự động download lần đầu chạy
# Nếu network bị block, set proxy:
export https_proxy=http://your-proxy:port
swift run BatchTranscribeTest audio.wav
```

### Error: "File not found"

```bash
# Kiểm tra đường dẫn file
ls -la ~/audio/meeting.wav

# Hoặc dùng đường dẫn tuyệt đối
swift run BatchTranscribeTest /Users/phuc/audio/meeting.wav
```

### Error: "Cannot load model"

```bash
# Xóa cache và download lại
rm -rf ~/.cache/fluidaudio/Models
swift run BatchTranscribeTest audio.wav
```

### Performance quá chậm

```bash
# Kiểm tra đang chạy trên ANE chưa
# Mở Activity Monitor > CPU > Tìm process
# Nếu CPU usage cao → không dùng ANE

# Solution: Đảm bảo model đã compile đúng
swift build -c release
swift run -c release BatchTranscribeTest audio.wav
```

## 🧠 Chi tiết Logic xử lý Real-time Transcription

### Overview

Hệ thống sử dụng **VAD (Voice Activity Detection)** để phát hiện voice → **ASR (Automatic Speech Recognition)** để transcribe, với 2 chế độ:

- **Streaming**: Transcribe liên tục trong khi nói (real-time feedback)
- **Final**: Transcribe toàn bộ sau khi kết thúc nói (chính xác nhất)

### State Machine

**States:**

1. **IDLE**: `isSpeaking=false` - Đang chờ, không nói
2. **SPEAKING**: `isSpeaking=true` - Đang trong speech session
3. **SILENCE_IN_SPEECH**: `isSpeaking=true` + `silenceFrameCount>0` - Im lặng tạm thời

**Transitions:**

```
IDLE
  → [detect voice prob>0.25]
  → SPEAKING

SPEAKING
  → [strong voice prob>0.35 + buffer≥0.25s]
  → Launch Streaming Transcription (không clear buffer)
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
- probability: 0.0-1.0 (xác suất có voice)
- state: Streaming state for context
```

**Thresholds:**

- `probability > 0.35` → Strong voice ✅ Trigger streaming transcription
- `probability > 0.25` → Weak voice 🟡 Buffer only (không transcribe)
- `probability ≤ 0.25` → No voice ⚪️ Silence

### Buffer Management

**Thread-Safety:**

```swift
bufferLock: NSLock
- Lock mọi thao tác append/copy/clear
- Tránh race condition giữa audio thread và transcribe tasks
```

**Buffer Operations:**

| Scenario          | Action                          | Clear Buffer?               | Lock |
| ----------------- | ------------------------------- | --------------------------- | ---- |
| Speech start      | `speechBuffer = []`             | ✅                          | ✅   |
| Voice detected    | `append(samples)`               | ❌                          | ✅   |
| Silence in speech | `append(samples)` (if count≤16) | ❌                          | ✅   |
| Streaming trigger | `copy()` buffer                 | ❌ (giữ để transcribe tiếp) | ✅   |
| Final trigger     | `copy()` + `clear()`            | ✅ (chuẩn bị segment mới)   | ✅   |

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
2. 📋 Copy buffer (toàn bộ audio từ đầu đến giờ)
3. 🔓 Unlock (KHÔNG clear - tiếp tục tích lũy)
4. 🎯 Enqueue to serial `transcriptionQueue`
5. 🔄 ASR transcribe async
6. 📝 Update UI với partial result

**Characteristics:**

- **Frequency**: Mỗi khi có strong voice + đủ buffer
- **Latency**: ~50-100ms
- **Purpose**: Real-time feedback cho user
- **Accuracy**: Tốt nhưng có thể thay đổi khi thêm context

### Final Transcription

**Trigger Conditions:**

```swift
if isSpeaking
   && !hasVoice
   && silenceFrameCount > 16 (~4 seconds)
```

**Process:**

1. 🔒 Lock buffer
2. 📋 Copy toàn bộ buffer (complete speech segment)
3. 🗑️ Clear buffer ngay lập tức
4. 🔓 Unlock
5. 🎯 Enqueue to serial `transcriptionQueue`
6. 🔄 ASR transcribe async
7. 📊 Show final result + stats
8. 🔄 Reset all state (VAD state, counters, flags)

**Characteristics:**

- **Frequency**: 1 lần khi kết thúc speech
- **Latency**: ~60-80ms
- **Purpose**: Kết quả chính xác nhất
- **Accuracy**: Cao nhất (full context từ đầu đến cuối)

### Silence Detection

**Counter Logic:**

```swift
silenceFrameCount = 0                    // Init
hasVoice → silenceFrameCount = 0         // Reset khi có voice
!hasVoice + isSpeaking → silenceFrameCount++ // Đếm silence trong speech
```

**Timeout:**

```
silenceThreshold = 16 chunks
@ 48kHz native: 16 × ~85ms ≈ 1.4s (nhưng VAD output slower)
Effective timeout: ~4 seconds
```

**During Silence:**

- **Count ≤ 16**: Continue buffering (cho phép pause trong câu)
- **Count > 16**: Trigger Final + reset toàn bộ state

### Concurrency Control

**1. Buffer Protection:**

```swift
bufferLock: NSLock
- Serialize tất cả buffer access
- Tránh race condition
```

**2. Transcription Queue:**

```swift
transcriptionQueue: Serial DispatchQueue
- QoS: .userInitiated
- Process transcriptions tuần tự
- Tránh model contention
```

**3. Active Transcription Limit:**

```swift
maxConcurrentTranscriptions = 3
activeTranscriptions counter
- Ngăn overload
- Graceful degradation
```

### State Reset (After Final)

```swift
isSpeaking = false                    // Exit speech mode
silenceFrameCount = 0                 // Reset silence counter
currentTranscript = ""                // Clear display
speechBuffer = []                     // Clear buffer (đã clear trước đó)
skippedShortAudio = 0                 // Reset debug counters
skippedLowVAD = 0
vadState = makeStreamState()          // Fresh VAD state
```

**Purpose:** Bắt đầu hoàn toàn sạch cho speech segment tiếp theo, tránh state cũ ảnh hưởng.

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

1. **Streaming KHÔNG clear buffer**: Để có full context cho lần transcribe tiếp theo, tăng accuracy
2. **Final clear buffer ngay**: Tránh reuse stale data cho segment mới
3. **Serial queue**: Tránh model conflict, đảm bảo order
4. **VAD state reset**: Tránh state cũ ảnh hưởng segment mới
5. **Lock protection**: Thread-safe cho buffer shared giữa audio thread và transcribe tasks
6. **4s silence threshold**: Balance giữa tự nhiên (cho phép pause) và responsive

### Critical Bug Fixes

**Vấn đề ban đầu:** Buffer 0.9s (14400 samples) gây lỗi "Invalid audio data" sau Final transcription

**Nguyên nhân:**

1. Buffer không được protect bằng lock → race condition
2. VAD state không reset sau Final → state corruption
3. Counters không reset → logic sai ở segment tiếp theo

**Giải pháp đã implement:**

1. ✅ Thêm NSLock cho tất cả buffer operations
2. ✅ Reset VAD state sau mỗi Final
3. ✅ Reset tất cả counters (skippedShortAudio, skippedLowVAD)
4. ✅ Serial queue thay vì Task.detached
5. ✅ Clear buffer ngay sau copy trong Final mode

## 🔗 Tài liệu tham khảo

- **FluidAudio GitHub**: https://github.com/FluidInference/FluidAudio
- **Documentation**: https://github.com/FluidInference/FluidAudio/tree/main/Documentation
- **HuggingFace Models**: https://huggingface.co/FluidInference
- **Discord Community**: https://discord.gg/WNsvaCtmDe

## 📄 License

Code examples này sử dụng FluidAudio SDK (Apache 2.0 License).

## 🙏 Credits

- **FluidInference Team** - FluidAudio SDK
- **Silero Team** - VAD models
- **NVIDIA NeMo** - Parakeet TDT models

---

**Note**: Đây là test implementation. Để production cần thêm:

- Error handling tốt hơn
- Support cho nhiều audio formats
- Real-time microphone capture
- Streaming output to file/API
- GPU acceleration options
