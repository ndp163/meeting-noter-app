import Foundation
import FluidAudio

actor VadService {
    private let vadManager: VadManager
    private var states: [String: VadStreamState] = [:]

    init(vadManager: VadManager) {
        self.vadManager = vadManager
    }

    func createState(id: String) async {
        let state = await vadManager.makeStreamState()
        states[id] = state
    }

    func destroyState(id: String) {
        states.removeValue(forKey: id)
    }

    func process(id: String, samples: [Float]) async throws -> Float {
        guard let state = states[id] else {
            throw VadError.stateNotFound
        }

        let result = try await vadManager.processStreamingChunk(
            samples,
            state: state,
            returnSeconds: true,
            timeResolution: 2
        )

        states[id] = result.state
        return result.probability
    }
}