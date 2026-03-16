/**
 * useSessionExportImport — Hook for exporting and importing ideation sessions.
 *
 * Export: calls export_ideation_session (returns raw JSON string), prompts save dialog,
 * writes file to disk.
 *
 * Import: prompts open dialog, validates file size, reads content, calls
 * import_ideation_session, invalidates session list, and activates the imported session.
 */

import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { readTextFile, writeTextFile, stat } from "@tauri-apps/plugin-fs";
import { toast } from "sonner";
import { useQueryClient } from "@tanstack/react-query";
import { z } from "zod";
import { typedInvoke } from "@/lib/tauri";
import { ideationKeys } from "@/hooks/useIdeation";
import { useIdeationStore } from "@/stores/ideationStore";

// ============================================================================
// Schemas
// ============================================================================

const ImportedSessionSchema = z.object({
  sessionId: z.string(),
  title: z.string().nullable(),
  proposalCount: z.number(),
  planVersionCount: z.number(),
});

// ============================================================================
// Constants
// ============================================================================

const MAX_IMPORT_FILE_SIZE = 10_485_760; // 10MB

// ============================================================================
// Hook
// ============================================================================

export function useSessionExportImport() {
  const [isExporting, setIsExporting] = useState(false);
  const [isImporting, setIsImporting] = useState(false);
  const queryClient = useQueryClient();
  const setActiveSession = useIdeationStore((state) => state.setActiveSession);

  /**
   * Import a session from a .ralphx-session file chosen via the file dialog.
   * Silently aborts if the user cancels the dialog.
   */
  async function importSession(projectId: string): Promise<void> {
    // Step 1: Open file dialog — null means user cancelled
    const path = await open({
      filters: [{ name: "RalphX Session", extensions: ["ralphx-session"] }],
    });

    if (path === null) {
      return;
    }

    setIsImporting(true);
    try {
      // Step 2: Check file size — skip size check if stat fails (file still valid)
      try {
        const fileStat = await stat(path);
        if (fileStat.size > MAX_IMPORT_FILE_SIZE) {
          toast.error("File too large (max 10MB)");
          setIsImporting(false);
          return;
        }
      } catch {
        // stat() failed — proceed without size check, backend will validate
      }

      // Step 3: Read file content
      const content = await readTextFile(path);

      // Step 4: Call backend import command
      const result = await typedInvoke(
        "import_ideation_session",
        { input: { jsonContent: content, projectId } },
        ImportedSessionSchema
      );

      // Step 5: Invalidate session list, activate imported session, show success
      await queryClient.invalidateQueries({
        queryKey: ideationKeys.sessionList(projectId),
      });
      setActiveSession(result.sessionId);
      toast.success(
        `Imported "${result.title ?? "Session"}" (${result.proposalCount} proposals)`
      );
    } catch (err) {
      // Step 6: Parse error code prefix for user-friendly messages
      const errorCode =
        err instanceof Error && err.message.startsWith("IMPORT_")
          ? err.message.split(":")[0]
          : null;

      const message =
        errorCode === "IMPORT_VERSION_UNSUPPORTED"
          ? "This file was created by a newer version of RalphX"
          : errorCode === "IMPORT_INVALID_FORMAT"
            ? "Invalid session file"
            : errorCode === "IMPORT_INVALID_DEPENDENCY"
              ? "Session file contains invalid dependencies"
              : "Failed to import session";

      toast.error(message);
    } finally {
      setIsImporting(false);
    }
  }

  /**
   * Export a session to a .ralphx-session file via the save dialog.
   * Silently aborts if the user cancels the dialog.
   * Guard: requires hasPlan to be true before exporting.
   */
  async function exportSession(
    sessionId: string,
    projectId: string,
    hasPlan: boolean
  ): Promise<void> {
    // Guard: nothing to export without a plan
    if (!hasPlan) {
      toast.error("No plan to export");
      return;
    }

    setIsExporting(true);
    try {
      // Step 2: Call backend — returns raw JSON string, not a typed object
      const jsonContent = await invoke<string>("export_ideation_session", {
        id: sessionId,
        projectId,
      });

      // Step 3: Open save dialog — null means user cancelled
      const savePath = await save({
        filters: [{ name: "RalphX Session", extensions: ["ralphx-session"] }],
        defaultPath: `session-${sessionId}.ralphx-session`,
      });

      if (savePath === null) {
        return;
      }

      // Step 4: Write file to disk
      try {
        await writeTextFile(savePath, jsonContent);
      } catch {
        // Reset exporting state on write failure before rethrowing
        setIsExporting(false);
        throw new Error("Failed to write session file");
      }

      // Step 5: Confirm success
      toast.success("Session exported successfully");
    } catch {
      toast.error("Failed to export session");
    } finally {
      setIsExporting(false);
    }
  }

  return { exportSession, importSession, isExporting, isImporting };
}
