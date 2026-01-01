import FluidAudio
import Foundation
import AVFoundation

/// Real-time streaming transcription with VAD-guided chunking
@main
struct StreamingTranscribeTest {
    static func main() async {
        print("=== FluidAudio Real-time Streaming Transcription Test ===")
        print("    (with VAD-guided speech detection)\n")
        
        // Get audio file path from command line
        let args = CommandLine.arguments
        guard args.count > 1 else {
            print("Usage: swift run StreamingTranscribeTest <audio-file-path> [--model-version v2|v3]")
            print("Example: swift run StreamingTranscribeTest ~/audio/meeting.wav --model-version v2")
            print("\nThis simulates real-time streaming with VAD + ASR")
            exit(1)
        }
        
        let audioPath = args[1]
        let audioURL = URL(fileURLWithPath: (audioPath as NSString).expandingTildeInPath)
        
        // Parse model version
        var modelVersion: AsrModelVersion = .v2
        if args.count > 3 && args[2] == "--model-version" {
            modelVersion = args[3].lowercased() == "v3" ? .v3 : .v2
        }
        
        guard FileManager.default.fileExists(atPath: audioURL.path) else {
            print("❌ Error: File not found at \(audioURL.path)")
            exit(1)
        }
        
        print("📁 Audio file: \(audioURL.path)")
        print("🤖 Model version: \(modelVersion == .v2 ? "v2 (English-only)" : "v3 (Multilingual)")")
        
        do {
            // Initialize VAD
            print("\n📥 Step 1/3: Initializing VAD...")
            let vadManager = try await VadManager(
                config: VadConfig(defaultThreshold: 0.5)
            )
            print("✅ VAD ready")
            
            // Initialize ASR
            print("\n📥 Step 2/3: Loading Parakeet models...")
            let startLoad = Date()
            let models = try await AsrModels.downloadAndLoad(version: modelVersion)
            let loadTime = Date().timeIntervalSince(startLoad)
            print("✅ Models loaded in \(String(format: "%.2f", loadTime))s")
            
            let asrManager = AsrManager(config: .default)
            try await asrManager.initialize(models: models)
            print("✅ ASR ready")
            
            // Load audio
            print("\n📥 Step 3/3: Loading audio...")
            let samples = try AudioConverter().resampleAudioFile(audioURL)
            let duration = Float(samples.count) / 16000.0
            print("✅ Audio loaded: \(String(format: "%.2f", duration))s")
            
            // Run VAD segmentation first
            print("\n🔍 Running VAD to detect speech segments...")
            var segmentConfig = VadSegmentationConfig.default
            segmentConfig.minSpeechDuration = 0.5  // At least 0.5s of speech
            segmentConfig.minSilenceDuration = 0.3  // 0.3s silence to split
            segmentConfig.speechPadding = 0.1  // Add 0.1s padding
            
            let vadStartTime = Date()
            let speechSegments = try await vadManager.segmentSpeech(samples, config: segmentConfig)
            let vadTime = Date().timeIntervalSince(vadStartTime)
            
            print("✅ VAD detected \(speechSegments.count) speech segments in \(String(format: "%.2f", vadTime))s")
            
            // Display segments
            print("\n📊 Speech Segments:")
            print(String(repeating: "-", count: 60))
            for (index, segment) in speechSegments.enumerated() {
                let segmentDuration = segment.endTime - segment.startTime
                print("   Segment \(index + 1): \(String(format: "%.2f", segment.startTime))s → \(String(format: "%.2f", segment.endTime))s (duration: \(String(format: "%.2f", segmentDuration))s)")
            }
            
            // Transcribe each speech segment
            print("\n🎤 Transcribing speech segments...")
            print(String(repeating: "=", count: 60))
            
            var allTranscriptions: [(timeRange: String, text: String)] = []
            let asrStartTime = Date()
            
            for (index, segment) in speechSegments.enumerated() {
                let startSample = Int(segment.startTime * 16000)
                let endSample = Int(segment.endTime * 16000)
                let segmentSamples = Array(samples[startSample..<min(endSample, samples.count)])
                
                print("\n[\(index + 1)/\(speechSegments.count)] Processing segment \(String(format: "%.2f", segment.startTime))s - \(String(format: "%.2f", segment.endTime))s...")
                
                let result = try await asrManager.transcribe(segmentSamples)
                
                if !result.text.isEmpty {
                    let timeRange = "\(String(format: "%.2f", segment.startTime))s - \(String(format: "%.2f", segment.endTime))s"
                    allTranscriptions.append((timeRange: timeRange, text: result.text))
                    
                    print("   📝 \"\(result.text)\"")
                    print("   ✓ Confidence: \(String(format: "%.2f%%", result.confidence * 100))")
                } else {
                    print("   ⚠️  No transcription (empty result)")
                }
            }
            
            let asrTime = Date().timeIntervalSince(asrStartTime)
            let totalProcessingTime = vadTime + asrTime
            let rtf = duration / Float(totalProcessingTime)
            
            // Final summary
            print("\n" + String(repeating: "=", count: 60))
            print("FULL TRANSCRIPTION")
            print(String(repeating: "=", count: 60) + "\n")
            
            for (index, transcription) in allTranscriptions.enumerated() {
                print("[\(transcription.timeRange)]")
                print("\(transcription.text)")
                if index < allTranscriptions.count - 1 {
                    print()
                }
            }
            
            print("\n" + String(repeating: "=", count: 60))
            print("PERFORMANCE SUMMARY")
            print(String(repeating: "=", count: 60))
            print("\n📊 Statistics:")
            print("   • Audio duration: \(String(format: "%.2f", duration))s")
            print("   • VAD processing: \(String(format: "%.3f", vadTime))s")
            print("   • ASR processing: \(String(format: "%.3f", asrTime))s")
            print("   • Total processing: \(String(format: "%.3f", totalProcessingTime))s")
            print("   • RTFx: \(String(format: "%.1f", rtf))x faster than real-time")
            print("   • Speech segments: \(speechSegments.count)")
            print("   • Successful transcriptions: \(allTranscriptions.count)")
            
            print("\n" + String(repeating: "=", count: 60))
            print("✅ Test completed successfully!")
            
        } catch {
            print("\n❌ Error: \(error)")
            if let localizedError = error as? LocalizedError {
                if let description = localizedError.errorDescription {
                    print("   Description: \(description)")
                }
                if let failureReason = localizedError.failureReason {
                    print("   Reason: \(failureReason)")
                }
            }
            exit(1)
        }
    }
}
