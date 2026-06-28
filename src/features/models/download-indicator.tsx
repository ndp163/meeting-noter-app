import { ProgressBar } from "@/design-system";

/**
 * Determinate download progress. The self-hosted S3 downloader resolves the
 * total byte size up-front (HEAD on every file) and reports byte-accurate
 * fractions, so we show a real percentage bar. Before the first byte lands
 * (size-resolution phase) `value` is 0/undefined — fall back to an indeterminate
 * bar so it never looks stuck at 0%.
 */
export const DownloadIndicator = ({ value }: { value?: number }) => {
  const known = typeof value === "number" && value > 0;
  const pct = Math.round((value ?? 0) * 100);
  return (
    <div className="flex flex-col gap-1 mt-1.5">
      <ProgressBar value={value ?? 0} indeterminate={!known} />
      <span className="text-xs tabular-nums self-end text-[var(--ds-text-3)]">
        {known ? `${pct}%` : "Starting…"}
      </span>
    </div>
  );
};
