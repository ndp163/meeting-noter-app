import FluidAudio
import Foundation
import AVFoundation

/// Ultra-fast real-time microphone transcription
@main
struct RealTimeMicTest {
    static func main() async {
        print("╔════════════════════════════════════════════════════════════╗")
        print("║  🎙️  Ultra-Fast Real-Time Transcription (Microphone)    ║")
        print("╚════════════════════════════════════════════════════════════╝")
        print()
        print("📝 Instructions:")
        print("   • Speak into your microphone")
        print("   • Text updates every ~1 second (real-time!)")
        print("   • Final result shown after you stop speaking")
        print("   • Press Ctrl+C to exit")
        print()
        
        do {
            let transcriber = try await LiveTranscriber()
            try await transcriber.start()
        } catch {
            print("\n❌ Error: \(error)")
            exit(1)
        }
    }
}

/// Live transcriber with ultra-fast streaming
class LiveTranscriber {
    private let audioEngine = AVAudioEngine()
    private let vadManager: VadManager
    private let asrManager: AsrManager
    
    private var vadState: VadStreamState?
    private var speechBuffer: [Float] = []
    private var isSpeaking = false
    private var silenceFrameCount = 0
    private let silenceThreshold = 16 // ~4 seconds
    
    private var totalTranscriptions = 0
    private var skippedShortAudio = 0
    private var skippedLowVAD = 0
    
    // Thread-safety: Lock để protect speechBuffer khỏi race condition
    private let bufferLock = NSLock()
    // Serial queue để xử lý transcription tuần tự, tránh concurrent issues
    private let transcriptionQueue = DispatchQueue(label: "com.fluidaudio.transcription", qos: .userInitiated)
    
    // Optimized streaming settings based on FluidAudio docs
    // Reference: centerSeconds: 11.2s (140 frames), leftContext: 1.6s (20 frames), rightContext: 1.6s (20 frames)
    // Total: 14.4s (within 15s hard limit)
    private let streamingChunkSize = 16000 / 2 // 0.5s chunks for real-time feel
    private let minChunkSize = 16000 / 4 // 0.25s minimum to avoid too frequent calls
    private var currentTranscript = ""
    private var activeTranscriptions = 0
    private let maxConcurrentTranscriptions = 3 // Cho phép tối đa 3 transcriptions đồng thời
    
    init() async throws {
        print("🔧 Initializing systems...")
        
        // Initialize VAD
        print("   ⏳ Loading VAD model...")
        self.vadManager = try await VadManager(
            config: VadConfig(defaultThreshold: 0.3) // Giảm xuống 0.3 để capture tốt hơn
        )
        print("   ✅ VAD ready (threshold: 0.3)")
        
        // Initialize ASR with optimized config
        print("   ⏳ Loading Parakeet v2 model...")
        let models = try await AsrModels.downloadAndLoad(version: .v2)
        
        // Use default config which is optimized for the model
        // Config already optimized with centerSeconds: 11.2, leftContext: 1.6, rightContext: 1.6
        self.asrManager = AsrManager(config: .default)
        try await asrManager.initialize(models: models)
        print("   ✅ ASR ready (11.2s center + 3.2s context = 14.4s chunks)")
        
        print("\n✅ All systems initialized!")
    }
    
    func start() async throws {
        // Request microphone permission
        let status = AVCaptureDevice.authorizationStatus(for: .audio)
        if status != .authorized {
            print("\n⚠️  Requesting microphone permission...")
            let granted = await AVCaptureDevice.requestAccess(for: .audio)
            if !granted {
                print("❌ Microphone access denied!")
                throw NSError(domain: "MicrophoneAccess", code: 1)
            }
        }
        
        // Initialize VAD state
        vadState = await vadManager.makeStreamState()
        
        // Setup audio input
        let inputNode = audioEngine.inputNode
        let recordingFormat = inputNode.outputFormat(forBus: 0)
        
        print("\n🎤 Microphone Info:")
        print("   • Sample Rate: \(recordingFormat.sampleRate) Hz")
        print("   • Channels: \(recordingFormat.channelCount)")
        print("   • Chunk size: 0.5s (\(streamingChunkSize) samples)")
        print("   • Min chunk: 0.25s (\(minChunkSize) samples)")
        // We need 16kHz mono for VAD/ASR
        let targetFormat = AVAudioFormat(
            commonFormat: .pcmFormatFloat32,
            sampleRate: 16000,
            channels: 1,
            interleaved: false
        )!
        
        guard let converter = AVAudioConverter(from: recordingFormat, to: targetFormat) else {
            throw NSError(domain: "AudioConverter", code: 2)
        }
        
        print("\n🔴 LISTENING... (speak now)")
        print(String(repeating: "─", count: 60))
        print("\n💡 Tip: Text updates every 1 second - speak naturally")
        print("💡 Longer sentences give better accuracy")
        print()
        
        // Install tap to capture audio
        let bufferSize: AVAudioFrameCount = 4096
        inputNode.installTap(onBus: 0, bufferSize: bufferSize, format: recordingFormat) { [weak self] buffer, time in
            guard let self = self else { return }
            
            // Convert to 16kHz mono
            guard let convertedBuffer = self.convertAudioBuffer(
                buffer,
                converter: converter,
                targetFormat: targetFormat
            ) else {
                return
            }
            
            // Extract float samples
            let samples = self.extractSamples(from: convertedBuffer)
            
            // Process with VAD and ASR
            Task {
                await self.processAudioChunk(samples)
            }
        }
        
        // Start audio engine
        audioEngine.prepare()
        try audioEngine.start()
        
        // Setup signal handler for Ctrl+C
        var shouldStop = false
        signal(SIGINT) { _ in
            print("\n\n👋 Stopping...")
            exit(0)
        }
        
        // Keep running until interrupted - use async-friendly approach
        await withCheckedContinuation { (continuation: CheckedContinuation<Void, Never>) in
            // Never resume - run forever until SIGINT
        }
    }
    
    private func processAudioChunk(_ samples: [Float]) async {
        guard let state = vadState else { return }
        
        do {
            // Run VAD on this chunk
            let result = try await vadManager.processStreamingChunk(
                samples,
                state: state,
                config: .default,
                returnSeconds: true,
                timeResolution: 2
            )
            
            vadState = result.state
            
            let probability = result.probability
            let isStrongVoice = probability > 0.35 // Strong voice - trigger transcription
            let isWeakVoice = probability > 0.25 && probability <= 0.35 // Weak voice - keep in buffer
            let hasVoice = probability > 0.25 // Any voice detection
            
            // Visual indicator
            let indicator: String
            if isStrongVoice {
                indicator = "🟢" // Green - strong
            } else if isWeakVoice {
                indicator = "🟡" // Yellow - weak
            } else {
                indicator = "⚪️" // White - no voice
            }
            
            let probBar = String(repeating: "█", count: Int(probability * 20))
            let emptyBar = String(repeating: "░", count: 20 - Int(probability * 20))
            
            // Log with voice type indicator
            if !isSpeaking || currentTranscript.isEmpty {
                let voiceType = isStrongVoice ? "strong" : (isWeakVoice ? "weak" : "")
                let suffix = voiceType.isEmpty ? "" : " (\(voiceType))"
                print("\r\(indicator) [\(probBar)\(emptyBar)] \(String(format: "%.3f", probability))\(suffix)", terminator: "")
                fflush(stdout)
            }
            
            // Speech detection logic - giữ weak voice trong buffer
            if hasVoice {
                // Reset silence counter for any voice (weak or strong)
                silenceFrameCount = 0
                
                if !isSpeaking {
                    // Start of speech (weak or strong)
                    isSpeaking = true
                    bufferLock.lock()
                    speechBuffer = []
                    bufferLock.unlock()
                    currentTranscript = ""
                    skippedShortAudio = 0
                    skippedLowVAD = 0
                    let timestamp = Date()
                    let voiceType = isStrongVoice ? "strong" : "weak"
                    print("\n\n🎙️  Speech started (\(voiceType) voice, prob: \(String(format: "%.3f", probability))) at \(String(format: "%.3f", timestamp.timeIntervalSince1970.truncatingRemainder(dividingBy: 1000)))")
                    print("─────────────────────────────────────────────────────────")
                }
                
                // Always buffer voice audio (weak or strong) - thread-safe
                bufferLock.lock()
                speechBuffer.append(contentsOf: samples)
                bufferLock.unlock()
                
                // Limit buffer size to avoid memory issues (max 15s as per model limit)
                let maxBufferSize = 16000 * 15 // 15 seconds max
                bufferLock.lock()
                let currentBufferSize = speechBuffer.count
                if currentBufferSize > maxBufferSize {
                    print("\n⚠️  Buffer overflow - trimming to 15s")
                    speechBuffer = Array(speechBuffer.suffix(maxBufferSize))
                }
                bufferLock.unlock()
                
                // Chỉ trigger transcription khi có strong voice và đủ chunk size
                if isStrongVoice && currentBufferSize >= minChunkSize && activeTranscriptions < maxConcurrentTranscriptions {
                    // CRITICAL: Copy buffer NHƯNG KHÔNG clear (streaming mode cần giữ buffer)
                    bufferLock.lock()
                    let bufferToTranscribe = Array(speechBuffer)
                    bufferLock.unlock()
                    
                    let bufferDuration = Float(bufferToTranscribe.count) / 16000.0
                    print("\n🚀 Launching transcription (buffer: \(String(format: "%.2f", bufferDuration))s, active: \(activeTranscriptions))")
                    
                    // Transcribe on serial queue để tránh concurrent issues
                    transcriptionQueue.async { [weak self] in
                        Task {
                            await self?.transcribeStreaming(bufferToTranscribe)
                        }
                    }
                } else if isWeakVoice {
                    // Weak voice - keep buffering but don't transcribe yet
                    // Nó sẽ được nối với audio sau
                }
            } else {
                if isSpeaking {
                    silenceFrameCount += 1
                    
                    // Continue buffering during short silence
                    if silenceFrameCount <= silenceThreshold {
                        bufferLock.lock()
                        speechBuffer.append(contentsOf: samples)
                        bufferLock.unlock()
                    } else {
                        // End of speech - transcribe final chunk
                        bufferLock.lock()
                        let bufferToTranscribe = Array(speechBuffer)
                        speechBuffer = [] // Clear ngay sau copy
                        bufferLock.unlock()
                        
                        if !bufferToTranscribe.isEmpty {
                            transcriptionQueue.async { [weak self] in
                                Task {
                                    await self?.transcribeFinal(bufferToTranscribe)
                                }
                            }
                        } else {
                            print("\n⚠️  Speech ended but buffer empty")
                        }
                        
                        isSpeaking = false
                        silenceFrameCount = 0
                        currentTranscript = ""
                        skippedShortAudio = 0
                        skippedLowVAD = 0
                        
                        // Reset VAD state để bắt đầu sạch cho segment tiếp theo
                        Task {
                            vadState = await vadManager.makeStreamState()
                        }
                        
                        // Log statistics if any audio was skipped
                        if skippedShortAudio > 0 {
                            print("\n📊 Debug: Skipped \(skippedShortAudio) short audio chunks")
                        }
                    }
                }
            }
            
        } catch {
            print("\n⚠️  VAD error: \(error)")
        }
    }
    
    private func transcribeStreaming(_ samples: [Float]) async {
        activeTranscriptions += 1
        defer { activeTranscriptions -= 1 }
        
        // Validate audio data
        guard !samples.isEmpty else {
            print("\n⚠️  Empty samples in streaming")
            return
        }
        
        let duration = Float(samples.count) / 16000.0
        
        // Skip very short buffers - giảm xuống 0.25s
        guard duration > 0.25 else { 
            skippedShortAudio += 1
            if skippedShortAudio <= 3 {
                print("\n⚠️  Skipped short audio: \(String(format: "%.2f", duration))s")
            }
            return 
        }
        
        // Validate audio is not corrupted (check for all zeros or NaN)
        let hasValidData = samples.contains { $0 != 0 && !$0.isNaN && !$0.isInfinite }
        guard hasValidData else {
            print("\n⚠️  Invalid audio data in streaming (all zeros or NaN, \(samples.count) samples)")
            return
        }
        
        do {
            let startTime = Date()
            let result = try await asrManager.transcribe(samples)
            let processingTime = Date().timeIntervalSince(startTime)
            
            if !result.text.isEmpty {
                // Clear previous line and print new transcript with timing
                let latency = Int(processingTime * 1000)
                let rtfx = duration / Float(processingTime)
                let concurrent = activeTranscriptions > 1 ? " [\(activeTranscriptions) tasks]" : ""
                print("\r\u{001B}[K📝 \(result.text) \u{001B}[90m[\(latency)ms, \(String(format: "%.0f", rtfx))x\(concurrent)]\u{001B}[0m", terminator: "")
                fflush(stdout)
                currentTranscript = result.text
            }
        } catch {
            // Log errors during streaming for debugging
            let errorMsg = "\(error)"
            if errorMsg.contains("invalidAudioData") {
                print("\n⚠️  Invalid audio data (\(samples.count) samples, \(String(format: "%.2f", duration))s)")
            } else {
                print("\n⚠️  Streaming transcribe error: \(error)")
            }
        }
    }
    
    private func transcribeFinal(_ samples: [Float]) async {
        activeTranscriptions += 1
        defer { activeTranscriptions -= 1 }
        
        // Validate audio data
        guard !samples.isEmpty else {
            print("\n⚠️  Empty samples in final transcription")
            return
        }
        
        let duration = Float(samples.count) / 16000.0
        
        // Skip very short buffers - giảm từ 0.3 xuống 0.2
        guard duration > 0.2 else {
            print("\n🔇 Speech ended (too short: \(String(format: "%.2f", duration))s)")
            print(String(repeating: "─", count: 60))
            print("🎤 Listening... (speak again or Ctrl+C to exit)\n")
            return
        }
        
        // Validate audio is not corrupted
        let hasValidData = samples.contains { $0 != 0 && !$0.isNaN && !$0.isInfinite }
        guard hasValidData else {
            print("\n⚠️  Invalid audio data (all zeros or NaN)")
            print(String(repeating: "─", count: 60))
            print("🎤 Listening... (speak again or Ctrl+C to exit)\n")
            return
        }
        
        do {
            let startTranscribe = Date()
            let result = try await asrManager.transcribe(samples)
            let transcribeTime = Date().timeIntervalSince(startTranscribe)
            
            totalTranscriptions += 1
            
            print("\n")
            print("🔇 Speech ended")
            print(String(repeating: "═", count: 60))
            
            if !result.text.isEmpty {
                print("📝 Final: \"\(result.text)\"")
                print()
                print("📊 Stats:")
                print("   • Duration: \(String(format: "%.2f", duration))s")
                print("   • Processing: \(String(format: "%.3f", transcribeTime))s")
                print("   • RTFx: \(String(format: "%.1f", duration / Float(transcribeTime)))x (target: ~120x on M4)")
                print("   • Latency: \(Int(transcribeTime * 1000))ms")
                print("   • Confidence: \(String(format: "%.1f", result.confidence * 100))%")
                print("   • Count: #\(totalTranscriptions)")
            } else {
                print("⚠️  No transcription (possibly unclear speech)")
                print("   • Duration: \(String(format: "%.2f", duration))s")
            }
            
            print(String(repeating: "─", count: 60))
            print("🎤 Listening... (speak again or Ctrl+C to exit)\n")
            
        } catch {
            let errorMsg = "\(error)"
            if errorMsg.contains("invalidAudioData") {
                print("\n❌ Invalid audio data detected")
                print("   • Sample count: \(samples.count)")
                print("   • Duration: \(String(format: "%.2f", duration))s")
                print("   • Has non-zero: \(samples.contains { $0 != 0 })")
            } else {
                print("\n❌ Transcription error: \(error)")
            }
            print(String(repeating: "─", count: 60))
            print("🎤 Listening... (speak again or Ctrl+C to exit)\n")
        }
    }
    
    private func convertAudioBuffer(
        _ buffer: AVAudioPCMBuffer,
        converter: AVAudioConverter,
        targetFormat: AVAudioFormat
    ) -> AVAudioPCMBuffer? {
        let capacity = AVAudioFrameCount(Double(buffer.frameLength) * targetFormat.sampleRate / buffer.format.sampleRate)
        
        guard let convertedBuffer = AVAudioPCMBuffer(
            pcmFormat: targetFormat,
            frameCapacity: capacity
        ) else {
            return nil
        }
        
        var error: NSError?
        let inputBlock: AVAudioConverterInputBlock = { inNumPackets, outStatus in
            outStatus.pointee = .haveData
            return buffer
        }
        
        converter.convert(to: convertedBuffer, error: &error, withInputFrom: inputBlock)
        
        if let error = error {
            print("⚠️  Conversion error: \(error)")
            return nil
        }
        
        return convertedBuffer
    }
    
    private func extractSamples(from buffer: AVAudioPCMBuffer) -> [Float] {
        guard let channelData = buffer.floatChannelData else {
            return []
        }
        
        let channelDataPointer = channelData[0]
        let frameLength = Int(buffer.frameLength)
        
        return Array(UnsafeBufferPointer(start: channelDataPointer, count: frameLength))
    }
    
    deinit {
        audioEngine.stop()
        audioEngine.inputNode.removeTap(onBus: 0)
    }
}
