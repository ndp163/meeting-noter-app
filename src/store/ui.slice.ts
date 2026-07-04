import { StateCreator } from "zustand";
import { immer } from "zustand/middleware/immer";
import { MeetingLanguage } from "@/types/meeting";

export type SettingsTab =
  | "models"
  | "summary"
  | "translate"
  | "captions"
  | "updates";

/** Expanded-sidebar width bounds (px). Below MIN a drag snaps to collapsed. */
export const SIDEBAR_MIN_WIDTH = 240;
export const SIDEBAR_MAX_WIDTH = 480;
export const SIDEBAR_DEFAULT_WIDTH = 320;
/** Drag narrower than this (px) collapses the sidebar to the icon rail. */
export const SIDEBAR_COLLAPSE_AT = 200;

const WIDTH_KEY = "sidebarWidth";
const COLLAPSED_KEY = "isSidebarCollapsed";
const CAPTION_KEY = "captionEnabled";

const clampWidth = (w: number) =>
  Math.min(SIDEBAR_MAX_WIDTH, Math.max(SIDEBAR_MIN_WIDTH, w));

const readWidth = () => {
  const stored = Number(localStorage.getItem(WIDTH_KEY));
  return stored ? clampWidth(stored) : SIDEBAR_DEFAULT_WIDTH;
};
const readCollapsed = () => localStorage.getItem(COLLAPSED_KEY) === "true";
const readCaptionEnabled = () => localStorage.getItem(CAPTION_KEY) === "true";

/** How translated transcript lines are shown. Set in Settings › Translation. */
export type TranscriptView = "translated" | "both";

export interface UISlice {
  isSidebarCollapsed: boolean;
  /** Width (px) of the expanded sidebar; persisted across restarts. */
  sidebarWidth: number;
  /** True while a recording is in progress. Mirrored from HomePage so any
   *  surface (e.g. the update banner) can guard against interrupting it. */
  isRecording: boolean;
  /** Language applied to the next recording started. */
  captureLanguage: MeetingLanguage;
  /** Settings modal visibility + active tab. */
  settingsOpen: boolean;
  settingsTab: SettingsTab;
  /** Translation display mode (mirrors the persisted setting). */
  transcriptView: TranscriptView;
  /** Whether the live-caption overlay shows during recording; persisted. */
  captionEnabled: boolean;

  // Actions
  toggleSidebar: () => void;
  setSidebarCollapsed: (collapsed: boolean) => void;
  setSidebarWidth: (width: number) => void;
  setRecording: (recording: boolean) => void;
  setCaptureLanguage: (language: MeetingLanguage) => void;
  openSettings: (tab?: SettingsTab) => void;
  closeSettings: () => void;
  setSettingsTab: (tab: SettingsTab) => void;
  setTranscriptView: (view: TranscriptView) => void;
  setCaptionEnabled: (enabled: boolean) => void;
}

export const createUISlice: StateCreator<UISlice, [], [], UISlice> = immer(
  (set) => ({
    isSidebarCollapsed: readCollapsed(),
    sidebarWidth: readWidth(),
    isRecording: false,
    captureLanguage: "en",
    settingsOpen: false,
    settingsTab: "models",
    transcriptView: "both",
    captionEnabled: readCaptionEnabled(),

    toggleSidebar: () =>
      set((state) => {
        state.isSidebarCollapsed = !state.isSidebarCollapsed;
        localStorage.setItem(COLLAPSED_KEY, String(state.isSidebarCollapsed));
      }),

    setSidebarCollapsed: (collapsed) =>
      set((state) => {
        state.isSidebarCollapsed = collapsed;
        localStorage.setItem(COLLAPSED_KEY, String(collapsed));
      }),

    setSidebarWidth: (width) =>
      set((state) => {
        state.sidebarWidth = clampWidth(width);
        localStorage.setItem(WIDTH_KEY, String(state.sidebarWidth));
      }),

    setRecording: (recording) =>
      set((state) => {
        state.isRecording = recording;
      }),

    setCaptureLanguage: (language) =>
      set((state) => {
        state.captureLanguage = language;
      }),

    openSettings: (tab) =>
      set((state) => {
        state.settingsOpen = true;
        if (tab) state.settingsTab = tab;
      }),

    closeSettings: () =>
      set((state) => {
        state.settingsOpen = false;
      }),

    setSettingsTab: (tab) =>
      set((state) => {
        state.settingsTab = tab;
      }),

    setTranscriptView: (view) =>
      set((state) => {
        state.transcriptView = view;
      }),

    setCaptionEnabled: (enabled) =>
      set((state) => {
        state.captionEnabled = enabled;
        localStorage.setItem(CAPTION_KEY, String(enabled));
      }),
  }),
) as StateCreator<UISlice, [], [], UISlice>;
