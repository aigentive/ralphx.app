/**
 * useMethodologyActivation - Integration hook for methodology activation
 *
 * Connects methodology activation to app state:
 * - Updates methodology store with activated methodology
 * - Invalidates workflow queries to reload columns
 * - Shows toast notifications via uiStore
 * - Manages loading states
 */

import { useCallback } from "react";
import { useQueryClient } from "@tanstack/react-query";
import * as methodologiesApi from "@/lib/api/methodologies";
import type {
  MethodologyActivationResponse,
  MethodologyResponse,
  MethodologyPhaseResponse,
  MethodologyTemplateResponse,
} from "@/lib/api/methodologies";
import {
  useMethodologyStore,
  selectActiveMethodology,
  type Methodology,
  type MethodologyPhase,
  type MethodologyTemplate,
} from "@/stores/methodologyStore";
import { useUiStore } from "@/stores/uiStore";
import { workflowKeys } from "./useWorkflows";
import { methodologyKeys } from "./useMethodologies";

// ============================================================================
// Types
// ============================================================================

interface UseMethodologyActivationReturn {
  /** Activate a methodology by ID */
  activate: (methodologyId: string) => Promise<MethodologyActivationResponse>;
  /** Deactivate a methodology by ID */
  deactivate: (methodologyId: string) => Promise<MethodologyResponse>;
  /** Whether activation/deactivation is in progress */
  isActivating: boolean;
  /** Currently active methodology, or null if none */
  activeMethodology: ReturnType<typeof selectActiveMethodology>;
}

// ============================================================================
// Helper Functions
// ============================================================================

let notificationCounter = 0;

function generateNotificationId(): string {
  return `notification-${Date.now()}-${++notificationCounter}`;
}

/**
 * Convert API phase response (snake_case) to store phase type (camelCase)
 */
function convertPhase(phase: MethodologyPhaseResponse): MethodologyPhase {
  return {
    id: phase.id,
    name: phase.name,
    order: phase.order,
    description: phase.description,
    agentProfiles: phase.agent_profiles,
    columnIds: phase.column_ids,
  };
}

/**
 * Convert API template response (snake_case) to store template type (camelCase)
 */
function convertTemplate(template: MethodologyTemplateResponse): MethodologyTemplate {
  return {
    artifactType: template.artifact_type,
    templatePath: template.template_path,
    name: template.name,
    description: template.description,
  };
}

/**
 * Convert API methodology response (snake_case) to store methodology type (camelCase)
 */
function convertMethodologyResponse(response: MethodologyResponse): Methodology {
  return {
    id: response.id,
    name: response.name,
    description: response.description,
    agentProfiles: response.agent_profiles,
    skills: response.skills,
    workflowId: response.workflow_id,
    workflowName: response.workflow_name,
    phases: response.phases.map(convertPhase),
    templates: response.templates.map(convertTemplate),
    isActive: response.is_active,
    phaseCount: response.phase_count,
    agentCount: response.agent_count,
    createdAt: response.created_at,
  };
}

// ============================================================================
// Hook Implementation
// ============================================================================

/**
 * Hook for integrating methodology activation with app state
 *
 * @returns Object with activate, deactivate, isActivating, and activeMethodology
 *
 * @example
 * ```tsx
 * const { activate, deactivate, isActivating, activeMethodology } = useMethodologyActivation();
 *
 * const handleActivate = async (id: string) => {
 *   const result = await activate(id);
 *   console.log(`Activated ${result.methodology.name}`);
 * };
 * ```
 */
export function useMethodologyActivation(): UseMethodologyActivationReturn {
  const queryClient = useQueryClient();

  // Methodology store
  const { setActivating, isActivating } = useMethodologyStore();
  const activeMethodology = useMethodologyStore(selectActiveMethodology);

  // UI store for notifications
  const addNotification = useUiStore((state) => state.addNotification);

  /**
   * Activate a methodology and update app state
   */
  const activate = useCallback(
    async (methodologyId: string): Promise<MethodologyActivationResponse> => {
      setActivating(true);

      try {
        // Call API to activate methodology
        const response = await methodologiesApi.activateMethodology(methodologyId);

        // Convert and add methodology to store (direct state update to ensure it exists)
        const convertedMethodology = convertMethodologyResponse(response.methodology);
        useMethodologyStore.setState((state) => ({
          methodologies: {
            ...state.methodologies,
            [methodologyId]: convertedMethodology,
          },
          activeMethodologyId: methodologyId,
        }));

        // Invalidate queries to refresh workflow columns
        queryClient.invalidateQueries({ queryKey: methodologyKeys.all });
        queryClient.invalidateQueries({ queryKey: workflowKeys.lists() });
        queryClient.invalidateQueries({ queryKey: workflowKeys.activeColumns() });

        // Show success notification
        addNotification({
          id: generateNotificationId(),
          type: "success",
          message: `Activated ${response.methodology.name}`,
          title: "Methodology Activated",
          duration: 3000,
        });

        return response;
      } catch (error) {
        // Show error notification
        const errorMessage = error instanceof Error ? error.message : "Unknown error";
        addNotification({
          id: generateNotificationId(),
          type: "error",
          message: `Activation failed: ${errorMessage}`,
          title: "Activation Error",
          duration: 5000,
        });

        throw error;
      } finally {
        setActivating(false);
      }
    },
    [queryClient, setActivating, addNotification]
  );

  /**
   * Deactivate a methodology and restore default workflow
   */
  const deactivate = useCallback(
    async (methodologyId: string): Promise<MethodologyResponse> => {
      setActivating(true);

      try {
        // Call API to deactivate methodology
        const response = await methodologiesApi.deactivateMethodology(methodologyId);

        // Convert and update methodology in store, clear active methodology
        const convertedMethodology = convertMethodologyResponse(response);
        useMethodologyStore.setState((state) => ({
          methodologies: {
            ...state.methodologies,
            [methodologyId]: convertedMethodology,
          },
          activeMethodologyId: null,
        }));

        // Invalidate queries to refresh workflow columns
        queryClient.invalidateQueries({ queryKey: methodologyKeys.all });
        queryClient.invalidateQueries({ queryKey: workflowKeys.lists() });
        queryClient.invalidateQueries({ queryKey: workflowKeys.activeColumns() });

        // Show success notification
        addNotification({
          id: generateNotificationId(),
          type: "success",
          message: "Returned to default workflow",
          title: "Methodology Deactivated",
          duration: 3000,
        });

        return response;
      } catch (error) {
        // Show error notification
        const errorMessage = error instanceof Error ? error.message : "Unknown error";
        addNotification({
          id: generateNotificationId(),
          type: "error",
          message: `Deactivation failed: ${errorMessage}`,
          title: "Deactivation Error",
          duration: 5000,
        });

        throw error;
      } finally {
        setActivating(false);
      }
    },
    [queryClient, setActivating, addNotification]
  );

  return {
    activate,
    deactivate,
    isActivating,
    activeMethodology,
  };
}
