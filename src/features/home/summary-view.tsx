import { useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Button, PillToggle, Prose } from "@/design-system";

const CLAUDE_CODE_URL = "https://docs.claude.com/en/docs/claude-code/setup";

interface SummaryViewProps {
  summary: string | undefined;
  summaryVi: string | undefined;
  isLoading: boolean;
  isTranslating: boolean;
  error?: string;
  translateError?: string;
  canRun: boolean;
  claudeReady: boolean | undefined;
  onRun: () => void;
  onTranslate: () => void;
}

type Lang = "en" | "vi";

export const SummaryView = ({
  summary,
  summaryVi,
  isLoading,
  isTranslating,
  error,
  translateError,
  canRun,
  claudeReady,
  onRun,
  onTranslate,
}: SummaryViewProps) => {
  const [lang, setLang] = useState<Lang>("en");

  if (isLoading) {
    return (
      <div className="text-[var(--ds-text-2)] text-sm">
        Generating summary with Claude…
      </div>
    );
  }

  if (claudeReady === false) {
    return (
      <div className="flex flex-col gap-2 items-start text-[var(--ds-text-2)] text-sm">
        <p>
          AI summaries run locally through the Claude Code CLI — your transcript
          never leaves this machine. This is the command-line tool (the{" "}
          <code className="text-[var(--ds-text)]">claude</code> binary), not the
          Claude desktop app. Install it, run{" "}
          <code className="text-[var(--ds-text)]">claude login</code>, then
          reopen this tab.
        </p>
        <button
          type="button"
          className="underline text-[var(--ds-text)] hover:text-[var(--ds-accent)]"
          onClick={() => void openUrl(CLAUDE_CODE_URL)}
        >
          Install the Claude Code CLI
        </button>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex flex-col gap-3 items-start">
        <p className="text-sm text-[var(--ds-rec)]">{error}</p>
        <Button variant="ghost" pill onClick={onRun}>
          Try again
        </Button>
      </div>
    );
  }

  if (!summary) {
    if (!canRun) {
      return (
        <div className="text-[var(--ds-text-2)] text-sm">
          A summary is available after a recording finishes.
        </div>
      );
    }
    return (
      <div className="flex flex-col gap-3 items-start">
        <p className="text-sm text-[var(--ds-text-2)]">
          Generate a TL;DR, key points, decisions, and action items from this
          meeting's transcript using Claude.
        </p>
        <Button variant="ghost" pill onClick={onRun}>
          Generate summary
        </Button>
      </div>
    );
  }

  const shown = lang === "vi" ? summaryVi : summary;

  return (
    <div className="flex flex-col gap-4">
      <PillToggle
        options={[
          { id: "en", label: "Original" },
          { id: "vi", label: "Tiếng Việt" },
        ]}
        value={lang}
        onChange={(id) => setLang(id as Lang)}
      />

      {lang === "vi" && !summaryVi ? (
        <div className="flex flex-col gap-3 items-start">
          {translateError && (
            <p className="text-sm text-[var(--ds-rec)]">{translateError}</p>
          )}
          {isTranslating ? (
            <p className="text-[var(--ds-text-2)] text-sm">
              Đang dịch sang tiếng Việt…
            </p>
          ) : (
            <Button variant="ghost" pill onClick={onTranslate}>
              Dịch sang tiếng Việt
            </Button>
          )}
        </div>
      ) : (
        <Prose markdown={shown} />
      )}

      <div className="flex items-center gap-2">
        <Button variant="ghost" pill onClick={onRun}>
          Regenerate
        </Button>
        {lang === "vi" && summaryVi && !isTranslating && (
          <Button variant="ghost" pill onClick={onTranslate}>
            Dịch lại
          </Button>
        )}
      </div>
    </div>
  );
};
