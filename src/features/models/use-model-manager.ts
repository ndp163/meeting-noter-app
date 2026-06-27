import { useCallback, useEffect, useState } from "react";
import { useBoundStore } from "@/store";
import {
  deleteLanguage,
  downloadLanguage,
  onSetupProgress,
} from "@/services/setup";
import type { MeetingLanguage } from "@/types/meeting";

type LangMap<T> = Partial<Record<MeetingLanguage, T>>;

/**
 * Shared model-download state for the onboarding screen and the Models panel.
 * Tracks installed languages (via the store), live per-language download
 * fractions, in-flight downloads, and errors. Listens to `setup://progress`
 * once and routes each event to its language.
 */
export const useModelManager = () => {
  const installed = useBoundStore.use.installedLanguages();
  const refresh = useBoundStore.use.refreshInstalledLanguages();

  const [progress, setProgress] = useState<LangMap<number>>({});
  const [downloading, setDownloading] = useState<MeetingLanguage[]>([]);
  const [errors, setErrors] = useState<LangMap<string>>({});

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    const unlisten = onSetupProgress(({ language, fraction }) =>
      setProgress((p) => ({ ...p, [language]: fraction })),
    );
    return () => {
      void unlisten.then((u) => u());
    };
  }, []);

  /** Download one language. Resolves true on success, false on failure (the
   *  error is also surfaced via `errors`). */
  const download = useCallback(
    async (language: MeetingLanguage): Promise<boolean> => {
      setErrors((e) => ({ ...e, [language]: undefined }));
      setProgress((p) => ({ ...p, [language]: 0 }));
      setDownloading((d) => (d.includes(language) ? d : [...d, language]));
      try {
        await downloadLanguage(language);
        await refresh();
        return true;
      } catch (e) {
        setErrors((er) => ({ ...er, [language]: String(e) }));
        return false;
      } finally {
        setDownloading((d) => d.filter((l) => l !== language));
        setProgress((p) => ({ ...p, [language]: undefined }));
      }
    },
    [refresh],
  );

  const remove = useCallback(
    async (language: MeetingLanguage) => {
      await deleteLanguage(language);
      await refresh();
    },
    [refresh],
  );

  return {
    /** Installed languages, or null until the first load resolves. */
    installed,
    progress,
    downloading,
    errors,
    download,
    remove,
    isInstalled: (l: MeetingLanguage) => installed?.includes(l) ?? false,
    isDownloading: (l: MeetingLanguage) => downloading.includes(l),
  };
};
