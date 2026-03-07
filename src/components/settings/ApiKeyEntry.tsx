/**
 * ApiKeyEntry - Expandable per-key row with full details and management actions.
 *
 * Collapsed: key name, prefix, dates, expand toggle.
 * Expanded: project scoping, permissions editing, audit log, rotate/revoke actions.
 */

import { useState, useCallback } from "react";
import {
  Key,
  Trash2,
  AlertTriangle,
  ChevronDown,
  ChevronUp,
  RotateCcw,
  Save,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { AuditLogViewer } from "./AuditLogViewer";
import { PermissionsBitmask } from "./PermissionsBitmask";
import { ProjectMultiSelect } from "./ProjectMultiSelect";
import { RotateKeyDialog } from "./RotateKeyDialog";
import {
  useRevokeApiKey,
  useUpdateKeyProjects,
  useUpdateKeyPermissions,
} from "@/hooks/useApiKeys";
import type { ApiKey } from "@/types/api-key";

// ============================================================================
// Props
// ============================================================================

export interface ApiKeyEntryProps {
  apiKey: ApiKey;
  onKeyChanged: () => void;
}

// ============================================================================
// Helpers
// ============================================================================

function formatDate(dateStr: string | null): string {
  if (!dateStr) return "Never";
  const d = new Date(dateStr);
  return d.toLocaleDateString("en-US", {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}

// ============================================================================
// Sub-components
// ============================================================================

interface SectionLabelProps {
  children: React.ReactNode;
}

function SectionLabel({ children }: SectionLabelProps) {
  return (
    <p className="text-xs font-medium text-[var(--text-muted)] uppercase tracking-wide mb-1.5">
      {children}
    </p>
  );
}

// ============================================================================
// Component
// ============================================================================

export function ApiKeyEntry({ apiKey, onKeyChanged }: ApiKeyEntryProps) {
  const [expanded, setExpanded] = useState(false);

  // Revoke state (two-click)
  const [confirmPending, setConfirmPending] = useState(false);
  const [revokeError, setRevokeError] = useState<string | null>(null);
  const revokeMutation = useRevokeApiKey();

  // Project editing state
  const [projectIds, setProjectIds] = useState<string[]>(apiKey.project_ids);
  const [projectSaveError, setProjectSaveError] = useState<string | null>(null);
  const updateProjectsMutation = useUpdateKeyProjects();

  // Permissions editing state
  const [permissions, setPermissions] = useState<number>(apiKey.permissions);
  const [permSaveError, setPermSaveError] = useState<string | null>(null);
  const updatePermsMutation = useUpdateKeyPermissions();

  // Rotate dialog
  const [rotateOpen, setRotateOpen] = useState(false);

  // ---- Revoke handlers ----

  const handleRevokeClick = useCallback(() => {
    if (!confirmPending) {
      setConfirmPending(true);
      return;
    }
    setRevokeError(null);
    revokeMutation.mutate(apiKey.id, {
      onSuccess: () => {
        onKeyChanged();
      },
      onError: (err) => {
        setRevokeError(err instanceof Error ? err.message : "Failed to revoke key");
        setConfirmPending(false);
      },
    });
  }, [apiKey.id, confirmPending, onKeyChanged, revokeMutation]);

  const handleCancelConfirm = useCallback(() => {
    setConfirmPending(false);
  }, []);

  // ---- Project save handler ----

  const handleSaveProjects = useCallback(() => {
    setProjectSaveError(null);
    updateProjectsMutation.mutate(
      { id: apiKey.id, projectIds },
      {
        onError: (err) => {
          setProjectSaveError(
            err instanceof Error ? err.message : "Failed to update projects"
          );
        },
      }
    );
  }, [apiKey.id, projectIds, updateProjectsMutation]);

  // ---- Permissions save handler ----

  const handleSavePermissions = useCallback(() => {
    setPermSaveError(null);
    updatePermsMutation.mutate(
      { id: apiKey.id, permissions },
      {
        onError: (err) => {
          setPermSaveError(
            err instanceof Error ? err.message : "Failed to update permissions"
          );
        },
      }
    );
  }, [apiKey.id, permissions, updatePermsMutation]);

  // ---- Rotate handlers ----

  const handleRotated = useCallback(() => {
    onKeyChanged();
  }, [onKeyChanged]);

  return (
    <div
      className="border-b border-[var(--border-subtle)] last:border-0"
      data-testid={`api-key-entry-${apiKey.id}`}
    >
      {/* ---- Collapsed row ---- */}
      <div className="flex items-start justify-between py-3">
        {/* Key info */}
        <div className="flex items-start gap-3 flex-1 min-w-0">
          <div
            className="p-1.5 rounded-md shrink-0 mt-0.5"
            style={{
              background: "rgba(255,107,53,0.08)",
              border: "1px solid rgba(255,107,53,0.15)",
            }}
          >
            <Key className="w-3.5 h-3.5 text-[var(--accent-primary)]" />
          </div>
          <div className="min-w-0 flex-1">
            <p className="text-sm font-medium text-[var(--text-primary)] truncate">
              {apiKey.name}
            </p>
            <p className="text-xs text-[var(--text-muted)] font-mono mt-0.5">
              {apiKey.key_prefix}...
            </p>
            <div className="flex items-center gap-3 mt-1">
              <span className="text-xs text-[var(--text-muted)]">
                Created {formatDate(apiKey.created_at)}
              </span>
              <span className="text-[var(--border-subtle)]">·</span>
              <span className="text-xs text-[var(--text-muted)]">
                Last used {formatDate(apiKey.last_used_at)}
              </span>
            </div>
          </div>
        </div>

        {/* Expand toggle */}
        <Button
          variant="ghost"
          size="sm"
          onClick={() => setExpanded((v) => !v)}
          data-testid={`expand-key-${apiKey.id}`}
          aria-expanded={expanded}
          className="h-7 w-7 p-0 ml-2 shrink-0 text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-surface-hover)]"
        >
          {expanded ? (
            <ChevronUp className="w-3.5 h-3.5" />
          ) : (
            <ChevronDown className="w-3.5 h-3.5" />
          )}
        </Button>
      </div>

      {/* ---- Expanded details ---- */}
      {expanded && (
        <div className="pb-4 px-1 space-y-4">
          {/* Projects */}
          <div>
            <SectionLabel>Project Access</SectionLabel>
            <ProjectMultiSelect
              selectedIds={projectIds}
              onChange={setProjectIds}
              disabled={updateProjectsMutation.isPending}
            />
            {projectSaveError && (
              <p className="text-xs text-red-400 mt-1 flex items-center gap-1">
                <AlertTriangle className="w-3 h-3 shrink-0" />
                {projectSaveError}
              </p>
            )}
            <Button
              variant="ghost"
              size="sm"
              onClick={handleSaveProjects}
              disabled={updateProjectsMutation.isPending}
              data-testid={`save-projects-${apiKey.id}`}
              className="mt-2 h-7 px-2 text-xs text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-surface-hover)] gap-1"
            >
              <Save className="w-3 h-3" />
              {updateProjectsMutation.isPending ? "Saving…" : "Save Projects"}
            </Button>
          </div>

          {/* Permissions */}
          <div>
            <SectionLabel>Permissions</SectionLabel>
            <PermissionsBitmask
              value={permissions}
              onChange={setPermissions}
              disabled={updatePermsMutation.isPending}
            />
            {permSaveError && (
              <p className="text-xs text-red-400 mt-1 flex items-center gap-1">
                <AlertTriangle className="w-3 h-3 shrink-0" />
                {permSaveError}
              </p>
            )}
            <Button
              variant="ghost"
              size="sm"
              onClick={handleSavePermissions}
              disabled={updatePermsMutation.isPending}
              data-testid={`save-permissions-${apiKey.id}`}
              className="mt-2 h-7 px-2 text-xs text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-surface-hover)] gap-1"
            >
              <Save className="w-3 h-3" />
              {updatePermsMutation.isPending ? "Saving…" : "Save Permissions"}
            </Button>
          </div>

          {/* Audit log */}
          <div>
            <SectionLabel>Recent Requests</SectionLabel>
            <AuditLogViewer keyId={apiKey.id} />
          </div>

          {/* Actions */}
          <div className="pt-2 border-t border-[var(--border-subtle)] flex items-center gap-2">
            {/* Rotate */}
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setRotateOpen(true)}
              data-testid={`rotate-key-${apiKey.id}`}
              className="h-7 px-2 text-xs text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-surface-hover)] gap-1"
            >
              <RotateCcw className="w-3 h-3" />
              Rotate Key
            </Button>

            <div className="flex-1" />

            {/* Revoke */}
            {confirmPending && !revokeMutation.isPending && (
              <Button
                variant="ghost"
                size="sm"
                onClick={handleCancelConfirm}
                className="h-7 px-2 text-xs text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-surface-hover)]"
              >
                Cancel
              </Button>
            )}
            <Button
              variant="ghost"
              size="sm"
              onClick={handleRevokeClick}
              disabled={revokeMutation.isPending}
              data-testid={`revoke-key-${apiKey.id}`}
              className={
                confirmPending
                  ? "h-7 px-2 text-xs text-red-400 hover:text-red-300 hover:bg-red-500/10 border border-red-500/30"
                  : "h-7 px-2 text-xs text-[var(--text-muted)] hover:text-red-400 hover:bg-red-500/10"
              }
            >
              {revokeMutation.isPending ? (
                <span>Revoking...</span>
              ) : confirmPending ? (
                <>
                  <AlertTriangle className="w-3 h-3 mr-1" />
                  Confirm?
                </>
              ) : (
                <>
                  <Trash2 className="w-3 h-3 mr-1" />
                  Revoke
                </>
              )}
            </Button>

            {revokeError && (
              <p className="text-xs text-red-400 flex items-center gap-1">
                <AlertTriangle className="w-3 h-3 shrink-0" />
                {revokeError}
              </p>
            )}
          </div>
        </div>
      )}

      {/* Rotate dialog */}
      <RotateKeyDialog
        open={rotateOpen}
        keyId={apiKey.id}
        keyName={apiKey.name}
        onClose={() => setRotateOpen(false)}
        onRotated={handleRotated}
      />
    </div>
  );
}
