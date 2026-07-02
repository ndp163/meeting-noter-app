import Foundation
import Translation

// Apple Translation framework bridge (on-device, offline).
//
// Two paths, matching how the framework actually works (proven in model-test):
//   • Translation itself is HEADLESS: `TranslationSession(installedSource:target:)`
//     + `translate()` — no SwiftUI. Requires macOS 26 and the language pack
//     already installed. ~20ms/segment.
//   • Pack DOWNLOAD needs the SwiftUI `.translationTask` consent sheet — a
//     third-party app cannot download packs headlessly (Translate-app /
//     Settings downloads do NOT register for us). We present that sheet in a
//     small window over the app (one time per language pair).
//
// Reuses `FluidAudioCallback` (text, error, context) from FluidAudio.swift —
// same module.

// MARK: - Headless translation session cache

@available(macOS 26, *)
final class TranslateBridge: @unchecked Sendable {
    static let shared = TranslateBridge()
    private var sessions: [String: TranslationSession] = [:]
    private let lock = NSLock()

    private func key(_ src: String, _ tgt: String) -> String { "\(src)>\(tgt)" }

    func session(src: String, tgt: String) -> TranslationSession {
        lock.lock(); defer { lock.unlock() }
        let k = key(src, tgt)
        if let s = sessions[k] { return s }
        let s = TranslationSession(
            installedSource: Locale.Language(identifier: src),
            target: Locale.Language(identifier: tgt)
        )
        sessions[k] = s
        return s
    }

    func clear() {
        lock.lock(); sessions.removeAll(); lock.unlock()
    }
}

// MARK: - Status

/// "installed" | "supported" | "unsupported"; error on OS < 26.
@_cdecl("apple_translate_status")
public func apple_translate_status(
    source: UnsafePointer<CChar>?,
    target: UnsafePointer<CChar>?,
    callback: FluidAudioCallback?,
    context: UnsafeMutableRawPointer?
) {
    guard let source, let target, let callback else { return }
    let src = String(cString: source)
    let tgt = String(cString: target)

    guard #available(macOS 26, *) else {
        let e = strdup("Translation requires macOS 26")
        callback(nil, e, context); free(e); return
    }

    Task {
        let availability = LanguageAvailability()
        let status = await availability.status(
            from: Locale.Language(identifier: src),
            to: Locale.Language(identifier: tgt)
        )
        let text: String
        switch status {
        case .installed: text = "installed"
        case .supported: text = "supported"
        case .unsupported: text = "unsupported"
        @unknown default: text = "unsupported"
        }
        let s = strdup(text)
        callback(s, nil, context); free(s)
    }
}

// MARK: - Translate one string

@_cdecl("apple_translate_text")
public func apple_translate_text(
    source: UnsafePointer<CChar>?,
    target: UnsafePointer<CChar>?,
    text: UnsafePointer<CChar>?,
    callback: FluidAudioCallback?,
    context: UnsafeMutableRawPointer?
) {
    guard let source, let target, let text, let callback else { return }
    let src = String(cString: source)
    let tgt = String(cString: target)
    let input = String(cString: text)

    guard #available(macOS 26, *) else {
        let e = strdup("Translation requires macOS 26")
        callback(nil, e, context); free(e); return
    }

    Task {
        do {
            let session = TranslateBridge.shared.session(src: src, tgt: tgt)
            let resp = try await session.translate(input)
            let s = strdup(resp.targetText)
            callback(s, nil, context); free(s)
        } catch {
            let e = strdup(error.localizedDescription)
            callback(nil, e, context); free(e)
        }
    }
}
