"use client";

import { listen } from "@tauri-apps/api/event";
import * as React from "react";
import type { BatchTaskEvent } from "@/types";

export function useBatchTaskEvents() {
  const [eventsByTask, setEventsByTask] = React.useState<
    Record<string, Record<string, BatchTaskEvent>>
  >({});

  React.useEffect(() => {
    let unlisten: (() => void) | undefined;

    void (async () => {
      unlisten = await listen<BatchTaskEvent>("batch-task-status", (event) => {
        const payload = event.payload;
        setEventsByTask((current) => ({
          ...current,
          [payload.task_id]: {
            ...(current[payload.task_id] ?? {}),
            [payload.profile_id]: payload,
          },
        }));
      });
    })();

    return () => {
      unlisten?.();
    };
  }, []);

  const clearTask = React.useCallback((taskId: string) => {
    setEventsByTask((current) => {
      const next = { ...current };
      delete next[taskId];
      return next;
    });
  }, []);

  return { eventsByTask, clearTask };
}
