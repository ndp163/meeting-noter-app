import { StateCreator } from "zustand";
import { immer } from "zustand/middleware/immer";
import { MeetingLanguage } from "@/types/meeting";

export interface UISlice {
  isSidebarCollapsed: boolean;
  /** Language applied to the next recording started. */
  captureLanguage: MeetingLanguage;

  // Actions
  toggleSidebar: () => void;
  setSidebarCollapsed: (collapsed: boolean) => void;
  setCaptureLanguage: (language: MeetingLanguage) => void;
}

export const createUISlice: StateCreator<UISlice, [], [], UISlice> = immer(
  (set) => ({
    isSidebarCollapsed: false,
    captureLanguage: "en",

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
  }),
) as StateCreator<UISlice, [], [], UISlice>;
