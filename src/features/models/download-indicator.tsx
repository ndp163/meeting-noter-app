import { useEffect, useState } from "react";
import { ProgressBar } from "@/design-system";

/**
 * Download activity indicator. FluidAudio downloads the large model weight as a
 * single file with no per-byte progress (and bounces across phases), so a
 * percentage bar reads as stuck or jumpy. Instead we show a continuous
 * indeterminate bar plus an elapsed timer — honest "still working" feedback
 * through the multi-minute download. Mount only while a download is in flight;
 * the timer starts from zero on mount.
 */
export const DownloadIndicator = () => {
  const [seconds, setSeconds] = useState(0);
  useEffect(() => {
    const id = setInterval(() => setSeconds((s) => s + 1), 1000);
    return () => clearInterval(id);
  }, []);

  const mm = String(Math.floor(seconds / 60)).padStart(2, "0");
  const ss = String(seconds % 60).padStart(2, "0");

  return (
    <div className="flex flex-col gap-1 mt-1.5">
      <ProgressBar indeterminate />
      <span className="text-xs tabular-nums self-end text-[var(--ds-text-3)]">
        Downloading… {mm}:{ss}
      </span>
    </div>
  );
};
