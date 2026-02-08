/**
 * ProjectAnalysisSection - Displays and manages project analysis settings
 *
 * Features:
 * - Read-only display of detected analysis entries
 * - Editable custom override JSON editor
 * - Re-analyze button to trigger project analyzer agent
 * - Template variables reference
 */

import { useState, useCallback } from "react";
import { Search, Loader2, Trash2, Save, ChevronDown, ChevronRight, Info } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { api } from "@/lib/tauri";
import { useProjectStore, selectActiveProject } from "@/stores/projectStore";
import { SectionCard } from "./SettingsView.shared";

/** Shape of a single analysis entry */
interface AnalysisEntry {
  path: string;
  label: string;
  install: string | null;
  validate: string[];
  worktree_setup: string[];
}

function parseAnalysisEntries(json: string | null): AnalysisEntry[] {
  if (!json) return [];
  try {
    const parsed = JSON.parse(json);
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

function formatTimestamp(iso: string | null): string {
  if (!iso) return "Never";
  try {
    const date = new Date(iso);
    return date.toLocaleString(undefined, {
      year: "numeric",
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  } catch {
    return "Unknown";
  }
}

/** Read-only display of a single analysis entry */
function AnalysisEntryCard({ entry }: { entry: AnalysisEntry }) {
  const [expanded, setExpanded] = useState(false);

  return (
    <div
      className="rounded-md border border-[var(--border-subtle)] overflow-hidden"
      style={{ background: "rgba(255,255,255,0.02)" }}
    >
      <button
        type="button"
        onClick={() => setExpanded(!expanded)}
        className="w-full flex items-center gap-2 px-3 py-2 text-left hover:bg-[rgba(45,45,45,0.3)] transition-colors"
      >
        {expanded ? (
          <ChevronDown className="w-3.5 h-3.5 text-[var(--text-muted)] shrink-0" />
        ) : (
          <ChevronRight className="w-3.5 h-3.5 text-[var(--text-muted)] shrink-0" />
        )}
        <code className="text-xs text-[var(--accent-primary)] font-medium">{entry.path}</code>
        <span className="text-xs text-[var(--text-muted)]">{entry.label}</span>
      </button>
      {expanded && (
        <div className="px-3 pb-3 pt-1 space-y-2 border-t border-[var(--border-subtle)]">
          {entry.install && (
            <div>
              <span className="text-[10px] uppercase tracking-wider text-[var(--text-muted)] font-medium">
                Install
              </span>
              <code className="block text-xs text-[var(--text-secondary)] mt-0.5 bg-[rgba(0,0,0,0.2)] px-2 py-1 rounded">
                {entry.install}
              </code>
            </div>
          )}
          {entry.validate.length > 0 && (
            <div>
              <span className="text-[10px] uppercase tracking-wider text-[var(--text-muted)] font-medium">
                Validate
              </span>
              <div className="mt-0.5 space-y-1">
                {entry.validate.map((cmd, i) => (
                  <code
                    key={i}
                    className="block text-xs text-[var(--text-secondary)] bg-[rgba(0,0,0,0.2)] px-2 py-1 rounded"
                  >
                    {cmd}
                  </code>
                ))}
              </div>
            </div>
          )}
          {entry.worktree_setup.length > 0 && (
            <div>
              <span className="text-[10px] uppercase tracking-wider text-[var(--text-muted)] font-medium">
                Worktree Setup
              </span>
              <div className="mt-0.5 space-y-1">
                {entry.worktree_setup.map((cmd, i) => (
                  <code
                    key={i}
                    className="block text-xs text-[var(--text-secondary)] bg-[rgba(0,0,0,0.2)] px-2 py-1 rounded"
                  >
                    {cmd}
                  </code>
                ))}
              </div>
            </div>
          )}
          {!entry.install && entry.validate.length === 0 && entry.worktree_setup.length === 0 && (
            <p className="text-xs text-[var(--text-muted)] italic">No commands configured</p>
          )}
        </div>
      )}
    </div>
  );
}

/** Template variables reference panel */
function TemplateVariablesInfo() {
  const [expanded, setExpanded] = useState(false);

  return (
    <div className="mt-3">
      <button
        type="button"
        onClick={() => setExpanded(!expanded)}
        className="flex items-center gap-1.5 text-xs text-[var(--text-muted)] hover:text-[var(--text-secondary)] transition-colors"
      >
        <Info className="w-3 h-3" />
        <span>Template Variables</span>
        {expanded ? <ChevronDown className="w-3 h-3" /> : <ChevronRight className="w-3 h-3" />}
      </button>
      {expanded && (
        <div
          className="mt-2 rounded-md px-3 py-2 space-y-1"
          style={{
            background: "rgba(255,107,53,0.05)",
            border: "1px solid rgba(255,107,53,0.15)",
          }}
        >
          <div className="flex items-start gap-2">
            <code className="text-[11px] text-[var(--accent-primary)] whitespace-nowrap">{"{project_root}"}</code>
            <span className="text-[11px] text-[var(--text-muted)]">Project working directory</span>
          </div>
          <div className="flex items-start gap-2">
            <code className="text-[11px] text-[var(--accent-primary)] whitespace-nowrap">{"{worktree_path}"}</code>
            <span className="text-[11px] text-[var(--text-muted)]">Task worktree directory (when available)</span>
          </div>
          <div className="flex items-start gap-2">
            <code className="text-[11px] text-[var(--accent-primary)] whitespace-nowrap">{"{task_branch}"}</code>
            <span className="text-[11px] text-[var(--text-muted)]">Task branch name (when available)</span>
          </div>
        </div>
      )}
    </div>
  );
}

export function ProjectAnalysisSection() {
  const project = useProjectStore(selectActiveProject);
  const updateProject = useProjectStore((s) => s.updateProject);

  const [isAnalyzing, setIsAnalyzing] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [customJson, setCustomJson] = useState<string | null>(null);
  const [jsonError, setJsonError] = useState<string | null>(null);

  // Derived state
  const detectedEntries = parseAnalysisEntries(project?.detectedAnalysis ?? null);
  const hasCustomOverride = project?.customAnalysis != null;
  const activeEntries = hasCustomOverride
    ? parseAnalysisEntries(project?.customAnalysis ?? null)
    : detectedEntries;

  // The text in the editor: local edits if editing, else project's custom_analysis
  const editorValue = customJson ?? project?.customAnalysis ?? "";

  const handleReanalyze = useCallback(async () => {
    if (!project) return;
    setIsAnalyzing(true);
    try {
      await api.projects.reanalyzeProject(project.id);
      toast.success("Re-analysis started. Results will appear shortly.");
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Failed to start re-analysis");
    } finally {
      setIsAnalyzing(false);
    }
  }, [project]);

  const handleSaveCustom = useCallback(async () => {
    if (!project) return;
    const value = (customJson ?? "").trim();

    // Validate JSON
    if (value) {
      try {
        const parsed = JSON.parse(value);
        if (!Array.isArray(parsed)) {
          setJsonError("Must be a JSON array");
          return;
        }
      } catch (e) {
        setJsonError(e instanceof Error ? e.message : "Invalid JSON");
        return;
      }
    }

    setJsonError(null);
    setIsSaving(true);
    try {
      const updated = await api.projects.updateCustomAnalysis(project.id, value || null);
      updateProject(project.id, {
        customAnalysis: updated.customAnalysis,
        updatedAt: updated.updatedAt,
      });
      setCustomJson(null);
      toast.success(value ? "Custom analysis saved" : "Custom analysis cleared");
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Failed to save custom analysis");
    } finally {
      setIsSaving(false);
    }
  }, [project, customJson, updateProject]);

  const handleClearCustom = useCallback(async () => {
    if (!project) return;
    setIsSaving(true);
    setJsonError(null);
    try {
      const updated = await api.projects.updateCustomAnalysis(project.id, null);
      updateProject(project.id, {
        customAnalysis: updated.customAnalysis,
        updatedAt: updated.updatedAt,
      });
      setCustomJson(null);
      toast.success("Custom override cleared, using detected analysis");
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Failed to clear custom analysis");
    } finally {
      setIsSaving(false);
    }
  }, [project, updateProject]);

  if (!project) return null;

  return (
    <SectionCard
      icon={<Search className="w-[18px] h-[18px] text-[var(--accent-primary)]" />}
      title="Project Analysis"
      description="Build system detection and validation commands"
    >
      {/* Header: Last Analyzed + Re-analyze */}
      <div className="flex items-center justify-between py-2">
        <span className="text-xs text-[var(--text-muted)]">
          Last Analyzed: {formatTimestamp(project.analyzedAt)}
        </span>
        <Button
          variant="ghost"
          size="sm"
          onClick={handleReanalyze}
          disabled={isAnalyzing}
          className="h-7 px-2.5 text-xs text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-surface-hover)]"
        >
          {isAnalyzing ? (
            <Loader2 className="w-3.5 h-3.5 animate-spin mr-1" />
          ) : (
            <Search className="w-3.5 h-3.5 mr-1" />
          )}
          Re-analyze
        </Button>
      </div>

      {/* Detected Analysis (read-only) */}
      <div className="py-2 border-t border-[var(--border-subtle)]">
        <div className="flex items-center justify-between mb-2">
          <h4 className="text-xs font-medium text-[var(--text-primary)]">
            {hasCustomOverride ? "Active (Custom Override)" : "Detected Analysis"}
          </h4>
          {hasCustomOverride && (
            <span
              className="text-[10px] px-1.5 py-0.5 rounded"
              style={{
                background: "rgba(255,107,53,0.12)",
                color: "var(--accent-primary)",
              }}
            >
              Override Active
            </span>
          )}
        </div>
        {activeEntries.length > 0 ? (
          <div className="space-y-1.5">
            {activeEntries.map((entry, i) => (
              <AnalysisEntryCard key={`${entry.path}-${i}`} entry={entry} />
            ))}
          </div>
        ) : (
          <p className="text-xs text-[var(--text-muted)] italic py-2">
            {project.analyzedAt
              ? "No build systems detected"
              : "Not yet analyzed. Click Re-analyze to detect build systems."}
          </p>
        )}
      </div>

      {/* Custom Override Editor */}
      <div className="py-2 border-t border-[var(--border-subtle)]">
        <div className="flex items-center justify-between mb-2">
          <h4 className="text-xs font-medium text-[var(--text-primary)]">Custom Override</h4>
          <div className="flex items-center gap-1.5">
            {(customJson != null || hasCustomOverride) && (
              <Button
                variant="ghost"
                size="sm"
                onClick={handleClearCustom}
                disabled={isSaving}
                className="h-6 px-2 text-[10px] text-[var(--text-muted)] hover:text-[var(--status-error)] hover:bg-[rgba(239,68,68,0.08)]"
              >
                <Trash2 className="w-3 h-3 mr-0.5" />
                Clear
              </Button>
            )}
            {customJson != null && (
              <Button
                variant="ghost"
                size="sm"
                onClick={handleSaveCustom}
                disabled={isSaving}
                className="h-6 px-2 text-[10px] text-[var(--accent-primary)] hover:bg-[rgba(255,107,53,0.08)]"
              >
                {isSaving ? (
                  <Loader2 className="w-3 h-3 animate-spin mr-0.5" />
                ) : (
                  <Save className="w-3 h-3 mr-0.5" />
                )}
                Save
              </Button>
            )}
          </div>
        </div>
        <p className="text-[11px] text-[var(--text-muted)] mb-2">
          Set custom commands to override detected analysis. JSON array format.
        </p>
        <textarea
          value={editorValue}
          onChange={(e) => {
            setCustomJson(e.target.value);
            setJsonError(null);
          }}
          placeholder={`[\n  {\n    "path": ".",\n    "label": "Node.js",\n    "install": "npm install",\n    "validate": ["npm run typecheck"],\n    "worktree_setup": []\n  }\n]`}
          rows={6}
          className="w-full rounded-md px-3 py-2 text-xs font-mono bg-[var(--bg-surface)] border border-[var(--border-default)] text-[var(--text-secondary)] placeholder:text-[var(--text-muted)] resize-y outline-none ring-0 focus:ring-0 focus:outline-none focus-visible:outline-none focus:border-[var(--accent-primary)]"
          style={{ boxShadow: "none", outline: "none" }}
        />
        {jsonError && (
          <p className="text-[11px] text-[var(--status-error)] mt-1">{jsonError}</p>
        )}
      </div>

      {/* Template Variables Reference */}
      <TemplateVariablesInfo />
    </SectionCard>
  );
}
