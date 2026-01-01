import FluidAudio
import Foundation
import AVFoundation

/// Simple batch transcription test for Parakeet TDT v2 model
@main
struct BatchTranscribeTest {
    static func main() async {
        print("=== FluidAudio Parakeet v2 Batch Transcription Test ===\n")
        
        // Get audio file path from command line or use default
        let args = CommandLine.arguments
        guard args.count > 1 else {
            print("Usage: swift run BatchTranscribeTest <audio-file-path>")
            print("Example: swift run BatchTranscribeTest ~/audio/meeting.wav")
            exit(1)
        }
        
        let audioPath = args[1]
        let audioURL = URL(fileURLWithPath: (audioPath as NSString).expandingTildeInPath)
        
        guard FileManager.default.fileExists(atPath: audioURL.path) else {
            print("❌ Error: File not found at \(audioURL.path)")
            exit(1)
        }
        
        print("📁 Audio file: \(audioURL.path)")
        
        do {
            // Initialize ASR with Parakeet v2 (English-only, highest recall)
            print("\n📥 Downloading and loading Parakeet v2 models...")
            let startLoad = Date()
            let models = try await AsrModels.downloadAndLoad(version: .v2)
            let loadTime = Date().timeIntervalSince(startLoad)
            print("✅ Models loaded in \(String(format: "%.2f", loadTime))s")
            
            // Create ASR manager
            let asrManager = AsrManager(config: .default)
            try await asrManager.initialize(models: models)
            print("✅ ASR Manager initialized")
            
            // Convert audio to 16kHz mono
            print("\n🎵 Converting audio to 16kHz mono...")
            let startConvert = Date()
            let samples = try AudioConverter().resampleAudioFile(audioURL)
            let convertTime = Date().timeIntervalSince(startConvert)
            let duration = Float(samples.count) / 16000.0
            print("✅ Audio converted in \(String(format: "%.2f", convertTime))s")
            print("   Duration: \(String(format: "%.2f", duration))s")
            print("   Samples: \(samples.count)")
            
            // Transcribe
            print("\n🎤 Transcribing...")
            let startTranscribe = Date()
            let result = try await asrManager.transcribe(samples)
            let transcribeTime = Date().timeIntervalSince(startTranscribe)
            
            // Calculate RTF (Real-Time Factor)
            let rtf = duration / Float(transcribeTime)
            
            // Display results
            print("\n" + String(repeating: "=", count: 60))
            print("TRANSCRIPTION RESULTS")
            print(String(repeating: "=", count: 60))
            print("\n📝 Text: \(result.text)")
            print("\n📊 Statistics:")
            print("   • Processing time: \(String(format: "%.3f", transcribeTime))s")
            print("   • Audio duration: \(String(format: "%.2f", duration))s")
            print("   • RTFx: \(String(format: "%.1f", rtf))x (faster than real-time)")
            print("   • Confidence: \(String(format: "%.2f%%", result.confidence * 100))")
            
            print("\n" + String(repeating: "=", count: 60))
            print("✅ Test completed successfully!")
            
        } catch {
            print("\n❌ Error: \(error)")
            exit(1)
        }
    }
}
