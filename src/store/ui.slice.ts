import { StateCreator } from "zustand";
import { immer } from "zustand/middleware/immer";
import { MeetingLanguage } from "@/types/meeting";

export type SettingsTab = "models" | "summary" | "updates";

export interface UISlice {
  isSidebarCollapsed: boolean;
  /** Language applied to the next recording started. */
  captureLanguage: MeetingLanguage;
  /** Settings modal visibility + active tab. */
  settingsOpen: boolean;
  settingsTab: SettingsTab;

  // Actions
  toggleSidebar: () => void;
  setSidebarCollapsed: (collapsed: boolean) => void;
  setCaptureLanguage: (language: MeetingLanguage) => void;
  openSettings: (tab?: SettingsTab) => void;
  closeSettings: () => void;
  setSettingsTab: (tab: SettingsTab) => void;
}

export const createUISlice: StateCreator<UISlice, [], [], UISlice> = immer(
  (set) => ({
    isSidebarCollapsed: false,
    captureLanguage: "en",
    settingsOpen: false,
    settingsTab: "models",

    toggleSidebar: () =>
      set((state) => {
        state.isSidebarCollapsed = !state.isSidebarCollapsed;
      }),

    setSidebarCollapsed: (collapsed) =>
      set((state) => {
        state.isSidebarCollapsed = collapsed;
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
  }),
) as StateCreator<UISlice, [], [], UISlice>;
