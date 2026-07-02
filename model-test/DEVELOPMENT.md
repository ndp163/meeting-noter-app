# Additional information and examples for developers

## 🧑‍💻 Development Guide

### Project Structure

```
model-test/
├── Package.swift              # Swift package definition
├── README.md                  # Main documentation (Vietnamese)
├── quick-test.sh             # Quick test script
├── .gitignore
├── test-audio/               # Place your test audio files here
│   └── README.md
└── Sources/
    ├── BatchTranscribeTest/
    │   └── main.swift        # Simple batch transcription
    ├── VadStreamTest/
    │   └── main.swift        # VAD streaming demo
    └── StreamingTranscribeTest/
        └── main.swift        # VAD + ASR streaming
```

### Adding Custom Tests

Create a new executable target in `Package.swift`:

```swift
.executableTarget(
    name: "MyCustomTest",
    dependencies: [
        .product(name: "FluidAudio", package: "FluidAudio")
    ],
    path: "Sources/MyCustomTest"
)
```

### Working with FluidAudio API

#### Basic Imports

```swift
import FluidAudio
import Foundation
import AVFoundation
```

#### Initialize VAD

```swift
let vadManager = try await VadManager(
    config: VadConfig(
        defaultThreshold: 0.5,        // Voice probability threshold
        computeUnits: .cpuAndNeuralEngine
    )
)
```

#### Initialize ASR

```swift
// Download and load models
let models = try await AsrModels.downloadAndLoad(version: .v2)

// Create manager
let asrManager = AsrManager(config: .default)
try await asrManager.initialize(models: models)
```

#### Convert Audio

```swift
let audioURL = URL(fileURLWithPath: "path/to/audio.wav")
let samples = try AudioConverter().resampleAudioFile(audioURL)
// Returns: [Float] at 16kHz mono
```

#### VAD Segmentation

```swift
var config = VadSegmentationConfig.default
config.minSpeechDuration = 0.5
config.minSilenceDuration = 0.3

let segments = try await vadManager.segmentSpeech(samples, config: config)
// Returns: [VadSegment] with startTime and endTime
```

#### Transcribe

```swift
let result = try await asrManager.transcribe(samples)
print(result.text)           // Full transcription
print(result.confidence)     // Overall confidence score
print(result.timings)        // Word-level timestamps
```

### Advanced: Real-time Microphone Capture

```swift
import AVFoundation

class MicrophoneCapture {
    let engine = AVAudioEngine()
    let vadManager: VadManager
    let asrManager: AsrManager

    func startCapture() {
        let inputNode = engine.inputNode
        let recordingFormat = inputNode.outputFormat(forBus: 0)

        inputNode.installTap(onBus: 0, bufferSize: 4096, format: recordingFormat) { buffer, time in
            // Convert to 16kHz mono
            let samples = self.convertBuffer(buffer)

            Task {
                // Process with VAD
                let vadResult = try await self.vadManager.processStreamingChunk(
                    samples,
                    state: self.vadState,
                    config: .default
                )

                if vadResult.event?.kind == .speechStart {
                    self.startRecording()
                } else if vadResult.event?.kind == .speechEnd {
                    let transcript = try await self.transcribeRecording()
                    print("Transcription: \(transcript)")
                }
            }
        }

        try engine.start()
    }
}
```

### Performance Tips

1. **Use Release builds for benchmarking:**

```bash
swift build -c release
swift run -c release BatchTranscribeTest audio.wav
```

2. **Ensure Apple Neural Engine is used:**

```swift
let config = AsrConfig(computeUnits: .cpuAndNeuralEngine)
```

3. **Batch processing for large files:**

```swift
// Process in chunks for very large files
let chunkSize = 16000 * 30  // 30 seconds
for chunk in samples.chunked(into: chunkSize) {
    let result = try await asrManager.transcribe(Array(chunk))
    print(result.text)
}
```

4. **Memory-mapped audio loading:**

```swift
// For very large files, use file-based API
let result = try await asrManager.transcribe(audioURL)
```

### Debugging

Enable debug logging:

```swift
let vadManager = try await VadManager(
    config: VadConfig(
        defaultThreshold: 0.5,
        debugMode: true  // Enable debug logs
    )
)
```

Check model loading:

```bash
# Models are cached here:
ls -la ~/.cache/fluidaudio/Models/
```

### Testing Different Audio Formats

FluidAudio's `AudioConverter` handles most formats automatically:

```swift
// All of these work:
let samples1 = try AudioConverter().resampleAudioFile(URL(fileURLWithPath: "audio.wav"))
let samples2 = try AudioConverter().resampleAudioFile(URL(fileURLWithPath: "audio.mp3"))
let samples3 = try AudioConverter().resampleAudioFile(URL(fileURLWithPath: "audio.m4a"))
let samples4 = try AudioConverter().resampleAudioFile(URL(fileURLWithPath: "audio.flac"))
```

### Common Patterns

#### Pattern 1: Offline Meeting Transcription

```swift
// 1. Load audio
let samples = try AudioConverter().resampleAudioFile(meetingURL)

// 2. Segment with VAD
let segments = try await vadManager.segmentSpeech(samples, config: .default)

// 3. Transcribe each segment
for segment in segments {
    let startSample = Int(segment.startTime * 16000)
    let endSample = Int(segment.endTime * 16000)
    let chunk = Array(samples[startSample..<endSample])

    let result = try await asrManager.transcribe(chunk)
    print("[\(segment.startTime)s] \(result.text)")
}
```

#### Pattern 2: Real-time Dictation

```swift
// Process streaming chunks
var vadState = await vadManager.makeStreamState()
var speechBuffer: [Float] = []

for chunk in audioChunks {
    let result = try await vadManager.processStreamingChunk(
        chunk,
        state: vadState,
        config: .default
    )
    vadState = result.state

    if result.isVoiceActive {
        speechBuffer.append(contentsOf: chunk)
    } else if !speechBuffer.isEmpty {
        // End of speech, transcribe
        let transcript = try await asrManager.transcribe(speechBuffer)
        print(transcript.text)
        speechBuffer = []
    }
}
```

#### Pattern 3: Speaker Diarization + Transcription

```swift
// Initialize both systems
let diarizerModels = try await DiarizerModels.downloadIfNeeded()
let diarizer = DiarizerManager()
diarizer.initialize(models: diarizerModels)

let asrModels = try await AsrModels.downloadAndLoad(version: .v2)
let asr = AsrManager(config: .default)
try await asr.initialize(models: asrModels)

// Process
let samples = try AudioConverter().resampleAudioFile(audioURL)
let diarResult = try diarizer.performCompleteDiarization(samples)

for segment in diarResult.segments {
    let startSample = Int(segment.startTimeSeconds * 16000)
    let endSample = Int(segment.endTimeSeconds * 16000)
    let chunk = Array(samples[startSample..<endSample])

    let transcript = try await asr.transcribe(chunk)
    print("[\(segment.speakerId)] \(transcript.text)")
}
```

### CI/CD Integration

```yaml
# .github/workflows/test.yml
name: Test FluidAudio

on: [push, pull_request]

jobs:
  test:
    runs-on: macos-13
    steps:
      - uses: actions/checkout@v3
      - name: Build
        run: swift build
      - name: Download test audio
        run: |
          mkdir -p test-audio
          # Download sample from LibriSpeech or similar
      - name: Run tests
        run: |
          swift run BatchTranscribeTest test-audio/sample.wav
```

## 📚 Additional Resources

### Documentation

- [FluidAudio ASR Guide](https://github.com/FluidInference/FluidAudio/blob/main/Documentation/ASR/GettingStarted.md)
- [VAD Documentation](https://github.com/FluidInference/FluidAudio/blob/main/Documentation/VAD/GettingStarted.md)
- [API Reference](https://github.com/FluidInference/FluidAudio/blob/main/Documentation/API.md)

### Models on HuggingFace

- [Parakeet v2](https://huggingface.co/FluidInference/parakeet-tdt-0.6b-v2-coreml)
- [Parakeet v3](https://huggingface.co/FluidInference/parakeet-tdt-0.6b-v3-coreml)
- [Silero VAD](https://huggingface.co/FluidInference/silero-vad-coreml)

### Example Apps

- [Spokenly](https://spokenly.app/) - Mac dictation app
- [Voice Ink](https://tryvoiceink.com/) - Local transcription
- [Slipbox](https://slipbox.ai/) - Meeting assistant

## 🤝 Contributing

Feel free to:

- Add more test scenarios
- Improve error handling
- Add support for other audio sources
- Create Python bindings (via subprocess)
- Add benchmarking tools

## 💡 Ideas for Extensions

1. **Web API**: Wrap these tests in a REST API
2. **Python Bridge**: Call Swift executables from Python
3. **GUI App**: Create a simple macOS app with SwiftUI
4. **Live Captions**: Real-time subtitles from microphone
5. **Batch Processing**: Process entire directories
6. **Quality Metrics**: WER/CER calculation with ground truth

---

**Happy Testing! 🚀**
