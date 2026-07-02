import { useEffect, useState, type ReactNode } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { AlertCircle, Check, Download, Loader2, Trash2 } from "lucide-react";
import { Button } from "@/design-system";
import { cn } from "@/lib/utils";
import { DownloadIndicator } from "@/features/models/download-indicator";
import { useLocalModel } from "@/features/models/use-local-model";
import { isClaudeAvailable } from "@/services/summary";

const CLAUDE_CODE_URL = "https://docs.claude.com/en/docs/claude-code/setup";

/**
 * Picks the AI-summary engine. Two radio cards: the Claude Code CLI (higher
 * quality, needs network + login) and a fully-offline on-device model that
 * downloads on demand. Each card shows its own readiness and actions.
 */
export const SummaryTab = () => {
  const { provider, setProvider, installed, downloading, progress, error, download, remove } =
    useLocalModel();
  const [claudeReady, setClaudeReady] = useState<boolean>();

  useEffect(() => {
    isClaudeAvailable()
      .then(setClaudeReady)
      .catch(() => setClaudeReady(false));
  }, []);

  return (
    <div className="flex flex-col gap-3 min-w-[420px]">
      <p className="text-sm text-[var(--ds-text-2)]">
        Choose how meeting summaries are generated. Either way your transcript
        stays on this machine.
      </p>

      <ProviderCard
        selected={provider === "claude"}
        onSelect={() => void setProvider("claude")}
        title="Claude Code CLI"
        subtitle="Higher quality · needs the claude CLI installed and logged in"
      >
        {claudeReady === false ? (
          <div className="flex flex-col gap-1.5 items-start">
            <span className="flex items-center gap-1.5 text-xs text-[var(--ds-text-3)]">
              <AlertCircle className="w-3.5 h-3.5" />
              The <code className="text-[var(--ds-text-2)]">claude</code> CLI
              isn&apos;t installed or logged in.
            </span>
            <button
              type="button"
              className="underline text-xs text-[var(--ds-text)] hover:text-[var(--ds-accent)]"
              onClick={() => void openUrl(CLAUDE_CODE_URL)}
            >
              Install the Claude Code CLI
            </button>
          </div>
        ) : claudeReady ? (
          <span className="flex items-center gap-1.5 text-xs text-[var(--ds-ok)]">
            <Check className="w-3.5 h-3.5" />
            Installed &amp; ready
          </span>
        ) : null}
      </ProviderCard>

      <ProviderCard
        selected={provider === "local"}
        onSelect={() => void setProvider("local")}
        title="On-device model"
        subtitle="Fully offline · no network or login · ~2.3 GB download"
      >
        <div className="flex items-center gap-2">
          {downloading ? (
            <span className="flex items-center gap-1.5 text-xs text-[var(--ds-text-2)]">
              <Loader2 className="w-3.5 h-3.5 animate-spin" />
              Downloading…
            </span>
          ) : installed ? (
            <>
              <span className="flex items-center gap-1.5 text-xs text-[var(--ds-ok)]">
                <Check className="w-3.5 h-3.5" />
                Installed
              </span>
              <Button
                variant="ghost"
                size="sm"
                icon={<Trash2 className="w-4 h-4" />}
                onClick={() => void remove()}
                aria-label="Remove on-device model"
              >
                Remove
              </Button>
            </>
          ) : (
            <Button
              variant="primary"
              size="sm"
              icon={<Download className="w-4 h-4" />}
              onClick={() => void download()}
            >
              Download
            </Button>
          )}
        </div>

        {downloading && <DownloadIndicator value={progress} />}
        {error && <p className="text-xs text-[var(--ds-rec)]">{error}</p>}
        {installed && !downloading && (
          <p className="text-xs text-[var(--ds-text-3)]">
            Lower quality than Claude; Vietnamese translation is roughest on this
            model.
          </p>
        )}
      </ProviderCard>
    </div>
  );
};

interface ProviderCardProps {
  selected: boolean;
  onSelect: () => void;
  title: string;
  subtitle: string;
  children?: ReactNode;
}

/** A selectable radio card. Body (status/actions) shows under the label. */
const ProviderCard = ({ selected, onSelect, title, subtitle, children }: ProviderCardProps) => (
  <div
    role="radio"
    aria-checked={selected}
    tabIndex={0}
    onClick={onSelect}
    onKeyDown={(e) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        onSelect();
      }
    }}
    className={cn(
      "ds-card ds-card--flat flex flex-col gap-2 p-3 cursor-pointer transition-colors",
      !selected && "hover:bg-[var(--ds-surface-2)]",
    )}
    style={selected ? { borderColor: "var(--ds-accent)" } : undefined}
  >
    <div className="flex items-start gap-3">
      <span
        className="mt-0.5 w-4 h-4 rounded-full border flex items-center justify-center shrink-0"
        style={{ borderColor: selected ? "var(--ds-accent)" : "var(--ds-border)" }}
      >
        {selected && (
          <span
            className="w-2 h-2 rounded-full"
            style={{ background: "var(--ds-accent)" }}
          />
        )}
      </span>
      <div className="flex flex-col min-w-0 flex-1 gap-0.5">
        <span className="text-sm font-medium text-[var(--ds-text)]">{title}</span>
        <span className="text-xs text-[var(--ds-text-3)]">{subtitle}</span>
      </div>
    </div>
    {children && <div className="pl-7">{children}</div>}
  </div>
);
