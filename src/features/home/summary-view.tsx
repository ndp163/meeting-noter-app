import { useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { cn } from "@/lib/utils";

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
      <div className="text-custom-text-secondary text-sm">
        Generating summary with Claude…
      </div>
    );
  }

  if (claudeReady === false) {
    return (
      <div className="text-custom-text-secondary text-sm">
        AI summaries use the Claude Code CLI. Install Claude Code and log in,
        then reopen this tab.
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex flex-col gap-3">
        <p className="text-sm text-custom-red">{error}</p>
        <button
          onClick={onRun}
          className="self-start rounded-full border border-custom-text-primary px-4 py-1.5 text-sm text-custom-text-primary hover:bg-custom-bg-secondary"
        >
          Try again
        </button>
      </div>
    );
  }

  if (!summary) {
    if (!canRun) {
      return (
        <div className="text-custom-text-secondary text-sm">
          A summary is available after a recording finishes.
        </div>
      );
    }
    return (
      <div className="flex flex-col gap-3">
        <p className="text-sm text-custom-text-secondary">
          Generate a TL;DR, key points, decisions, and action items from this
          meeting's transcript using Claude.
        </p>
        <button
          onClick={onRun}
          className="self-start rounded-full border border-custom-text-primary px-4 py-1.5 text-sm text-custom-text-primary hover:bg-custom-bg-secondary"
        >
          Generate summary
        </button>
      </div>
    );
  }

  const shown = lang === "vi" ? summaryVi : summary;

  return (
    <div className="flex flex-col gap-4">
      {/* Language toggle */}
      <div className="flex items-center gap-2">
        <LangButton
          label="Original"
          active={lang === "en"}
          onClick={() => setLang("en")}
        />
        <LangButton
          label="Tiếng Việt"
          active={lang === "vi"}
          onClick={() => setLang("vi")}
        />
      </div>

      {lang === "vi" && !summaryVi ? (
        <div className="flex flex-col gap-3">
          {translateError && (
            <p className="text-sm text-custom-red">{translateError}</p>
          )}
          {isTranslating ? (
            <p className="text-custom-text-secondary text-sm">
              Đang dịch sang tiếng Việt…
            </p>
          ) : (
            <button
              onClick={onTranslate}
              className="self-start rounded-full border border-custom-text-primary px-4 py-1.5 text-sm text-custom-text-primary hover:bg-custom-bg-secondary"
            >
              Dịch sang tiếng Việt
            </button>
          )}
        </div>
      ) : (
        <div className="prose prose-sm max-w-none text-custom-text-primary">
          <ReactMarkdown remarkPlugins={[remarkGfm]}>{shown}</ReactMarkdown>
        </div>
      )}

      <div className="flex items-center gap-2">
        <button
          onClick={onRun}
          className="self-start rounded-full border border-custom-text-primary px-4 py-1.5 text-sm text-custom-text-primary hover:bg-custom-bg-secondary"
        >
          Regenerate
        </button>
        {lang === "vi" && summaryVi && !isTranslating && (
          <button
            onClick={onTranslate}
            className="self-start rounded-full border border-custom-text-primary px-4 py-1.5 text-sm text-custom-text-primary hover:bg-custom-bg-secondary"
          >
            Dịch lại
          </button>
        )}
      </div>
    </div>
  );
};

const LangButton = ({
  label,
  active,
  onClick,
}: {
  label: string;
  active: boolean;
  onClick: () => void;
}) => (
  <button
    onClick={onClick}
    className={cn(
      "rounded-full px-3 py-1 text-sm",
      active
        ? "bg-custom-text-primary text-white"
        : "text-custom-text-secondary hover:bg-custom-bg-secondary"
    )}
  >
    {label}
  </button>
);
