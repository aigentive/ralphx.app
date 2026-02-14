/**
 * Generic state machine hook for quick action flows
 *
 * State transitions:
 * idle ──(startConfirmation)──> confirming ──(confirm)──> creating ──(success)──> success
 *   ^                               |                        |                       |
 *   └────────(cancel)───────────────┘                        |                       |
 *   └────────(error)──────────────────────────────────────────┘                       |
 *   └────────(dismiss/viewEntity)──────────────────────────────────────────────────────┘
 *
 * Usage:
 * ```tsx
 * const action: QuickAction = {
 *   id: "ideation",
 *   label: "Start new ideation session",
 *   execute: async (query) => createSession(query),
 *   navigateTo: (id) => router.push(`/sessions/${id}`),
 *   // ... other properties
 * };
 *
 * const flow = useQuickActionFlow(action);
 *
 * // User types in search → show action row → select
 * flow.startConfirmation();
 *
 * // User confirms
 * await flow.confirm(query);
 *
 * // User clicks "View"
 * flow.viewEntity();
 * ```
 */

import { useState, useCallback, useRef } from "react";
import type { LucideIcon } from "lucide-react";

export type QuickActionFlowState = "idle" | "confirming" | "creating" | "success";

export interface QuickAction {
  /** Unique identifier for this action type */
  id: string;
  /** Label shown in the action row (e.g. "Start new ideation session") */
  label: string;
  /** Icon to display */
  icon: LucideIcon;
  /** Description shown with the query (e.g. `"${query}"`) */
  description: (query: string) => string;
  /** Whether this action should appear given the current query */
  isVisible: (query: string) => boolean;
  /** Execute the action. Returns entity ID on success. */
  execute: (query: string) => Promise<string>;
  /** Label shown during creation (e.g. "Creating your ideation session...") */
  creatingLabel: string;
  /** Label shown on success (e.g. "Session created!") */
  successLabel: string;
  /** Button text on success (e.g. "View Session") */
  viewLabel: string;
  /** Navigate to the created entity */
  navigateTo: (entityId: string) => void;
}

export interface UseQuickActionFlowReturn {
  /** Current flow state */
  flowState: QuickActionFlowState;
  /** ID of the created entity (available in success state) */
  createdEntityId: string | null;
  /** Error message if creation failed */
  error: string | null;
  /** Start the confirmation flow */
  startConfirmation: () => void;
  /** Confirm and execute the action */
  confirm: (query: string) => Promise<void>;
  /** Cancel confirmation (returns to idle) */
  cancel: () => void;
  /** View the created entity (calls navigateTo, then returns to idle) */
  viewEntity: () => void;
  /** Dismiss success banner or error (returns to idle) */
  dismiss: () => void;
  /** True when flow is blocking UI (confirming | creating | success) */
  isBlocking: boolean;
}

export function useQuickActionFlow(action: QuickAction): UseQuickActionFlowReturn {
  const [flowState, setFlowState] = useState<QuickActionFlowState>("idle");
  const [createdEntityId, setCreatedEntityId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Track if we're currently executing to prevent double-confirm
  const isExecutingRef = useRef(false);

  // Store action and entity ID in refs to always use latest version and stabilize callbacks
  const actionRef = useRef(action);
  actionRef.current = action;

  const createdEntityIdRef = useRef<string | null>(null);
  createdEntityIdRef.current = createdEntityId;

  const startConfirmation = useCallback(() => {
    setFlowState("confirming");
    setError(null);
  }, []);

  const confirm = useCallback(async (query: string) => {
    // Prevent double-confirm
    if (isExecutingRef.current) {
      return;
    }

    isExecutingRef.current = true;
    setFlowState("creating");
    setError(null);

    try {
      const entityId = await actionRef.current.execute(query);
      setCreatedEntityId(entityId);
      setFlowState("success");
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : "An error occurred";
      setError(errorMessage);
      setFlowState("idle");
      setCreatedEntityId(null);
    } finally {
      isExecutingRef.current = false;
    }
  }, []);

  const cancel = useCallback(() => {
    setFlowState("idle");
    setError(null);
  }, []);

  const viewEntity = useCallback(() => {
    if (createdEntityIdRef.current) {
      actionRef.current.navigateTo(createdEntityIdRef.current);
    }
    setFlowState("idle");
    setCreatedEntityId(null);
  }, []);

  const dismiss = useCallback(() => {
    setFlowState("idle");
    setCreatedEntityId(null);
    setError(null);
  }, []);

  const isBlocking = flowState !== "idle";

  return {
    flowState,
    createdEntityId,
    error,
    startConfirmation,
    confirm,
    cancel,
    viewEntity,
    dismiss,
    isBlocking,
  };
}
