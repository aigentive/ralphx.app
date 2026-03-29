import { useIdeationStore } from "@/stores/ideationStore";
import { useProjectStore } from "@/stores/projectStore";
import { useUiStore } from "@/stores/uiStore";

/**
 * Navigate to a specific ideation session.
 * Switches the main view to "ideation" and selects the target session.
 * If the session belongs to a different project, pre-writes the target
 * project's view/session maps and calls selectProject so the App.tsx
 * effect handles the rest (RESTORE phase reads our pre-written values).
 * Safe to call from any current view.
 */
export function navigateToIdeationSession(sessionId: string): void {
  const session = useIdeationStore.getState().sessions[sessionId];

  if (!session) {
    console.warn(
      `navigateToIdeationSession: session "${sessionId}" not found in store — falling back to direct navigation`,
    );
    useUiStore.getState().setCurrentView("ideation");
    useIdeationStore.getState().setActiveSession(sessionId);
    return;
  }

  const { activeProjectId } = useProjectStore.getState();
  const targetProjectId = session.projectId;

  if (activeProjectId !== null && activeProjectId !== targetProjectId) {
    // Cross-project navigation: pre-write maps so the App.tsx effect reads
    // the correct view and session during its RESTORE phase, then trigger
    // the project switch via selectProject.
    const uiState = useUiStore.getState();
    useUiStore.setState({
      viewByProject: { ...uiState.viewByProject, [targetProjectId]: "ideation" },
      sessionByProject: {
        ...uiState.sessionByProject,
        [targetProjectId]: sessionId,
      },
    });
    useProjectStore.getState().selectProject(targetProjectId);
    return;
  }

  // Same-project navigation: fast path.
  useUiStore.getState().setCurrentView("ideation");
  useIdeationStore.getState().setActiveSession(sessionId);
}
