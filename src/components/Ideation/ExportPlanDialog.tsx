import { useState, useMemo } from "react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { CheckCircle2, FolderOpen } from "lucide-react";
import { useProjects } from "@/hooks/useProjects";
import { useExportPlanToProject } from "@/hooks/useExportPlanToProject";

// ============================================================================
// Types
// ============================================================================

export interface ExportPlanDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  sessionId: string;
  sessionTitle: string | null;
  verificationStatus: string;
}

// ============================================================================
// Helpers
// ============================================================================

function getVerificationBadgeLabel(status: string): string {
  if (status === "imported_verified") return "Verified (imported)";
  if (status === "verified") return "Verified";
  return status.replace(/_/g, " ");
}

// ============================================================================
// Component
// ============================================================================

export function ExportPlanDialog({
  open,
  onOpenChange,
  sessionId,
  sessionTitle,
  verificationStatus,
}: ExportPlanDialogProps) {
  const [targetPath, setTargetPath] = useState("");
  const [showDropdown, setShowDropdown] = useState(false);
  const [successSession, setSuccessSession] = useState<{
    title: string | null;
  } | null>(null);

  const { data: projects } = useProjects();
  const mutation = useExportPlanToProject();

  const matchingProjects = useMemo(() => {
    if (!targetPath || !projects) return [];
    return projects.filter((p) =>
      p.workingDirectory.startsWith(targetPath)
    );
  }, [targetPath, projects]);

  const handleOpenChange = (next: boolean) => {
    if (!next) {
      setTargetPath("");
      setShowDropdown(false);
      setSuccessSession(null);
      mutation.reset();
    }
    onOpenChange(next);
  };

  const handlePathChange = (value: string) => {
    setTargetPath(value);
    setShowDropdown(value.length > 0);
    mutation.reset();
  };

  const handleSelectProject = (workingDirectory: string) => {
    setTargetPath(workingDirectory);
    setShowDropdown(false);
  };

  const handleSubmit = () => {
    if (!targetPath.trim()) return;
    mutation.mutate(
      { targetProjectPath: targetPath.trim(), sourceSessionId: sessionId },
      {
        onSuccess: (session) => {
          setSuccessSession({ title: session.title ?? null });
        },
      }
    );
  };

  const badgeLabel = getVerificationBadgeLabel(verificationStatus);
  const isPending = mutation.isPending;
  const hasError = mutation.isError;

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent
        className="max-w-md"
        style={{
          backgroundColor: "var(--bg-elevated)",
          borderColor: "var(--border-subtle)",
        }}
      >
        <DialogHeader>
          <DialogTitle style={{ color: "var(--text-primary)" }}>
            Export Plan to Project
          </DialogTitle>
          <DialogDescription style={{ color: "var(--text-muted)" }}>
            Create a copy of this verified plan in another project.
          </DialogDescription>
        </DialogHeader>

        {/* Source plan info */}
        <div
          className="rounded-lg px-3 py-2.5 text-sm mt-1"
          style={{
            backgroundColor: "var(--bg-surface)",
            borderColor: "var(--border-subtle)",
            border: "1px solid var(--border-subtle)",
          }}
        >
          <div className="flex items-center gap-2">
            <span
              className="font-medium truncate flex-1"
              style={{ color: "var(--text-primary)" }}
            >
              {sessionTitle ?? "Untitled plan"}
            </span>
            <span
              className="shrink-0 text-xs px-1.5 py-0.5 rounded"
              style={{
                backgroundColor: "color-mix(in srgb, var(--accent-primary) 15%, transparent)",
                color: "var(--accent-primary)",
              }}
            >
              {badgeLabel}
            </span>
          </div>
        </div>

        {successSession ? (
          /* Success state */
          <div
            className="flex flex-col items-center text-center py-6 gap-3"
          >
            <CheckCircle2
              className="w-10 h-10"
              style={{ color: "var(--accent-primary)" }}
            />
            <p
              className="font-medium text-sm"
              style={{ color: "var(--text-primary)" }}
            >
              Plan exported successfully
            </p>
            {successSession.title && (
              <p className="text-xs" style={{ color: "var(--text-muted)" }}>
                New session: &ldquo;{successSession.title}&rdquo;
              </p>
            )}
            <button
              onClick={() => handleOpenChange(false)}
              className="mt-2 text-xs px-3 py-1.5 rounded-md transition-colors"
              style={{
                backgroundColor: "var(--bg-surface)",
                color: "var(--text-secondary)",
                border: "1px solid var(--border-subtle)",
              }}
            >
              Close
            </button>
          </div>
        ) : (
          <>
            {/* Path input */}
            <div className="relative mt-1">
              <FolderOpen
                className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 pointer-events-none"
                style={{ color: "var(--text-muted)" }}
              />
              <Input
                placeholder="/path/to/project"
                value={targetPath}
                onChange={(e) => handlePathChange(e.target.value)}
                onFocus={() => {
                  if (targetPath.length > 0) setShowDropdown(true);
                }}
                className="pl-9 h-9 text-sm bg-[var(--bg-surface)] border-[var(--border-subtle)] text-[var(--text-primary)] placeholder:text-[var(--text-muted)] outline-none ring-0 focus:ring-0 focus:outline-none focus-visible:outline-none"
                style={{ boxShadow: "none", outline: "none" }}
                disabled={isPending}
              />

              {/* Autocomplete dropdown */}
              {showDropdown && matchingProjects.length > 0 && (
                <div
                  className="absolute top-full left-0 right-0 mt-1 rounded-lg overflow-hidden z-50 shadow-lg"
                  style={{
                    backgroundColor: "var(--bg-elevated)",
                    border: "1px solid var(--border-subtle)",
                  }}
                >
                  {matchingProjects.map((project) => (
                    <button
                      key={project.id}
                      onClick={() =>
                        handleSelectProject(project.workingDirectory)
                      }
                      className="w-full text-left px-3 py-2 text-sm hover:bg-[var(--bg-hover)] transition-colors"
                    >
                      <div
                        className="font-medium truncate"
                        style={{ color: "var(--text-primary)" }}
                      >
                        {project.name}
                      </div>
                      <div
                        className="text-xs truncate"
                        style={{ color: "var(--text-muted)" }}
                      >
                        {project.workingDirectory}
                      </div>
                    </button>
                  ))}
                </div>
              )}
            </div>

            {/* Error message */}
            {hasError && (
              <p className="text-xs" style={{ color: "hsl(0 72% 60%)" }}>
                {mutation.error?.message ?? "Export failed. Please try again."}
              </p>
            )}

            <DialogFooter>
              <button
                onClick={() => handleOpenChange(false)}
                className="text-sm px-3 py-1.5 rounded-md transition-colors"
                style={{
                  color: "var(--text-secondary)",
                  backgroundColor: "var(--bg-surface)",
                  border: "1px solid var(--border-subtle)",
                }}
                disabled={isPending}
              >
                Cancel
              </button>
              <button
                onClick={handleSubmit}
                disabled={isPending || !targetPath.trim()}
                className="text-sm px-4 py-1.5 rounded-md font-medium transition-opacity disabled:opacity-50"
                style={{
                  backgroundColor: "var(--accent-primary)",
                  color: "white",
                }}
              >
                {isPending ? "Creating..." : "Create in Project"}
              </button>
            </DialogFooter>
          </>
        )}
      </DialogContent>
    </Dialog>
  );
}
