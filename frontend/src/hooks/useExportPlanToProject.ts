import { useMutation, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { ideationKeys } from "@/hooks/useIdeation";
import { IdeationSessionSchema, type IdeationSession } from "@/types/ideation";

export interface ExportPlanInput {
  targetProjectPath: string;
  sourceSessionId: string;
  title?: string;
}

export function useExportPlanToProject() {
  const queryClient = useQueryClient();
  return useMutation<IdeationSession, Error, ExportPlanInput>({
    mutationFn: async ({ targetProjectPath, sourceSessionId, title }) => {
      const result = await invoke("create_cross_project_session", {
        input: {
          targetProjectPath,
          sourceSessionId,
          ...(title !== undefined && { title }),
        },
      });
      return IdeationSessionSchema.parse(result);
    },
    onSuccess: () => {
      // Invalidate all session lists since we don't know the target project ID upfront
      queryClient.invalidateQueries({ queryKey: ideationKeys.sessions() });
    },
  });
}
