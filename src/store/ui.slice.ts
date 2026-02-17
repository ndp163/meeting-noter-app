import { StateCreator } from "zustand";
import { immer } from "zustand/middleware/immer";

export interface UISlice {
  isSidebarCollapsed: boolean;

  // Actions
  toggleSidebar: () => void;
  setSidebarCollapsed: (collapsed: boolean) => void;
}

export const createUISlice: StateCreator<UISlice, [], [], UISlice> = immer(
  (set) => ({
    isSidebarCollapsed: false,

    toggleSidebar: () =>
      set((state) => {
        state.isSidebarCollapsed = !state.isSidebarCollapsed;
      }),

    setSidebarCollapsed: (collapsed) =>
      set((state) => {
        state.isSidebarCollapsed = collapsed;
      }),
  }),
) as StateCreator<UISlice, [], [], UISlice>;
