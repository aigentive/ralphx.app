import { useEffect, useRef } from "react";

export function useSyncedAgentProjectFocus(
  projectId: string,
  setFocusedProject: (projectId: string | null) => void,
) {
  const syncedProjectIdRef = useRef<string | null>(null);

  useEffect(() => {
    if (!projectId || syncedProjectIdRef.current === projectId) {
      return;
    }
    syncedProjectIdRef.current = projectId;
    setFocusedProject(projectId);
  }, [projectId, setFocusedProject]);
}
