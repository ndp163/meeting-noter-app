import { StateCreator } from "zustand";
import { immer } from "zustand/middleware/immer";

export type ToastTone = "error" | "success" | "info";

export interface Toast {
  id: string;
  message: string;
  tone: ToastTone;
}

export interface ToastSlice {
  toasts: Toast[];
  /** Show a toast. Returns its id. */
  pushToast: (message: string, tone?: ToastTone) => string;
  dismissToast: (id: string) => void;
}

const newId = () =>
  typeof crypto !== "undefined" && "randomUUID" in crypto
    ? crypto.randomUUID()
    : `${Date.now()}-${Math.round(Math.random() * 1e6)}`;

export const createToastSlice: StateCreator<ToastSlice, [], [], ToastSlice> =
  immer((set) => ({
    toasts: [],

    pushToast: (message, tone = "info") => {
      const id = newId();
      set((state) => {
        state.toasts.push({ id, message, tone });
      });
      return id;
    },

    dismissToast: (id) =>
      set((state) => {
        state.toasts = state.toasts.filter((t) => t.id !== id);
      }),
  })) as StateCreator<ToastSlice, [], [], ToastSlice>;
