import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";

import { useEventBus } from "@/providers/EventProvider";
import type { Unsubscribe } from "@/lib/event-bus";
import { designSystemKeys } from "./useProjectDesignSystems";

const DESIGN_EVENTS = [
  "design:system_created",
  "design:system_updated",
  "design:schema_published",
  "design:artifact_created",
  "design:styleguide_item_approved",
  "design:styleguide_item_feedback_created",
  "design:run_started",
  "design:run_completed",
  "design:export_completed",
  "design:import_completed",
] as const;

export function useDesignSystemEvents() {
  const bus = useEventBus();
  const queryClient = useQueryClient();

  useEffect(() => {
    const invalidateFromPayload = (payload: unknown) => {
      const designSystemId = designSystemIdFromPayload(payload);
      const projectId = projectIdFromPayload(payload);

      queryClient.invalidateQueries({ queryKey: designSystemKeys.all });
      if (projectId) {
        queryClient.invalidateQueries({ queryKey: designSystemKeys.project(projectId) });
      }
      if (designSystemId) {
        queryClient.invalidateQueries({ queryKey: designSystemKeys.detail(designSystemId) });
        queryClient.invalidateQueries({
          queryKey: designSystemKeys.styleguideItems(designSystemId),
        });
        queryClient.invalidateQueries({
          queryKey: designSystemKeys.styleguideViewModel(designSystemId),
        });
      }
    };

    const unsubscribes: Unsubscribe[] = DESIGN_EVENTS.map((eventName) =>
      bus.subscribe<unknown>(eventName, invalidateFromPayload),
    );

    return () => {
      unsubscribes.forEach((unsubscribe) => unsubscribe());
    };
  }, [bus, queryClient]);
}

function designSystemIdFromPayload(payload: unknown): string | null {
  const record = asRecord(payload);
  if (!record) {
    return null;
  }
  return (
    readString(record, "designSystemId") ??
    readString(record, "design_system_id") ??
    readNestedString(record, "designSystem", "id") ??
    null
  );
}

function projectIdFromPayload(payload: unknown): string | null {
  const record = asRecord(payload);
  if (!record) {
    return null;
  }
  return (
    readString(record, "primaryProjectId") ??
    readString(record, "primary_project_id") ??
    readNestedString(record, "designSystem", "primaryProjectId") ??
    readNestedString(record, "design_system", "primary_project_id") ??
    null
  );
}

function readNestedString(
  record: Record<string, unknown>,
  key: string,
  nestedKey: string,
): string | null {
  const nested = asRecord(record[key]);
  return nested ? readString(nested, nestedKey) : null;
}

function readString(record: Record<string, unknown>, key: string): string | null {
  const value = record[key];
  return typeof value === "string" && value.trim() ? value : null;
}

function asRecord(value: unknown): Record<string, unknown> | null {
  return value && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}
