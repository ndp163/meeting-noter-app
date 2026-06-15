import { useEffect, useState } from "react";
import { Loader2 } from "lucide-react";
import {
  onSetupStage,
  prefetchModels,
  type SetupStage,
} from "@/services/setup";

const STAGE_LABEL: Record<SetupStage, string> = {
  "preparing-transcription": "Downloading transcription model…",
  "preparing-speaker": "Downloading speaker model…",
  ready: "Finishing up…",
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
          <Loader2 className="h-6 w-6 animate-spin text-custom-text-secondary" />
          <p className="text-sm text-custom-text-secondary">
            {STAGE_LABEL[stage]}
          </p>
          <p className="text-xs text-custom-text-secondary tabular-nums">
            {formatElapsed(elapsed)} · one-time download
          </p>
        </>
      )}
    </div>
  );
};
