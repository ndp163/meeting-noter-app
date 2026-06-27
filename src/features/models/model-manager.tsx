import { Check, Download, Loader2, Trash2 } from "lucide-react";
import { Button } from "@/design-system";
import { LANGUAGES } from "@/lib/languages";
import type { MeetingLanguage } from "@/types/meeting";
import { useModelManager } from "./use-model-manager";
import { DownloadIndicator } from "./download-indicator";

/**
 * In-app language/model list: shows each language's install state and lets the
 * user download or remove it. Used inside the settings modal.
 */
export const ModelManager = () => {
  const { errors, download, remove, isInstalled, isDownloading } =
    useModelManager();

  return (
    <div className="flex flex-col gap-2">
      {LANGUAGES.map((lang) => (
        <ModelRow
          key={lang.id}
          id={lang.id}
          label={lang.label}
          native={lang.native}
          approxSize={lang.approxSize}
          installed={isInstalled(lang.id)}
          downloading={isDownloading(lang.id)}
          error={errors[lang.id]}
          onDownload={() => download(lang.id)}
          onRemove={() => remove(lang.id)}
        />
      ))}
      <p className="text-xs text-[var(--ds-text-3)] mt-1">
        Models run fully offline. The shared speaker model (~13 MB) downloads
        once with your first language.
      </p>
    </div>
  );
};

interface ModelRowProps {
  id: MeetingLanguage;
  label: string;
  native: string;
  approxSize: string;
  installed: boolean;
  downloading: boolean;
  error?: string;
  onDownload: () => void;
  onRemove: () => void;
}

const ModelRow = ({
  label,
  native,
  approxSize,
  installed,
  downloading,
  error,
  onDownload,
  onRemove,
}: ModelRowProps) => (
  <div className="ds-card ds-card--flat flex flex-col gap-2 p-3">
    <div className="flex items-center gap-3">
      <div className="flex flex-col min-w-0 flex-1">
        <span className="text-sm font-medium text-[var(--ds-text)]">
          {label}
          {native !== label && (
            <span className="text-[var(--ds-text-3)] font-normal"> · {native}</span>
          )}
        </span>
        <span className="text-xs text-[var(--ds-text-3)]">
          {installed ? "Installed" : approxSize}
        </span>
      </div>

      {downloading ? (
        <span className="flex items-center gap-1.5 text-xs text-[var(--ds-text-2)]">
          <Loader2 className="w-3.5 h-3.5 animate-spin" />
          Downloading…
        </span>
      ) : installed ? (
        <div className="flex items-center gap-1.5">
          <span className="flex items-center gap-1 text-xs text-[var(--ds-ok)]">
            <Check className="w-4 h-4" />
          </span>
          <Button
            variant="ghost"
            size="sm"
            icon={<Trash2 className="w-4 h-4" />}
            onClick={onRemove}
            aria-label={`Remove ${label} model`}
          >
            Remove
          </Button>
        </div>
      ) : (
        <Button
          variant="primary"
          size="sm"
          icon={<Download className="w-4 h-4" />}
          onClick={onDownload}
        >
          Download
        </Button>
      )}
    </div>

    {downloading && <DownloadIndicator />}
    {error && <p className="text-xs text-[var(--ds-rec)]">{error}</p>}
  </div>
);
