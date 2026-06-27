import { useState } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { ChevronLeft } from "lucide-react";
import {
  Brand,
  CaptureButton,
  MeetingCard,
  Tabs,
  Message,
  SpeakerSegment,
  Prose,
  PillToggle,
  AudioPlayer,
  StatusDot,
  IconButton,
  Button,
  type MeetingSource,
} from "../index";

/**
 * Full Meeting Noter home screen assembled from the design system — an
 * interactive prototype of the modern-minimal UI. NOT a library component;
 * excluded from design-sync via cfg.titleMap.
 */

type Tab = "transcript" | "summary" | "diarization";

const MEETINGS: {
  id: string;
  title: string;
  timestamp: string;
  source: MeetingSource;
  recording?: boolean;
}[] = [
  { id: "q3", title: "Q3 Roadmap Sync", timestamp: "10:24 · 27/06/26", source: "Teams", recording: true },
  { id: "design", title: "Design Review — Mobile", timestamp: "09:10 · 27/06/26", source: "Meet" },
  { id: "alex", title: "1:1 with Alex", timestamp: "16:45 · 26/06/26", source: "Zoom" },
  { id: "retro", title: "Sprint Retro", timestamp: "14:00 · 25/06/26", source: "Teams" },
];

const TRANSCRIPT = [
  { speaker: "Sarah", ts: "00:02", text: "Thanks everyone for joining. Let's start with where we are on the Q3 roadmap and then talk through the open risks.", me: false },
  { speaker: "You", ts: "00:11", text: "Sounds good. The transcription pipeline is shipped and stable — we serialized the CoreML inference last week so the over-release crash is gone.", me: true },
  { speaker: "Sarah", ts: "00:24", text: "Great. What's the status on diarization quality? Last I heard speaker labels were drifting on longer calls.", me: false },
  { speaker: "You", ts: "00:31", text: "Improved. We're clustering on embeddings now and letting users rename speakers inline, which sticks across the session.", me: true },
];

const SEGMENTS = [
  { speaker: "Sarah", ts: "10:24:02", text: "Thanks everyone for joining. Let's start with the Q3 roadmap." },
  { speaker: "You", ts: "10:24:11", text: "Sounds good. The transcription pipeline is shipped and stable." },
  { speaker: "Sarah", ts: "10:24:24", text: "What's the status on diarization quality?" },
];

const SUMMARY = `### TL;DR
Q3 roadmap on track — transcription stable, diarization improved. Two risks flagged for follow-up.

### Key points
- Transcription pipeline shipped; CoreML crash resolved.
- Diarization now clusters on embeddings; inline speaker rename persists.
- Onboarding download progress still needs scoping.

### Action items
- You — scope onboarding download progress (Thu).
- Sarah — review diarization metrics on long calls (Fri).`;

const AppPrototype = ({ vintage }: { vintage?: boolean }) => {
  const [tab, setTab] = useState<Tab>("transcript");
  const [active, setActive] = useState("q3");
  const [capturing, setCapturing] = useState(true);
  const [lang, setLang] = useState("en");
  const [playing, setPlaying] = useState(false);

  return (
    <div
      className={vintage ? "ds-root ds-theme-vintage" : "ds-root"}
      style={{
        height: "100vh",
        background: "var(--ds-bg)",
        padding: 24,
        boxSizing: "border-box",
      }}
    >
      <div
        style={{
          maxWidth: 1180,
          height: "100%",
          margin: "0 auto",
          background: "var(--ds-surface)",
          border: "1px solid var(--ds-border)",
          borderRadius: "var(--ds-radius-lg)",
          boxShadow: "var(--ds-shadow-lg)",
          overflow: "hidden",
          display: "flex",
        }}
      >
        {/* Sidebar */}
        <aside
          style={{
            width: 300,
            flexShrink: 0,
            borderRight: "1px solid var(--ds-border)",
            background: "#fcfcfb",
            padding: 16,
            display: "flex",
            flexDirection: "column",
            gap: 14,
          }}
        >
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
            <Brand />
            <IconButton icon={<ChevronLeft size={18} />} label="Collapse sidebar" />
          </div>
          <CaptureButton capturing={capturing} onClick={() => setCapturing((c) => !c)} />
          <div style={{ fontSize: 11, fontWeight: 600, letterSpacing: ".06em", color: "var(--ds-text-3)", textTransform: "uppercase", padding: "2px 4px" }}>
            Meetings
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: 4, overflow: "auto" }}>
            {MEETINGS.map((m) => (
              <MeetingCard
                key={m.id}
                title={m.title}
                timestamp={m.timestamp}
                source={m.source}
                recording={m.recording && capturing}
                active={m.id === active}
                onClick={() => setActive(m.id)}
              />
            ))}
          </div>
        </aside>

        {/* Main */}
        <main style={{ flex: 1, display: "flex", flexDirection: "column", minWidth: 0, padding: 18, gap: 14 }}>
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <Tabs
              items={[
                { id: "transcript", label: "Transcript" },
                { id: "summary", label: "Summary" },
                { id: "diarization", label: "Diarization" },
              ]}
              value={tab}
              onChange={(t) => setTab(t as Tab)}
            />
            <div style={{ marginLeft: "auto" }}>
              <StatusDot tone={capturing ? "rec" : "idle"} pulse={capturing} label={capturing ? "Listening" : "Idle"} />
            </div>
          </div>

          <div
            style={{
              flex: 1,
              border: "1px solid var(--ds-border)",
              borderRadius: "var(--ds-radius)",
              padding: 22,
              overflow: "auto",
            }}
          >
            {tab === "transcript" && (
              <div>
                {TRANSCRIPT.map((m, i) => (
                  <Message
                    key={i}
                    speaker={m.speaker}
                    timestamp={m.ts}
                    content={m.text}
                    isUser={m.me}
                    divided={i < TRANSCRIPT.length - 1}
                    onSeek={() => {}}
                  />
                ))}
              </div>
            )}
            {tab === "summary" && (
              <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
                <PillToggle
                  options={[
                    { id: "en", label: "Original" },
                    { id: "vi", label: "Tiếng Việt" },
                  ]}
                  value={lang}
                  onChange={setLang}
                />
                <Prose markdown={SUMMARY} />
                <Button variant="ghost" pill style={{ alignSelf: "flex-start" }}>
                  Regenerate
                </Button>
              </div>
            )}
            {tab === "diarization" && (
              <div>
                {SEGMENTS.map((s, i) => (
                  <SpeakerSegment
                    key={i}
                    speaker={s.speaker}
                    timestamp={s.ts}
                    content={s.text}
                    divided={i < SEGMENTS.length - 1}
                    onRename={() => {}}
                  />
                ))}
              </div>
            )}
          </div>

          <AudioPlayer
            current="00:31"
            duration="02:14"
            progress={0.23}
            playing={playing}
            onToggle={() => setPlaying((p) => !p)}
          />
        </main>
      </div>
    </div>
  );
};

const meta = {
  title: "Prototype/Meeting Noter App",
  component: AppPrototype,
  parameters: { layout: "fullscreen" },
} satisfies Meta<typeof AppPrototype>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Home: Story = {};

/** Same screen, Sepia / Paper vintage theme (.ds-theme-vintage). */
export const Vintage: Story = { args: { vintage: true } };
