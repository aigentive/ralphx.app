import { useUiStore } from "@/stores/uiStore";
import { useIdeationStore } from "@/stores/ideationStore";

/**
 * Navigate to a specific ideation session.
 * Switches the main view to "ideation" and selects the target session.
 * Safe to call from any current view.
 */
export function navigateToIdeationSession(sessionId: string): void {
  useUiStore.getState().setCurrentView("ideation");
  useIdeationStore.getState().setActiveSession(sessionId);
}
