import { useCallback, useState } from "react";
import { Check } from "lucide-react";
import { SetupCard, Step, Button } from "@/design-system";
import { cn, formatBytes } from "@/lib/utils";
import { LANGUAGES } from "@/lib/languages";
import type { MeetingLanguage } from "@/types/meeting";
import { useModelManager } from "@/features/models/use-model-manager";
import { DownloadIndicator } from "@/features/models/download-indicator";

type Phase = "select" | "download";

/**
 * First-launch setup. The user picks which language model(s) to install
 * (at least one — the app needs a model to transcribe), then they download
 * with real per-language progress. More languages can be added later from the
 * in-app Models panel.
 */
export const Onboarding = ({ onDone }: { onDone: () => void }) => {
  const [phase, setPhase] = useState<Phase>("select");
  const [selected, setSelected] = useState<MeetingLanguage[]>(["en"]);
  const { errors, progress, sizes, download, cancel, isInstalled, isDownloading } =
    useModelManager();

  // Cancel any in-flight downloads and return to the language picker.
  const cancelAll = () => {
    selected.forEach((l) => cancel(l));
    setPhase("select");
  };

  // Real download size when known, else the static estimate.
  const sizeOf = (lang: MeetingLanguage, fallback: string) => {
    const b = sizes[lang];
    return b ? formatBytes(b) : fallback;
  };

  // Total download for the current selection (only the languages not yet
  // installed, and only when their real sizes are known).
  const totalBytes = selected
    .filter((l) => !isInstalled(l))
    .reduce((sum, l) => sum + (sizes[l] ?? 0), 0);

  const toggle = (lang: MeetingLanguage) =>
    setSelected((s) =>
      s.includes(lang) ? s.filter((l) => l !== lang) : [...s, lang],
    );

  // Download each selected language in turn; finish once they all install.
  // Stops on the first failure so the user can retry (already-installed
  // languages are skipped on retry).
  const runDownloads = useCallback(
    async (langs: MeetingLanguage[]) => {
      for (const lang of langs) {
        if (isInstalled(lang)) continue;
        const ok = await download(lang);
        if (!ok) return;
      }
      onDone();
    },
    [download, isInstalled, onDone],
  );

  const start = () => {
    setPhase("download");
    void runDownloads(selected);
  };

  return (
    <div
      className="ds-root ds-theme-vintage flex h-screen flex-col items-center justify-center px-8"
      style={{ background: "var(--ds-bg)" }}
    >
      {phase === "select" ? (
        <SetupCard
          title="Setting up Meeting Noter"
          subtitle="Choose the languages you'll transcribe. You can add more later."
          footer={
            <div className="flex flex-col gap-2" style={{ marginTop: 22 }}>
              <Button
                variant="primary"
                block
                disabled={selected.length === 0}
                onClick={start}
              >
                {selected.length > 1 ? "Download models" : "Download model"}
              </Button>
              {selected.length === 0 ? (
                <span className="text-xs text-[var(--ds-text-3)] text-center">
                  Select at least one language to continue
                </span>
              ) : totalBytes > 0 ? (
                <span className="text-xs text-[var(--ds-text-3)] text-center">
                  About {formatBytes(totalBytes)} to download
                </span>
              ) : null}
            </div>
          }
        >
          <div className="flex flex-col gap-2">
            {LANGUAGES.map((lang) => {
              const active = selected.includes(lang.id);
              return (
                <button
                  key={lang.id}
                  type="button"
                  role="checkbox"
                  aria-checked={active}
                  onClick={() => toggle(lang.id)}
                  className={cn(
                    "flex items-center gap-3 p-3 rounded-[var(--ds-radius-sm)] border text-left transition-colors",
                    active
                      ? "border-[var(--ds-accent)] bg-[var(--ds-surface-2)]"
                      : "border-[var(--ds-border)] hover:bg-[var(--ds-surface-2)]",
                  )}
                >
                  <span
                    className={cn(
                      "flex items-center justify-center w-5 h-5 rounded-[6px] border shrink-0",
                      active
                        ? "bg-[var(--ds-accent)] border-[var(--ds-accent)] text-[var(--ds-bg)]"
                        : "border-[var(--ds-border-2)]",
                    )}
                  >
                    {active && <Check className="w-3.5 h-3.5" />}
                  </span>
                  <span className="flex flex-col min-w-0 flex-1">
                    <span className="text-sm font-medium text-[var(--ds-text)]">
                      {lang.label}
                      {lang.native !== lang.label && (
                        <span className="text-[var(--ds-text-3)] font-normal">
                          {" "}
                          · {lang.native}
                        </span>
                      )}
                    </span>
                    <span className="text-xs text-[var(--ds-text-3)]">
                      {sizeOf(lang.id, lang.approxSize)}
                    </span>
                  </span>
                </button>
              );
            })}
          </div>
        </SetupCard>
      ) : (
        <SetupCard
          title="Setting up Meeting Noter"
          subtitle="Downloading your language models — this is a one-time step."
          footer={
            <div className="flex flex-col gap-3" style={{ marginTop: 22 }}>
              <p className="text-xs" style={{ color: "var(--ds-text-3)" }}>
                Models run fully offline. Large files — usually 1–2 min each.
              </p>
              {selected.some((l) => isDownloading(l)) && (
                <Button variant="ghost" size="sm" pill onClick={cancelAll}>
                  Cancel
                </Button>
              )}
            </div>
          }
        >
          {selected.map((lang, i) => {
            const info = LANGUAGES.find((l) => l.id === lang)!;
            const done = isInstalled(lang);
            const active = isDownloading(lang);
            const error = errors[lang];
            return (
              <Step
                key={lang}
                index={i + 1}
                label={`${info.label} model`}
                state={done ? "done" : active ? "active" : "todo"}
                hint={done ? "Installed" : sizeOf(lang, info.approxSize)}
              >
                {active && <DownloadIndicator value={progress[lang]} />}
                {error && (
                  <div className="flex flex-col items-start gap-2 mt-2">
                    <p className="text-xs text-[var(--ds-rec)]">{error}</p>
                    <Button
                      variant="ghost"
                      size="sm"
                      pill
                      onClick={() => void runDownloads(selected)}
                    >
                      Try again
                    </Button>
                  </div>
                )}
              </Step>
            );
          })}
        </SetupCard>
      )}
    </div>
  );
};
