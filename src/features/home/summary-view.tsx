import { useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  Sparkles,
  RefreshCw,
  RotateCcw,
  Languages,
  Download,
  ExternalLink,
} from "lucide-react";
import { Button, PillToggle, Prose } from "@/design-system";
import { useBoundStore } from "@/store";
import { targetLabel } from "@/services/translation";
import type { SummaryProvider } from "@/services/summary";

const CLAUDE_CODE_URL = "https://docs.claude.com/en/docs/claude-code/setup";

interface SummaryViewProps {
  summary: string | undefined;
  /** On-demand translation of the summary + the language it was made for. */
  summaryTranslation: { lang: string; text: string } | undefined;
  /** Configured translation target (BCP-47 code); "" = none set. */
  translateTarget: string;
  isLoading: boolean;
  isTranslating: boolean;
  error?: string;
  translateError?: string;
  canRun: boolean;
  /** Whether the selected provider is ready to run. */
  summaryReady: boolean | undefined;
  summaryProvider: SummaryProvider;
  onRun: () => void;
  onTranslate: () => void;
}

type View = "original" | "translated";

export const SummaryView = ({
  summary,
  summaryTranslation,
  translateTarget,
  isLoading,
  isTranslating,
  error,
  translateError,
  canRun,
  summaryReady,
  summaryProvider,
  onRun,
  onTranslate,
}: SummaryViewProps) => {
  const [view, setView] = useState<View>("original");
  const openSettings = useBoundStore.use.openSettings();

  if (isLoading) {
    return (
      <div className="text-[var(--ds-text-2)] text-sm">
        {summaryProvider === "local"
          ? "Generating summary on-device…"
          : "Generating summary with Claude…"}
      </div>
    );
  }

  if (summaryReady === false) {
    return summaryProvider === "local" ? (
      <div className="flex flex-col gap-2 items-start text-[var(--ds-text-2)] text-sm">
        <p>
          The on-device summary model isn't installed yet. Download it once
          (~2.3 GB) and summaries run fully offline — no network, no login.
        </p>
        <Button
          variant="primary"
          size="sm"
          icon={<Download className="w-4 h-4" />}
          onClick={() => openSettings("summary")}
        >
          Download the on-device model
        </Button>
      </div>
    ) : (
      <div className="flex flex-col gap-2 items-start text-[var(--ds-text-2)] text-sm">
        <p>
          AI summaries run locally through the Claude Code CLI — your transcript
          never leaves this machine. This is the command-line tool (the{" "}
          <code className="text-[var(--ds-text)]">claude</code> binary), not the
          Claude desktop app. Install it, run{" "}
          <code className="text-[var(--ds-text)]">claude login</code>, then
          reopen this tab.
        </p>
        <Button
          variant="primary"
          size="sm"
          icon={<ExternalLink className="w-4 h-4" />}
          onClick={() => void openUrl(CLAUDE_CODE_URL)}
        >
          Install the Claude Code CLI
        </Button>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex flex-col gap-3 items-start">
        <p className="text-sm text-[var(--ds-rec)]">{error}</p>
        <Button
          variant="ghost"
          size="sm"
          icon={<RotateCcw className="w-4 h-4" />}
          onClick={onRun}
        >
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
        <Button
          variant="primary"
          size="sm"
          icon={<Sparkles className="w-4 h-4" />}
          onClick={onRun}
        >
          Generate summary
        </Button>
      </div>
    );
  }

  // A translation only counts if it matches the currently-configured target;
  // switching the target in Settings invalidates a stale one.
  const label = translateTarget ? targetLabel(translateTarget) : "";
  const translation =
    summaryTranslation?.lang === translateTarget
      ? summaryTranslation?.text
      : undefined;
  const showingTranslation = view === "translated" && !!translateTarget;

  return (
    <div className="flex flex-col gap-4">
      {translateTarget && (
        <PillToggle
          aria-label="Summary language"
          options={[
            { id: "original", label: "Original" },
            { id: "translated", label },
          ]}
          value={view}
          onChange={(id) => setView(id as View)}
        />
      )}

      {showingTranslation && !translation ? (
        <div className="flex flex-col gap-3 items-start">
          {translateError && (
            <p className="text-sm text-[var(--ds-rec)]">{translateError}</p>
          )}
          {isTranslating ? (
            <p className="text-[var(--ds-text-2)] text-sm">
              Translating to {label}…
            </p>
          ) : (
            <Button
              variant="primary"
              size="sm"
              icon={<Languages className="w-4 h-4" />}
              onClick={onTranslate}
            >
              Translate to {label}
            </Button>
          )}
        </div>
      ) : (
        <Prose markdown={showingTranslation ? translation : summary} />
      )}

      {showingTranslation ? (
        translation &&
        !isTranslating && (
          <div className="flex items-center gap-2 border-t border-[var(--ds-border)] pt-3">
            <Button
              variant="ghost"
              size="sm"
              icon={<Languages className="w-4 h-4" />}
              onClick={onTranslate}
            >
              Retranslate
            </Button>
          </div>
        )
      ) : (
        <div className="flex items-center gap-2 border-t border-[var(--ds-border)] pt-3">
          <Button
            variant="ghost"
            size="sm"
            icon={<RefreshCw className="w-4 h-4" />}
            onClick={onRun}
          >
            Regenerate
          </Button>
        </div>
      )}
    </div>
  );
};
