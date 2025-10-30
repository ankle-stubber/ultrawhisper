import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { LogEntry, LogFilter } from "../lib/types";

interface LogsStore {
  logs: LogEntry[];
  loading: boolean;
  filter: LogFilter;
  autoScroll: boolean;
  bufferSize: number;

  // Actions
  setFilter: (filter: LogFilter) => void;
  setAutoScroll: (enabled: boolean) => void;
  loadLogs: () => Promise<void>;
  clearLogs: () => Promise<void>;
  exportLogs: (format: "txt" | "json", path: string) => Promise<void>;
  startListening: () => Promise<void>;
  stopListening: () => void;
}

let unlistenLogEntry: UnlistenFn | null = null;
let unlistenLogCleared: UnlistenFn | null = null;

export const useLogsStore = create<LogsStore>((set, get) => ({
  logs: [],
  loading: false,
  filter: {},
  autoScroll: true,
  bufferSize: 0,

  setFilter: (filter: LogFilter) => set({ filter }),

  setAutoScroll: (enabled: boolean) => set({ autoScroll: enabled }),

  loadLogs: async () => {
    set({ loading: true });
    try {
      const response = await invoke<{ entries: LogEntry[]; buffer_size: number }>(
        "get_logs"
      );
      set({ logs: response.entries, bufferSize: response.buffer_size, loading: false });
    } catch (error) {
      console.error("Failed to load logs:", error);
      set({ loading: false });
    }
  },

  clearLogs: async () => {
    try {
      await invoke("clear_logs");
      set({ logs: [], bufferSize: 0 });
    } catch (error) {
      console.error("Failed to clear logs:", error);
    }
  },

  exportLogs: async (format: "txt" | "json", path: string) => {
    try {
      await invoke("export_logs", { format, path });
    } catch (error) {
      console.error("Failed to export logs:", error);
      throw error;
    }
  },

  startListening: async () => {
    // Don't set up multiple listeners
    if (unlistenLogEntry) return;

    // Load initial logs
    await get().loadLogs();

    // Listen for new log entries (debounced by backend)
    unlistenLogEntry = await listen<LogEntry>("log-entry", (event) => {
      const newEntry = event.payload;
      set((state) => ({
        logs: [...state.logs, newEntry],
        bufferSize: state.bufferSize + 1,
      }));
    });

    // Listen for log cleared event
    unlistenLogCleared = await listen("log-cleared", () => {
      set({ logs: [], bufferSize: 0 });
    });
  },

  stopListening: () => {
    if (unlistenLogEntry) {
      unlistenLogEntry();
      unlistenLogEntry = null;
    }
    if (unlistenLogCleared) {
      unlistenLogCleared();
      unlistenLogCleared = null;
    }
  },
}));
