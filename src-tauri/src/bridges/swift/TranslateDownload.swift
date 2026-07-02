import Foundation
#if canImport(AppKit)
import AppKit
import SwiftUI
import Translation

// One-time language-pack download. The Translation framework only downloads a
// pack via the SwiftUI `.translationTask` consent flow, so we host a tiny
// SwiftUI view in a small window over the app to trigger it. Proven to work
// inside a plain NSApplication (which Tauri is) in model-test/TranslateEmbedTest.

@available(macOS 26, *)
@MainActor
final class TranslateDownloadPresenter {
    static let shared = TranslateDownloadPresenter()
    private var window: NSWindow?

    func present(src: String, tgt: String, done: @escaping (String?) -> Void) {
        // Only one at a time.
        if window != nil { done("A download is already in progress"); return }

        let view = TranslateDownloadView(source: src, target: tgt) { [weak self] error in
            self?.window?.close()
            self?.window = nil
            done(error)
        }
        let hosting = NSHostingController(rootView: view)
        let w = NSWindow(contentViewController: hosting)
        w.setContentSize(NSSize(width: 420, height: 180))
        w.styleMask = [.titled]
        w.title = "Download translation language"
        w.center()
        w.isReleasedWhenClosed = false
        w.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
        window = w
    }
}

@available(macOS 26, *)
private struct TranslateDownloadView: View {
    let source: String
    let target: String
    let onFinish: (String?) -> Void

    @State private var config: TranslationSession.Configuration?
    @State private var message = "Preparing…"

    var body: some View {
        VStack(spacing: 16) {
            ProgressView()
            Text(message).font(.callout).multilineTextAlignment(.center)
        }
        .padding(24)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .onAppear {
            config = .init(
                source: Locale.Language(identifier: source),
                target: Locale.Language(identifier: target)
            )
        }
        .translationTask(config) { session in
            do {
                message = "Downloading \(source) → \(target)…"
                try await session.prepareTranslation()
                onFinish(nil)
            } catch {
                onFinish(error.localizedDescription)
            }
        }
    }
}

/// Trigger the one-time download sheet. Calls back "success" or an error.
@_cdecl("apple_translate_download")
public func apple_translate_download(
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

    // Wrap the FFI pointers so the @Sendable main-actor closure can carry them.
    let sink = TranslateCallbackSink(callback: callback, context: context)
    Task { @MainActor in
        TranslateDownloadPresenter.shared.present(src: src, tgt: tgt) { error in
            if let error {
                let e = strdup(error)
                sink.callback(nil, e, sink.context); free(e)
            } else {
                let s = strdup("success")
                sink.callback(s, nil, sink.context); free(s)
            }
        }
    }
}

private struct TranslateCallbackSink: @unchecked Sendable {
    let callback: FluidAudioCallback
    let context: UnsafeMutableRawPointer?
}
#endif
