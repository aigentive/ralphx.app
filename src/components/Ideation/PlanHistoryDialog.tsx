/**
 * PlanHistoryDialog - Modal dialog to display historical plan versions
 *
 * Features:
 * - Shows plan content at a specific version
 * - Markdown rendering
 * - Close button
 */

import { useEffect, useState } from "react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { artifactApi } from "@/api/artifact";
import type { Artifact } from "@/types/artifact";

// ============================================================================
// Types
// ============================================================================

export interface PlanHistoryDialogProps {
  /** Whether the dialog is open */
  isOpen: boolean;
  /** Callback when dialog is closed */
  onClose: () => void;
  /** Artifact ID to fetch */
  artifactId: string;
  /** Version number to fetch */
  version: number;
}

// ============================================================================
// Component
// ============================================================================

export function PlanHistoryDialog({
  isOpen,
  onClose,
  artifactId,
  version,
}: PlanHistoryDialogProps) {
  const [artifact, setArtifact] = useState<Artifact | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!isOpen) {
      // Reset state when dialog closes
      setArtifact(null);
      setError(null);
      return;
    }

    // Fetch the artifact at the specified version
    const fetchArtifact = async () => {
      setIsLoading(true);
      setError(null);
      try {
        const result = await artifactApi.getAtVersion(artifactId, version);
        if (result) {
          setArtifact(result);
        } else {
          setError("Plan version not found");
        }
      } catch (err) {
        setError(
          err instanceof Error ? err.message : "Failed to load plan version"
        );
      } finally {
        setIsLoading(false);
      }
    };

    fetchArtifact();
  }, [isOpen, artifactId, version]);

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
      <DialogContent
        className="max-w-3xl max-h-[80vh] overflow-hidden flex flex-col"
        style={{
          backgroundColor: "var(--bg-elevated)",
          borderColor: "var(--border-subtle)",
        }}
      >
        <DialogHeader>
          <DialogTitle
            style={{
              color: "var(--text-primary)",
              fontFamily: "SF Pro Display, -apple-system, BlinkMacSystemFont, sans-serif",
            }}
          >
            {artifact?.name || "Plan"} (v{version})
          </DialogTitle>
        </DialogHeader>

        <div
          className="flex-1 overflow-y-auto p-4 rounded-lg"
          style={{
            backgroundColor: "var(--bg-surface)",
          }}
        >
          {isLoading && (
            <div
              className="flex items-center justify-center py-12"
              style={{ color: "var(--text-muted)" }}
            >
              Loading plan version...
            </div>
          )}

          {error && (
            <div
              className="p-4 rounded-lg"
              style={{
                backgroundColor: "var(--bg-hover)",
                color: "var(--text-primary)",
              }}
            >
              {error}
            </div>
          )}

          {artifact && artifact.content.type === "inline" && (
            <div
              className="prose prose-sm max-w-none"
              style={{ color: "var(--text-primary)" }}
            >
              <pre
                className="whitespace-pre-wrap font-mono text-sm"
                style={{
                  color: "var(--text-primary)",
                  backgroundColor: "transparent",
                }}
              >
                {artifact.content.text}
              </pre>
            </div>
          )}

          {artifact && artifact.content.type === "file" && (
            <div
              style={{
                color: "var(--text-muted)",
              }}
            >
              File content at: {artifact.content.path}
            </div>
          )}
        </div>

        <div className="flex justify-end pt-4">
          <button
            onClick={onClose}
            className="px-4 py-2 rounded-lg font-medium transition-all"
            style={{
              backgroundColor: "var(--bg-hover)",
              color: "var(--text-primary)",
            }}
          >
            Close
          </button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
