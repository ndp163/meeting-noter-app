import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

/** Which engine produces summaries. `claude` = Claude Code CLI (needs network +
 *  login); `local` = on-device MLX model (fully offline, downloaded on demand). */
export type SummaryProvider = "claude" | "local";

/** What the summary is built from: the raw live transcript, or the diarized
 *  speaker-labelled transcript (accurate per-person attribution). */
export type SummarySource = "transcript" | "speakers";

export const summarizeMeeting = async (
  meetingId: string,
  source?: SummarySource,
): Promise<string> => {
  return await invoke<string>("summarize_meeting", {
    meetingId,
    source: source ?? null,
  });
};

export const translateSummary = async (
  meetingId: string,
  target: string,
): Promise<string> => {
  return await invoke<string>("translate_summary", { meetingId, target });
};

export const generateTitle = async (meetingId: string): Promise<string> => {
  return await invoke<string>("generate_title", { meetingId });
};

export const isClaudeAvailable = async (): Promise<boolean> => {
  return await invoke<boolean>("claude_available");
};

/** The persisted summary provider (defaults to `claude`). */
export const getSummaryProvider = (): Promise<SummaryProvider> =>
  invoke<SummaryProvider>("get_summary_provider");

/** Persist the summary provider choice. */
export const setSummaryProvider = (provider: SummaryProvider): Promise<void> =>
  invoke("set_summary_provider", { provider });

/** Whether the local MLX model (dylib + metallib + weights) is installed. */
export const isLocalModelAvailable = (): Promise<boolean> =>
  invoke<boolean>("local_model_available");

/** Download the local model set; rejects on failure so the UI can retry. */
export const downloadLocalModel = (): Promise<void> =>
  invoke("download_local_model");

/** Remove the local model to reclaim disk. */
export const deleteLocalModel = (): Promise<void> =>
  invoke("delete_local_model");

/** Subscribe to local-model download progress (fraction 0.0–1.0). Returns an
 *  unlisten fn. */
export const onLocalModelProgress = (handler: (fraction: number) => void) =>
  listen<{ fraction: number }>("mlx://progress", (e) =>
    handler(e.payload.fraction),
  );
