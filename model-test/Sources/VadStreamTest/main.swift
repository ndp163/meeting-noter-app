import FluidAudio
import Foundation
import AVFoundation

/// Real-time VAD streaming test with speech detection
@main
struct VadStreamTest {
    static func main() async {
        print("=== FluidAudio VAD Real-time Streaming Test ===\n")
        
        // Get audio file path from command line or use default
        let args = CommandLine.arguments
        guard args.count > 1 else {
            print("Usage: swift run VadStreamTest <audio-file-path>")
            print("Example: swift run VadStreamTest ~/audio/meeting.wav")
            print("\nThis simulates real-time streaming by processing audio in chunks")
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
            // Initialize VAD
            print("\n📥 Initializing VAD...")
            let vadManager = try await VadManager(
                config: VadConfig(defaultThreshold: 0.5)
            )
            print("✅ VAD initialized")
            
            // Load and convert audio
            print("\n🎵 Loading audio...")
            let samples = try AudioConverter().resampleAudioFile(audioURL)
            let duration = Float(samples.count) / Float(VadManager.sampleRate)
            print("✅ Audio loaded: \(String(format: "%.2f", duration))s, \(samples.count) samples")
            
            // Simulate real-time streaming
            print("\n🔴 Starting real-time VAD simulation...")
            print("   Chunk size: \(VadManager.chunkSize) samples (~256ms)")
            print(String(repeating: "-", count: 60))
            
            var state = await vadManager.makeStreamState()
            let chunkSize = VadManager.chunkSize
            var chunkIndex = 0
            var speechSegments: [(start: Float, end: Float)] = []
            var currentSpeechStart: Float? = nil
            
            let startTime = Date()
            
            // Process in chunks
            for startSample in stride(from: 0, to: samples.count, by: chunkSize) {
                let endSample = min(startSample + chunkSize, samples.count)
                var chunk = Array(samples[startSample..<endSample])
                
                // Pad last chunk if needed
                if chunk.count < chunkSize {
                    chunk.append(contentsOf: Array(repeating: Float(0), count: chunkSize - chunk.count))
                }
                
                let result = try await vadManager.processStreamingChunk(
                    chunk,
                    state: state,
                    config: .default,
                    returnSeconds: true,
                    timeResolution: 2
                )
                
                state = result.state
                
                let currentTime = Float(startSample) / Float(VadManager.sampleRate)
                let probability = result.probability
                
                // Visual representation
                let probBar = String(repeating: "█", count: Int(probability * 20))
                let emptyBar = String(repeating: "░", count: 20 - Int(probability * 20))
                let statusIcon = probability > 0.5 ? "🟢" : "⚪️"
                
                print("[\(String(format: "%6.2f", currentTime))s] \(statusIcon) [\(probBar)\(emptyBar)] \(String(format: "%.3f", probability))", terminator: "")
                
                // Handle events
                if let event = result.event {
                    switch event.kind {
                    case .speechStart:
                        if let time = event.time {
                            currentSpeechStart = Float(time)
                        } else {
                            currentSpeechStart = currentTime
                        }
                        print(" 🎙️  SPEECH START", terminator: "")
                    case .speechEnd:
                        if let start = currentSpeechStart {
                            let end: Float
                            if let time = event.time {
                                end = Float(time)
                            } else {
                                end = currentTime
                            }
                            speechSegments.append((start: start, end: end))
                            print(" 🔇 SPEECH END (duration: \(String(format: "%.2f", end - start))s)", terminator: "")
                        }
                        currentSpeechStart = nil
                    }
                }
                
                print() // New line
                
                chunkIndex += 1
                
                // Optional: simulate real-time delay (comment out for fast processing)
                // try await Task.sleep(nanoseconds: UInt64(256 * 1_000_000)) // 256ms
            }
            
            let processingTime = Date().timeIntervalSince(startTime)
            let rtf = duration / Float(processingTime)
            
            // Summary
            print("\n" + String(repeating: "=", count: 60))
            print("VAD ANALYSIS SUMMARY")
            print(String(repeating: "=", count: 60))
            print("\n📊 Statistics:")
            print("   • Audio duration: \(String(format: "%.2f", duration))s")
            print("   • Processing time: \(String(format: "%.3f", processingTime))s")
            print("   • RTFx: \(String(format: "%.1f", rtf))x faster than real-time")
            print("   • Chunks processed: \(chunkIndex)")
            
            print("\n🎙️  Speech Segments Detected: \(speechSegments.count)")
            if !speechSegments.isEmpty {
                var totalSpeechTime: Float = 0
                for (index, segment) in speechSegments.enumerated() {
                    let segmentDuration = segment.end - segment.start
                    totalSpeechTime += segmentDuration
                    print("   \(index + 1). \(String(format: "%6.2f", segment.start))s → \(String(format: "%6.2f", segment.end))s (duration: \(String(format: "%.2f", segmentDuration))s)")
                }
                let speechPercentage = (totalSpeechTime / duration) * 100
                print("\n   Total speech time: \(String(format: "%.2f", totalSpeechTime))s (\(String(format: "%.1f", speechPercentage))%)")
            }
            
            print("\n" + String(repeating: "=", count: 60))
            print("✅ Test completed successfully!")
            
        } catch {
            print("\n❌ Error: \(error)")
            exit(1)
        }
    }
}
