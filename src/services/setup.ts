import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type SetupStage =
  | "preparing-transcription"
  | "preparing-speaker"
  | "ready";

const SETUP_STAGE_EVENT = "setup://stage";

/** Whether all speech models are already cached (skip onboarding if true). */
export const modelsReady = (): Promise<boolean> => invoke<boolean>("models_ready");

/** Download both model sets; rejects on failure so the UI can retry. */
export const prefetchModels = (): Promise<void> => invoke("prefetch_models");

/** Subscribe to coarse download stage updates. Returns an unlisten fn. */
export const onSetupStage = (handler: (stage: SetupStage) => void) =>
  listen<{ stage: SetupStage }>(SETUP_STAGE_EVENT, (e) =>
    handler(e.payload.stage),
  );
