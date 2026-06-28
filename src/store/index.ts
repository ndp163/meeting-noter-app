import { create, StateCreator, StoreApi, UseBoundStore } from "zustand";
import { createTeamsSlice, TeamsSlice } from "./teams.slice";
import { createMeetingsSlice, MeetingsSlice } from "./meetings.slice";
import { createUISlice, UISlice } from "./ui.slice";
import { createModelsSlice, ModelsSlice } from "./models.slice";
import { createUpdateSlice, UpdateSlice } from "./update.slice";
import { immer } from "zustand/middleware/immer";

type WithSelectors<S> = S extends { getState: () => infer T }
  ? S & { use: { [K in keyof T]: () => T[K] } }
  : never;

const createSelectors = <S extends UseBoundStore<StoreApi<object>>>(
  _store: S,
) => {
  const store = _store as WithSelectors<typeof _store>;
  store.use = {};
  for (const k of Object.keys(store.getState())) {
    (store.use as any)[k] = () => store((s) => s[k as keyof typeof s]);
  }

  return store;
};

type StoreState = TeamsSlice &
  MeetingsSlice &
  UISlice &
  ModelsSlice &
  UpdateSlice;

const useBoundStoreBase = create<StoreState>()(
  immer((...a) => ({
    ...createTeamsSlice(...(a as Parameters<StateCreator<StoreState>>)),
    ...createMeetingsSlice(...(a as Parameters<StateCreator<StoreState>>)),
    ...createUISlice(...(a as Parameters<StateCreator<StoreState>>)),
    ...createModelsSlice(...(a as Parameters<StateCreator<StoreState>>)),
    ...createUpdateSlice(...(a as Parameters<StateCreator<StoreState>>)),
  })) as StateCreator<StoreState, [], [], StoreState>,
);

export const useBoundStore = createSelectors(useBoundStoreBase);
