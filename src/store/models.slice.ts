import { StateCreator } from "zustand";
import { immer } from "zustand/middleware/immer";
import { MeetingLanguage } from "@/types/meeting";
import { installedLanguages } from "@/services/setup";

export interface ModelsSlice {
  /** Languages whose models are installed on disk. `null` until first load. */
  installedLanguages: MeetingLanguage[] | null;

  // Actions
  refreshInstalledLanguages: () => Promise<void>;
  isLanguageInstalled: (language: MeetingLanguage) => boolean;
}

export const createModelsSlice: StateCreator<ModelsSlice, [], [], ModelsSlice> =
  immer((set, get) => ({
    installedLanguages: null,

    refreshInstalledLanguages: async () => {
      const langs = await installedLanguages();
      set((state) => {
        state.installedLanguages = langs;
      });
    },

    isLanguageInstalled: (language) =>
      get().installedLanguages?.includes(language) ?? false,
  })) as StateCreator<ModelsSlice, [], [], ModelsSlice>;
