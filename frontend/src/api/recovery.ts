// Tauri invoke wrappers for recovery actions

import { z } from "zod";
import { typedInvoke } from "@/lib/tauri";

export type RecoveryAction = "restart" | "cancel";

export async function recoverTaskExecution(taskId: string): Promise<boolean> {
  return typedInvoke(
    "recover_task_execution",
    { taskId },
    z.boolean()
  );
}

export async function resolveRecoveryPrompt(
  taskId: string,
  action: RecoveryAction
): Promise<boolean> {
  return typedInvoke(
    "resolve_recovery_prompt",
    { taskId, action },
    z.boolean()
  );
}
