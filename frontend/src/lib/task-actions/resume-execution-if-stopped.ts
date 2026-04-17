import { api } from "@/lib/tauri";

export async function resumeExecutionIfStopped(projectId: string): Promise<boolean> {
  const executionStatus = await api.execution.getStatus(projectId).catch(() => null);
  if (executionStatus?.haltMode !== "stopped") {
    return false;
  }

  await api.execution.resume(projectId);
  return true;
}
