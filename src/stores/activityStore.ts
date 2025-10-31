import { create } from "zustand";

interface ActivityStore {
  activeWorkflowId: string | null;
  startedAt: number | null;

  setActiveWorkflow: (id: string) => void;
  clearActiveWorkflow: () => void;
}

export const useActivityStore = create<ActivityStore>((set) => ({
  activeWorkflowId: null,
  startedAt: null,

  setActiveWorkflow: (id: string) => set({ activeWorkflowId: id, startedAt: Date.now() }),
  clearActiveWorkflow: () => set({ activeWorkflowId: null, startedAt: null }),
}));

