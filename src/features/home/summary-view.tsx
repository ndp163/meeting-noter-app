import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

interface SummaryViewProps {
  summary: string | undefined;
  isLoading: boolean;
  error?: string;
  canRun: boolean;
  claudeReady: boolean | undefined;
  onRun: () => void;
}

export const SummaryView = ({
  summary,
  isLoading,
  error,
  canRun,
  claudeReady,
  onRun,
}: SummaryViewProps) => {
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

  return (
    <div className="flex flex-col gap-4">
      <div className="prose prose-sm max-w-none text-custom-text-primary">
        <ReactMarkdown remarkPlugins={[remarkGfm]}>{summary}</ReactMarkdown>
      </div>
      <button
        onClick={onRun}
        className="self-start rounded-full border border-custom-text-primary px-4 py-1.5 text-sm text-custom-text-primary hover:bg-custom-bg-secondary"
      >
        Regenerate
      </button>
    </div>
  );
};
