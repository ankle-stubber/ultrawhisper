import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { StoredWorkflow } from "../lib/types";

export function useWorkflows() {
  const [workflows, setWorkflows] = useState<StoredWorkflow[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<StoredWorkflow[]>("list_workflows");
      setWorkflows(result);
    } catch (err) {
      console.error("Failed to load workflows:", err);
      setError(err instanceof Error ? err.message : "Failed to load workflows");
    } finally {
      setLoading(false);
    }
  }, []);

  // Subscribe to workflows-changed event
  useEffect(() => {
    let unlisten: (() => void) | undefined;

    const setup = async () => {
      // Initial load
      await load();

      // Listen for changes
      unlisten = await listen("workflows-changed", () => {
        console.log("Workflows changed, reloading...");
        load();
      });
    };

    setup();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, [load]);

  const save = useCallback(async (workflow: StoredWorkflow) => {
    try {
      await invoke("upsert_workflow", { workflow });
      // Event will trigger reload automatically
    } catch (err) {
      console.error("Failed to save workflow:", err);
      throw err;
    }
  }, []);

  const remove = useCallback(async (id: string) => {
    try {
      await invoke("delete_workflow", { id });
      // Event will trigger reload automatically
    } catch (err) {
      console.error("Failed to delete workflow:", err);
      throw err;
    }
  }, []);

  return { workflows, loading, error, load, save, remove };
}
