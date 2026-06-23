import { useEffect, useState } from "react";
import { Check, Loader2 } from "lucide-react";
import {
  onSetupStage,
  prefetchModels,
  type SetupStage,
} from "@/services/setup";

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
    <div className="flex h-screen flex-col items-center justify-center gap-4 px-8 text-center">
      <h1 className="text-lg font-medium text-custom-text-primary">
        Setting up Noter
      </h1>

      {error ? (
        <div className="flex flex-col items-center gap-3">
          <p className="max-w-sm text-sm text-custom-red">
            Couldn’t download the speech models. Check your connection and try
            again.
          </p>
          <p className="max-w-sm text-xs text-custom-text-secondary">{error}</p>
          <button
            onClick={() => setAttempt((n) => n + 1)}
            className="rounded-full border border-custom-text-primary px-4 py-1.5 text-sm text-custom-text-primary hover:bg-custom-bg-secondary"
          >
            Try again
          </button>
        </div>
      ) : (
        <>
          <p className="text-sm text-custom-text-secondary">
            {current < STEPS.length
              ? `Step ${current + 1} of ${STEPS.length} · Downloading models…`
              : "Finishing up…"}
          </p>

          <ul className="flex flex-col gap-2 text-left">
            {STEPS.map((s, i) => {
              const done = i < current;
              const active = i === current;
              return (
                <li key={s.stage} className="flex items-center gap-2.5 text-sm">
                  <span className="flex h-5 w-5 items-center justify-center">
                    {done ? (
                      <Check className="h-4 w-4 text-custom-text-highlight" />
                    ) : active ? (
                      <Loader2 className="h-4 w-4 animate-spin text-custom-text-secondary" />
                    ) : (
                      <span className="h-2 w-2 rounded-full bg-custom-text-secondary/30" />
                    )}
                  </span>
                  <span
                    className={
                      done || active
                        ? "text-custom-text-primary"
                        : "text-custom-text-secondary/50"
                    }
                  >
                    {s.label}
                  </span>
                  <span className="text-xs text-custom-text-secondary/60 tabular-nums">
                    {s.size}
                  </span>
                </li>
              );
            })}
          </ul>

          <p className="text-xs text-custom-text-secondary tabular-nums">
            {formatElapsed(elapsed)} · ~450 MB · one-time download, usually 1–2
            min
          </p>

          {elapsed > 30 && (
            <p className="max-w-xs text-xs text-custom-text-secondary/70">
              Still going — these are large files and can take a minute on
              slower connections. Hang tight.
            </p>
          )}
        </>
      )}
    </div>
  );
};
