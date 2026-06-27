import { useEffect, useState } from "react";
import {
  onSetupStage,
  prefetchModels,
  type SetupStage,
} from "@/services/setup";
import { SetupCard, Step, Button } from "@/design-system";

// Ordered download steps shown as a checklist. Sizes are approximate on-disk
// totals; transcription is by far the largest, which is why a single percent
// bar would stall then jump — the checklist gives honest per-step progress.
const STEPS: { stage: SetupStage; label: string; size: string }[] = [
  { stage: "preparing-transcription", label: "Transcription model", size: "~444 MB" },
  { stage: "preparing-speaker", label: "Speaker model", size: "~13 MB" },
];

// Index of the in-flight step; equals STEPS.length once everything is done.
const stepIndex = (stage: SetupStage) => {
  const i = STEPS.findIndex((s) => s.stage === stage);
  return i === -1 ? STEPS.length : i;
};

const formatElapsed = (seconds: number) => {
  const mm = Math.floor(seconds / 60);
  const ss = seconds % 60;
  return `${String(mm).padStart(2, "0")}:${String(ss).padStart(2, "0")}`;
};

/**
 * One-time setup screen shown on first launch while the speech models
 * download. Blocks the app until models are ready; on failure the user can
 * retry (there is no skip — the app needs the models to function).
 */
export const Onboarding = ({ onDone }: { onDone: () => void }) => {
  const [stage, setStage] = useState<SetupStage>("preparing-transcription");
  const [error, setError] = useState<string | null>(null);
  const [elapsed, setElapsed] = useState(0);
  // Bumping `attempt` re-runs the prefetch effect on retry.
  const [attempt, setAttempt] = useState(0);

  // Elapsed timer, reset on each attempt and stopped on error.
  useEffect(() => {
    if (error) return;
    setElapsed(0);
    const started = Date.now();
    const id = setInterval(
      () => setElapsed(Math.floor((Date.now() - started) / 1000)),
      1000,
    );
    return () => clearInterval(id);
  }, [error, attempt]);

  // Run the download and stream stage updates.
  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    setError(null);
    setStage("preparing-transcription");

    onSetupStage((s) => setStage(s)).then((u) => {
      if (cancelled) u();
      else unlisten = u;
    });

    prefetchModels()
      .then(() => {
        if (!cancelled) onDone();
      })
      .catch((e) => {
        if (!cancelled) setError(String(e));
      });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [attempt, onDone]);

  const current = stepIndex(stage);

  return (
    <div className="ds-root ds-theme-vintage flex h-screen flex-col items-center justify-center px-8" style={{ background: "var(--ds-bg)" }}>
      <SetupCard
        title="Setting up Noter"
        subtitle={
          error
            ? "Couldn’t download the speech models."
            : current < STEPS.length
              ? `Step ${current + 1} of ${STEPS.length} · Downloading models…`
              : "Finishing up…"
        }
        footer={
          error ? (
            <div className="flex flex-col items-center gap-3" style={{ marginTop: 22 }}>
              <p className="text-xs" style={{ color: "var(--ds-text-3)" }}>{error}</p>
              <Button variant="primary" onClick={() => setAttempt((n) => n + 1)}>
                Try again
              </Button>
            </div>
          ) : (
            <p className="text-xs tabular-nums" style={{ marginTop: 22, color: "var(--ds-text-3)" }}>
              {formatElapsed(elapsed)} · ~450 MB · one-time download, usually 1–2 min
              {elapsed > 30 && " · large files, hang tight"}
            </p>
          )
        }
      >
        {STEPS.map((s, i) => (
          <Step
            key={s.stage}
            index={i + 1}
            label={s.label}
            state={error ? "todo" : i < current ? "done" : i === current ? "active" : "todo"}
            hint={s.size}
          />
        ))}
      </SetupCard>
    </div>
  );
};
