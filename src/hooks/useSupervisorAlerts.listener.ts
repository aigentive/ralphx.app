/**
 * Supervisor event listener hook
 *
 * Listens for supervisor events and alerts from the backend
 * and updates the supervisor store accordingly.
 */

import { useEffect } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { SupervisorEventSchema, SupervisorAlertSchema } from "@/types/supervisor";
import { useSupervisorStore } from "./useSupervisorAlerts.store";

/**
 * Hook to listen for supervisor events from the backend
 *
 * Listens to both 'supervisor:event' (raw events) and 'supervisor:alert' (alerts)
 * channels and updates the supervisor store accordingly.
 *
 * @param options - Optional configuration
 */
export function useSupervisorEventListener(options: { enabled?: boolean } = {}) {
  const { enabled = true } = options;
  const addAlert = useSupervisorStore((s) => s.addAlert);
  const setConnected = useSupervisorStore((s) => s.setConnected);

  useEffect(() => {
    if (!enabled) return;

    const unlisteners: Promise<UnlistenFn>[] = [];

    // Listen for supervisor alerts (pre-processed by backend)
    unlisteners.push(
      listen<unknown>("supervisor:alert", (event) => {
        const parsed = SupervisorAlertSchema.omit({
          id: true,
          acknowledged: true,
          createdAt: true,
          acknowledgedAt: true,
        }).safeParse(event.payload);

        if (parsed.success) {
          addAlert(parsed.data);
        }
      })
    );

    // Listen for raw supervisor events (for custom processing)
    unlisteners.push(
      listen<unknown>("supervisor:event", (event) => {
        const parsed = SupervisorEventSchema.safeParse(event.payload);

        if (!parsed.success) {
          return;
        }

        // Convert certain events to alerts
        const supervisorEvent = parsed.data;

        switch (supervisorEvent.type) {
          case "error":
            addAlert({
              taskId: supervisorEvent.taskId,
              type: "error",
              severity: supervisorEvent.info.recoverable ? "medium" : "high",
              message: supervisorEvent.info.message,
              details: `Source: ${supervisorEvent.info.source}`,
            });
            break;

          case "token_threshold":
            addAlert({
              taskId: supervisorEvent.taskId,
              type: "token_warning",
              severity: "medium",
              message: `Token usage ${supervisorEvent.tokensUsed} exceeds threshold ${supervisorEvent.threshold}`,
              suggestedAction: "pause",
            });
            break;

          case "time_threshold":
            addAlert({
              taskId: supervisorEvent.taskId,
              type: "time_warning",
              severity: "medium",
              message: `Execution time ${supervisorEvent.elapsedMinutes}min exceeds threshold ${supervisorEvent.thresholdMinutes}min`,
              suggestedAction: "pause",
            });
            break;
        }
      })
    );

    setConnected(true);

    return () => {
      setConnected(false);
      void Promise.all(unlisteners).then((listeners) => {
        listeners.forEach((fn) => fn());
      });
    };
  }, [enabled, addAlert, setConnected]);
}
