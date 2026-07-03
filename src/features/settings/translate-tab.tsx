import { useCallback, useEffect, useState } from "react";
import { Check, ChevronDown, Download, Languages, Loader2 } from "lucide-react";
import { Button, PillToggle } from "@/design-system";
import { cn, friendlyError } from "@/lib/utils";
import {
  getTranslateConfig,
  setTranslateConfig,
  translateStatus,
  translateDownload,
  TRANSLATE_TARGETS as TARGETS,
  type TranslateAvailability,
  type TranscriptView,
} from "@/services/translation";
import { installedLanguages } from "@/services/setup";
import { useBoundStore } from "@/store";
import type { MeetingLanguage } from "@/types/meeting";

const SOURCE_LABEL: Record<string, string> = { en: "English", ja: "Japanese" };

/**
 * Realtime translation settings. A master on/off card; the configuration below
 * dims and disables when it's off. Everything runs on-device.
 */
export const TranslateTab = () => {
  const [enabled, setEnabled] = useState(false);
  const [target, setTarget] = useState("vi");
  const [view, setView] = useState<TranscriptView>("both");
  const [sources, setSources] = useState<MeetingLanguage[]>([]);
  const setTranscriptView = useBoundStore.use.setTranscriptView();
  const setSettingsTab = useBoundStore.use.setSettingsTab();

  useEffect(() => {
    getTranslateConfig()
      .then((c) => {
        setEnabled(c.enabled);
        if (c.target) setTarget(c.target);
        setView(c.view);
      })
      .catch(() => {});
    installedLanguages()
      .then(setSources)
      .catch(() => {});
  }, []);

  const save = useCallback(
    (next: { enabled?: boolean; target?: string; view?: TranscriptView }) => {
      const e = next.enabled ?? enabled;
      const t = next.target ?? target;
      const v = next.view ?? view;
      setEnabled(e);
      setTarget(t);
      setView(v);
      if (next.view) setTranscriptView(v);
      void setTranslateConfig(e, t, v);
    },
    [enabled, target, view, setTranscriptView],
  );

  const applicableSources = sources.filter((s) => s !== target);

  return (
    <div className="flex flex-col gap-4">
      {/* Master toggle */}
      <div className="ds-card ds-card--flat flex items-center gap-3 p-4">
        <span
          className="flex items-center justify-center w-9 h-9 rounded-full shrink-0"
          style={{ background: "var(--ds-surface-2)", color: "var(--ds-accent)" }}
        >
          <Languages className="w-4 h-4" />
        </span>
        <div className="flex flex-col min-w-0 flex-1">
          <span className="text-sm font-medium text-[var(--ds-text)]">
            Realtime translation
          </span>
          <span className="text-xs text-[var(--ds-text-3)]">
            Translate the live transcript on-device. Nothing leaves your machine.
          </span>
        </div>
        <PillToggle
          options={[
            { id: "on", label: "On" },
            { id: "off", label: "Off" },
          ]}
          value={enabled ? "on" : "off"}
          onChange={(id) => save({ enabled: id === "on" })}
        />
      </div>

      {/* Configuration — dimmed + inert when translation is off. `inert` blocks
          keyboard reach too (pointer-events alone leaves controls tabbable). */}
      <div
        aria-disabled={!enabled}
        inert={!enabled}
        className={cn(
          "flex flex-col gap-5 transition-opacity",
          !enabled && "opacity-45",
        )}
      >
        <Field label="Translate into">
          <div className="relative">
            <select
              value={target}
              onChange={(e) => save({ target: e.target.value })}
              disabled={!enabled}
              className="appearance-none text-sm rounded-[var(--ds-radius)] border pl-3 pr-8 py-1.5 min-w-[160px] cursor-pointer"
              style={{
                background: "var(--ds-surface)",
                color: "var(--ds-text)",
                borderColor: "var(--ds-border)",
              }}
            >
              {TARGETS.map((t) => (
                <option key={t.id} value={t.id}>
                  {t.label}
                </option>
              ))}
            </select>
            <ChevronDown className="absolute right-2 top-1/2 -translate-y-1/2 w-4 h-4 pointer-events-none text-[var(--ds-text-3)]" />
          </div>
        </Field>

        <Field label="Show in transcript">
          <PillToggle
            options={[
              { id: "translated", label: "Translated" },
              { id: "both", label: "Both (bilingual)" },
            ]}
            value={view}
            onChange={(id) => save({ view: id as TranscriptView })}
          />
        </Field>

        <div className="flex flex-col gap-2">
          <span className="text-sm font-medium text-[var(--ds-text)]">
            Language packs
          </span>
          <div className="ds-card ds-card--flat flex flex-col divide-y divide-[var(--ds-border)]">
            {applicableSources.length === 0 ? (
              <div className="flex flex-col items-start gap-2 px-3 py-2.5">
                <span className="text-xs text-[var(--ds-text-3)]">
                  No applicable transcription languages installed.
                </span>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => setSettingsTab("models")}
                >
                  Manage languages
                </Button>
              </div>
            ) : (
              applicableSources.map((src) => (
                <PackRow key={src} source={src} target={target} />
              ))
            )}
          </div>
        </div>
      </div>
    </div>
  );
};

/** Label + control row with consistent spacing. */
const Field = ({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) => (
  <div className="flex items-center justify-between gap-3">
    <span className="text-sm font-medium text-[var(--ds-text)]">{label}</span>
    {children}
  </div>
);

interface PackRowProps {
  source: MeetingLanguage;
  target: string;
}

/** One source→target pack: status + a download button when needed. */
const PackRow = ({ source, target }: PackRowProps) => {
  const [status, setStatus] = useState<TranslateAvailability | "loading">("loading");
  const [downloading, setDownloading] = useState(false);
  const [error, setError] = useState<string>();

  const refresh = useCallback(() => {
    translateStatus(source, target)
      .then(setStatus)
      .catch((e) => {
        console.error("Translate status check failed", source, target, e);
        setStatus("unsupported");
        setError(friendlyError(e, "Couldn't check this language pack."));
      });
  }, [source, target]);

  useEffect(refresh, [refresh]);

  const targetLabel = TARGETS.find((t) => t.id === target)?.label ?? target;
  const pairLabel = `${SOURCE_LABEL[source] ?? source} → ${targetLabel}`;

  const download = async () => {
    setDownloading(true);
    setError(undefined);
    try {
      await translateDownload(source, target);
      refresh();
    } catch (e) {
      console.error("Language pack download failed", source, target, e);
      setError(friendlyError(e, "Download failed — please try again."));
    } finally {
      setDownloading(false);
    }
  };

  return (
    <div className="flex items-center justify-between gap-3 px-3 py-2.5">
      <div className="flex flex-col min-w-0">
        <span className="text-sm text-[var(--ds-text)]">{pairLabel}</span>
        {error && <span className="text-xs text-[var(--ds-rec)]">{error}</span>}
      </div>
      {status === "loading" ? (
        <Loader2 className="w-3.5 h-3.5 animate-spin text-[var(--ds-text-3)]" />
      ) : status === "installed" ? (
        <span className="flex items-center gap-1.5 text-xs text-[var(--ds-ok)]">
          <Check className="w-3.5 h-3.5" />
          Ready
        </span>
      ) : status === "supported" ? (
        <Button
          variant="primary"
          size="sm"
          icon={
            downloading ? (
              <Loader2 className="w-4 h-4 animate-spin" />
            ) : (
              <Download className="w-4 h-4" />
            )
          }
          onClick={() => void download()}
          disabled={downloading}
        >
          {downloading ? "Downloading…" : "Download"}
        </Button>
      ) : (
        <span className="text-xs text-[var(--ds-text-3)]">Unsupported</span>
      )}
    </div>
  );
};
