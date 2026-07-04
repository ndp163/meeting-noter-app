import { invoke } from "@tauri-apps/api/core";

/**
 * Live-caption overlay window controls. The window is a standalone webview
 * (label `caption`) that subscribes to `transcription://chunk` itself — see
 * `src/features/caption/caption-window.tsx`. These wrappers only drive its
 * lifecycle; no transcript data passes through them.
 */

/** Create (or re-show) the always-on-top caption overlay. */
export const showCaption = async (): Promise<void> => {
  await invoke("show_caption_window");
};

/** Hide the overlay without destroying it. */
export const hideCaption = async (): Promise<void> => {
  await invoke("hide_caption_window");
};

/**
 * Toggle input pass-through. `true` lets clicks fall through to the meeting app
 * behind the overlay; set `false` while the pointer is over the caption so its
 * own controls remain clickable.
 */
export const setCaptionClickThrough = async (ignore: boolean): Promise<void> => {
  await invoke("set_caption_click_through", { ignore });
};
