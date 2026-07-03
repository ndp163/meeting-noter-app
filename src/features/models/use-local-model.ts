import { useCallback, useEffect, useState } from "react";
import { friendlyError } from "@/lib/utils";
import {
  deleteLocalModel,
  downloadLocalModel,
  getSummaryProvider,
  isLocalModelAvailable,
  onLocalModelProgress,
  setSummaryProvider as persistProvider,
  type SummaryProvider,
} from "@/services/summary";

/**
 * State for the on-device summary model: the selected provider (persisted in
 * the backend), whether the model is installed, and live download progress.
 * Listens to `mlx://progress` once while a download is in flight.
 */
export const useLocalModel = () => {
  const [provider, setProviderState] = useState<SummaryProvider>("claude");
  const [installed, setInstalled] = useState<boolean | null>(null);
  const [downloading, setDownloading] = useState(false);
  const [progress, setProgress] = useState<number>();
  const [error, setError] = useState<string>();

  useEffect(() => {
    void getSummaryProvider().then(setProviderState);
    void isLocalModelAvailable().then(setInstalled);
  }, []);

  useEffect(() => {
    const unlisten = onLocalModelProgress(setProgress);
    return () => {
      void unlisten.then((u) => u());
    };
  }, []);

  const setProvider = useCallback(async (next: SummaryProvider) => {
    setProviderState(next);
    await persistProvider(next);
  }, []);

  const download = useCallback(async (): Promise<boolean> => {
    setError(undefined);
    setProgress(0);
    setDownloading(true);
    try {
      await downloadLocalModel();
      setInstalled(true);
      return true;
    } catch (e) {
      console.error("Local model download failed", e);
      setError(friendlyError(e, "Download failed — please try again."));
      return false;
    } finally {
      setDownloading(false);
      setProgress(undefined);
    }
  }, []);

  const remove = useCallback(async () => {
    await deleteLocalModel();
    setInstalled(false);
  }, []);

  return {
    provider,
    setProvider,
    installed,
    downloading,
    progress,
    error,
    download,
    remove,
  };
};
