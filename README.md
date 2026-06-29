# Meeting Noter

[![Downloads](https://img.shields.io/github/downloads/ndp163/meeting-noter-releases/total?label=downloads&color=blue)](https://github.com/ndp163/meeting-noter-releases/releases)
[![License: AGPL-3.0](https://img.shields.io/badge/license-AGPL--3.0-blue)](LICENSE)
![Platform](https://img.shields.io/badge/platform-macOS%20(Apple%20Silicon)-lightgrey)

**Private, on-device meeting transcription for macOS.** Live transcript of any
meeting — mic + system audio — running 100% on your Mac. No cloud, no account,
no data ever leaves the machine.

> Built for people who can't (or won't) send meeting audio to a SaaS server:
> regulated industries, legal, healthcare, finance, and anyone who just wants
> their conversations to stay theirs.

<!-- TODO: drop a 15–20s demo GIF here — live transcription in action. This is
     the single highest-leverage thing in this README. Record it before launch. -->

## Why

Cloud notetakers (Granola, Otter, Fireflies…) upload your audio, charge per seat
per month, and your conversations live on someone else's servers. Meeting Noter
flips that:

- **100% offline** — ASR, voice detection, and speaker diarization all run
  locally via Apple's Neural Engine. Airplane mode works.
- **No subscription tax** — local inference costs nothing to run, so it doesn't
  need a per-minute cloud bill to cover.
- **Your audio stays on disk** — every meeting is just files in a folder you own.

## Features

- 🎙️ **Live dual-source capture** — transcribes your mic and the other side
  (system/speaker audio) as separate streams, so cross-talk never garbles either.
- ⚡ **Real-time transcript** — partial text as you speak, committed on silence.
- 🗣️ **Speaker diarization** — who-said-what, computed offline from the
  per-source tracks.
- 🌐 **Multi-language ASR** — English and Japanese today (more planned).
- 🔔 **Meeting auto-detect** — optional overlay when a meeting starts.
- 💾 **Local-first storage** — each meeting is `audio.wav` / `mic.wav` /
  `speaker.wav` / `meeting.json` in a directory you control.
- 🤖 **AI summaries (optional)** — TL;DR, key points, decisions, and action
  items, generated locally via the Claude Code CLI. See below.

## Requirements

- **macOS on Apple Silicon** (M1 or newer). Models run on CoreML / the Neural
  Engine; there is no Intel build.
- ~450 MB–600 MB disk per language for the ASR model (downloaded on first use).

## Install

<!-- TODO: link the latest signed + notarized .dmg / .app from the releases page. -->

Download the latest signed build from the [releases page](#). On first launch the
app downloads the on-device models, then you're ready.

## AI summaries (optional)

Transcription is fully offline and needs nothing extra. The **AI summary** tab
is the one optional feature with an external dependency: it shells out to the
[Claude Code CLI](https://docs.claude.com/en/docs/claude-code/setup) installed on
your own machine — the command-line `claude` binary, **not** the Claude desktop
app.

- The transcript is piped to the local `claude` binary on stdin — it goes to
  Claude Code, not to this app's servers (there are none). No API key is stored
  by Meeting Noter; it reuses your existing Claude Code login.
- To enable: install Claude Code, run `claude login`, then open the Summary tab
  and reopen it. The app auto-detects the `claude` binary in the usual install
  locations.
- If `claude` isn't found, the Summary tab tells you and links to the install
  page. Everything else in the app keeps working without it.

## Architecture

React + TypeScript frontend, Rust backend (Tauri 2), and a Swift
`FluidAudioBridge` that wraps the on-device ASR / VAD / diarization models. The
audio pipeline runs one transcription stream per source with a VAD-driven
segmenter, and diarization runs offline against the saved per-source WAVs.

Full diagrams and the transcription sequence: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Development

This is a pnpm workspace — **use pnpm, not npm/yarn**.

| Task | Command |
|---|---|
| Frontend dev | `pnpm dev` |
| Full app (Tauri) | `pnpm tauri dev` |
| Build | `pnpm build` |
| Storybook (design system) | `pnpm storybook` |
| Style guard | `pnpm lint:styles` |
| Rust | `cargo …` inside `src-tauri/` |

Contributor and architecture rules live in [CLAUDE.md](CLAUDE.md) and the nested
`CLAUDE.md` files under `src/design-system/` and `src-tauri/`.

## License

Licensed under the **GNU Affero General Public License v3.0** — see
[LICENSE](LICENSE). You may use, study, modify, and share it; if you run a
modified version as a network service or distribute it, you must release your
source under the same terms.
