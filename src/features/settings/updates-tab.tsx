import {
  RefreshCw,
  Download,
  CheckCircle2,
  AlertCircle,
  Loader2,
} from "lucide-react";
import { Button, Prose, ProgressBar } from "@/design-system";
import { useBoundStore } from "@/store";

// The updater date arrives as RFC3339-ish; show a readable date, fall back to
// the raw string if it doesn't parse.
const formatDate = (raw: string) => {
  const d = new Date(raw);
  return Number.isNaN(d.getTime())
    ? raw
    : d.toLocaleDateString(undefined, {
        year: "numeric",
        month: "short",
        day: "numeric",
      });
};

/** Update panel: current version, manual check, release notes, install. */
export const UpdatesTab = () => {
  const phase = useBoundStore.use.updatePhase();
  const info = useBoundStore.use.updateInfo();
  const progress = useBoundStore.use.updateProgress();
  const current = useBoundStore.use.currentVersion();
  const checkUpdate = useBoundStore.use.checkUpdate();
  const runInstall = useBoundStore.use.runInstall();

  const checking = phase === "checking";
  const installing = phase === "installing";

  return (
    <div className="flex flex-col gap-4 min-w-[420px]">
      <div className="flex items-center justify-between gap-4">
        <div className="flex flex-col">
          <span className="text-xs text-[var(--ds-text-2)]">
            Current version
          </span>
          <span className="text-base font-medium text-[var(--ds-text)]">
            {current ? `v${current}` : "—"}
          </span>
        </div>
        <Button
          variant="ghost"
          onClick={() => void checkUpdate()}
          disabled={checking || installing}
          icon={
            checking ? (
              <Loader2 className="w-4 h-4 animate-spin" />
            ) : (
              <RefreshCw className="w-4 h-4" />
            )
          }
        >
          {checking ? "Checking…" : "Check for updates"}
        </Button>
      </div>

      {phase === "uptodate" && (
        <div className="flex items-center gap-2 text-sm text-[var(--ds-ok)]">
          <CheckCircle2 className="w-4 h-4 shrink-0" />
          You&apos;re on the latest version.
        </div>
      )}

      {phase === "error" && (
        <div className="flex items-center gap-2 text-sm text-[var(--ds-rec)]">
          <AlertCircle className="w-4 h-4 shrink-0" />
          Couldn&apos;t check for updates. Check your connection and try again.
        </div>
      )}

      {(phase === "available" || installing) && info && (
        <div className="flex flex-col gap-3 p-4 rounded-[var(--ds-radius)] border border-[var(--ds-border)] bg-[var(--ds-surface-2)]">
          <div className="flex items-baseline justify-between gap-2">
            <span className="text-base font-semibold text-[var(--ds-text)]">
              Version {info.version} available
            </span>
            {info.date && (
              <span className="text-xs text-[var(--ds-text-3)]">
                {formatDate(info.date)}
              </span>
            )}
          </div>

          {info.notes ? (
            <Prose
              markdown={info.notes}
              className="max-h-64 overflow-y-auto pr-1"
            />
          ) : (
            <span className="text-sm text-[var(--ds-text-2)]">
              No release notes provided.
            </span>
          )}

          {installing ? (
            <div className="flex flex-col gap-1.5">
              <ProgressBar value={progress} />
              <span className="text-xs text-[var(--ds-text-2)]">
                Downloading… {Math.round(progress * 100)}%
              </span>
            </div>
          ) : (
            <Button
              variant="accent"
              icon={<Download className="w-4 h-4" />}
              onClick={() => void runInstall()}
            >
              Install &amp; restart
            </Button>
          )}
        </div>
      )}
    </div>
  );
};
