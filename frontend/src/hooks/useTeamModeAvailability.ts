import { useMemo } from "react";

import { useFeatureFlags } from "@/hooks/useFeatureFlags";
import { useAgentHarnessSettings } from "@/hooks/useIdeationHarnessSettings";
import type { ContextType } from "@/types/chat-conversation";

interface TeamModeAvailability {
  featureFlagEnabled: boolean;
  harnessResolved: boolean;
  ideationTeamModeAvailable: boolean;
  executionTeamModeAvailable: boolean;
  isAvailableForContext: (contextType: ContextType | string | null | undefined) => boolean;
}

export function useTeamModeAvailability(projectId: string | null): TeamModeAvailability {
  const { data: featureFlags, isPlaceholderData: isFeatureFlagsPlaceholder } =
    useFeatureFlags();
  const {
    lanes,
    isLoading: isHarnessLoading,
    isPlaceholderData: isHarnessPlaceholderData,
  } = useAgentHarnessSettings(projectId);

  const harnessResolved = !isHarnessLoading && !isHarnessPlaceholderData;
  const featureFlagEnabled = !isFeatureFlagsPlaceholder && featureFlags.teamMode;

  const ideationPrimaryLane = lanes.find((lane) => lane.lane === "ideation_primary");
  const executionWorkerLane = lanes.find((lane) => lane.lane === "execution_worker");

  const ideationTeamModeAvailable =
    featureFlagEnabled &&
    harnessResolved &&
    ideationPrimaryLane?.effectiveHarness === "claude";

  const executionTeamModeAvailable =
    featureFlagEnabled &&
    harnessResolved &&
    executionWorkerLane?.effectiveHarness === "claude";

  const isAvailableForContext = useMemo(
    () => (contextType: ContextType | string | null | undefined) => {
      switch (contextType) {
        case "ideation":
          return ideationTeamModeAvailable;
        case "task_execution":
          return executionTeamModeAvailable;
        default:
          return false;
      }
    },
    [executionTeamModeAvailable, ideationTeamModeAvailable],
  );

  return {
    featureFlagEnabled,
    harnessResolved,
    ideationTeamModeAvailable,
    executionTeamModeAvailable,
    isAvailableForContext,
  };
}
