# Meeting Noter — Architecture

Tauri desktop app (macOS). React/TS frontend, Rust backend, Swift `FluidAudioBridge`
for on-device ASR / VAD / diarization. All processing is offline/local.

## 1. System architecture

```mermaid
flowchart TB
  subgraph FE["Frontend — React + TypeScript"]
    direction TB
    Pages["HomePage (pages/home.tsx)"]
    subgraph UI["Feature components"]
      Sidebar["Sidebar<br/>(meeting list, rename, capture toggle)"]
      Main["MainContent<br/>Tabs: Transcript / Summary / Diarization"]
      Alert["AlertWindow<br/>(meeting-detected overlay)"]
    end
    Store["Zustand store<br/>meetings / ui / teams slices"]
    subgraph Svc["Services (Tauri invoke wrappers)"]
      SvcT["transcription.ts"]
      SvcM["meetings.ts"]
      SvcD["diarization.ts"]
      SvcDet["meeting-detector.ts"]
    end
    Pages --> UI
    Pages --> Store
    Pages --> Svc
    UI --> Store
  end

  IPC{{"Tauri IPC<br/>invoke commands + events"}}

  subgraph BE["Backend — Rust (src-tauri)"]
    direction TB
    subgraph Cmd["commands/"]
      CT["transcription"]
      CM["meetings"]
      CD["diarization"]
      CDet["detection"]
    end
    Session["session.rs<br/>RecorderState, event emit"]
    Recorder["recorder.rs<br/>AudioRecorder"]
    subgraph Audio["audio/"]
      Cap["capture: mic / speaker (CoreAudio)"]
      Proc["processing: mixer + resampler"]
      Pipe["transcription: pipeline + segmenter"]
    end
    Detector["meeting_detector.rs"]
    Paths["paths.rs (file storage)"]
    Bridge["bridges/fluidaudio.rs<br/>FFI wrapper"]
  end

  Native["FluidAudioBridge (Swift)<br/>ASR + VAD + Diarization models"]
  FS[("Meeting dir on disk<br/>audio.wav / mic.wav / speaker.wav / meeting.json")]

  Svc <--> IPC
  IPC <--> Cmd
  CT --> Session --> Recorder
  Recorder --> Audio
  CD --> Bridge
  CDet --> Detector
  Cap --> Proc --> FS
  Pipe --> Bridge
  Bridge --> Native
  Proc --> Pipe
  CM --> Paths --> FS
  Detector -. "emits meeting-detected" .-> IPC
  Session -. "emits transcription://chunk" .-> IPC
```

## 2. How transcription works (live capture)

```mermaid
sequenceDiagram
  autonumber
  participant UI as HomePage
  participant Cmd as start_transcription
  participant Sess as session.rs
  participant Rec as AudioRecorder
  participant Cap as Mic/Speaker threads
  participant Mix as Mixer
  participant Pipe as Pipeline (per source)
  participant Seg as Segmenter
  participant FA as FluidAudio (Swift FFI)
  participant Store as Zustand store

  UI->>Cmd: invoke start_transcription(meetingId)
  Cmd->>Sess: session::start
  Sess->>Rec: recorder.start(events_tx, meetingId)
  Rec->>FA: initialize ASR + VAD (once)
  Rec->>Mix: spawn mixer (writes WAV tracks)
  Rec->>Pipe: spawn pipeline x2 (mic, speaker)
  Rec->>Cap: spawn capture threads

  loop per audio chunk (48kHz)
    Cap->>Mix: raw samples -> audio.wav / mic.wav / speaker.wav
    Cap->>Pipe: same samples (per-source channel)
    Pipe->>Pipe: resample 48k -> 16k
    Pipe->>FA: VAD.process(chunk) -> voice probability
    Pipe->>Seg: push(voice_prob, samples)
    alt buffer ready
      Seg-->>Pipe: Segment {Partial | Segment | Sentence}
      Pipe->>FA: asr.transcribe(audio) [sequential, in-order]
      FA-->>Pipe: text
      Pipe->>Sess: TranscriptionEvent{source, text, finality}
      Sess-->>UI: emit "transcription://chunk"
      UI->>Store: add / update transcript message
    else not ready
      Seg-->>Pipe: (buffer, emit nothing)
    end
  end

  UI->>Cmd: invoke stop_transcription
  Cmd->>Sess: cancel token -> threads drain & exit, WAV finalized
```

### Segmenter finality (what drives partial vs final text)

| Finality   | When                                          | Frontend effect                  |
|------------|-----------------------------------------------|----------------------------------|
| `Partial`  | While speaking, throttled preview             | Updates current message (live)   |
| `Segment`  | Buffer hits flush/max size during long speech | Commits text, keeps same message |
| `Sentence` | Silence lasts past threshold                  | Ends message, next starts new    |

Key properties:
- One pipeline per source (mic, speaker) → independent VAD state, no cross-talk.
- ASR runs **sequentially per stream** → results never arrive out of order; stale
  partials are dropped when newer audio is already queued.
- Mixer writes three WAVs: `audio.wav` (mixed), `mic.wav`, `speaker.wav` — the
  per-source tracks feed offline diarization later.
