import { Check, FolderGit2, Plus } from "lucide-react";
import { useEffect, useMemo, useState } from "react";

import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import type { CreateDesignSystemInput } from "@/lib/tauri";
import type { Project } from "@/types/project";

interface DesignSourceComposerDialogProps {
  isOpen: boolean;
  projects: Project[];
  focusedProjectId: string | null;
  isCreating?: boolean;
  createError?: string | null;
  onOpenChange: (open: boolean) => void;
  onCreate: (input: CreateDesignSystemInput) => void;
}

export function DesignSourceComposerDialog({
  isOpen,
  projects,
  focusedProjectId,
  isCreating = false,
  createError = null,
  onOpenChange,
  onCreate,
}: DesignSourceComposerDialogProps) {
  const initialPrimaryProjectId = useMemo(
    () =>
      projects.find((project) => project.id === focusedProjectId)?.id ??
      projects[0]?.id ??
      "",
    [focusedProjectId, projects],
  );
  const [primaryProjectId, setPrimaryProjectId] = useState(initialPrimaryProjectId);
  const primaryProject = projects.find((project) => project.id === primaryProjectId) ?? null;
  const [name, setName] = useState("");
  const [didEditName, setDidEditName] = useState(false);
  const [primaryPaths, setPrimaryPaths] = useState("");
  const [referenceProjectIds, setReferenceProjectIds] = useState<string[]>([]);
  const [referencePaths, setReferencePaths] = useState<Record<string, string>>({});

  useEffect(() => {
    if (!isOpen) {
      return;
    }
    setPrimaryProjectId(initialPrimaryProjectId);
    setName(projects.find((project) => project.id === initialPrimaryProjectId)?.name
      ? `${projects.find((project) => project.id === initialPrimaryProjectId)?.name} Design System`
      : "Design System");
    setDidEditName(false);
    setPrimaryPaths("");
    setReferenceProjectIds([]);
    setReferencePaths({});
  }, [initialPrimaryProjectId, isOpen, projects]);

  useEffect(() => {
    if (!primaryProject || didEditName) {
      return;
    }
    setName(`${primaryProject.name} Design System`);
  }, [didEditName, primaryProject]);

  const referenceProjects = projects.filter((project) => project.id !== primaryProjectId);

  const toggleReferenceProject = (projectId: string, checked: boolean) => {
    setReferenceProjectIds((current) => {
      if (checked) {
        return current.includes(projectId) ? current : [...current, projectId];
      }
      return current.filter((id) => id !== projectId);
    });
  };

  const createDesignSystem = () => {
    if (!primaryProject) {
      return;
    }
    onCreate({
      primaryProjectId: primaryProject.id,
      name: name.trim() || `${primaryProject.name} Design System`,
      selectedPaths: parsePathList(primaryPaths),
      sources: referenceProjectIds.map((projectId) => ({
        projectId,
        role: "reference",
        selectedPaths: parsePathList(referencePaths[projectId] ?? ""),
      })),
    });
  };

  return (
    <Dialog open={isOpen} onOpenChange={onOpenChange}>
      <DialogContent
        className="max-w-[680px] overflow-hidden"
        data-testid="design-source-composer"
        onOpenAutoFocus={(event) => event.preventDefault()}
      >
        <DialogHeader>
          <div className="min-w-0">
            <DialogTitle>Create design system</DialogTitle>
            <DialogDescription>
              Select source projects and optional source paths.
            </DialogDescription>
          </div>
        </DialogHeader>

        <div className="max-h-[68vh] overflow-y-auto px-6 py-4 space-y-5">
          <section className="space-y-2">
            <label className="text-[12px] font-medium" style={{ color: "var(--text-muted)" }} htmlFor="design-system-name">
              Name
            </label>
            <input
              id="design-system-name"
              value={name}
              onChange={(event) => {
                setDidEditName(true);
                setName(event.target.value);
              }}
              className="h-9 w-full rounded-md border bg-transparent px-3 text-[13px] outline-none"
              style={{ borderColor: "var(--overlay-weak)", color: "var(--text-primary)" }}
              data-testid="design-source-name"
            />
          </section>

          <section className="space-y-2">
            <div className="text-[12px] font-medium" style={{ color: "var(--text-muted)" }}>
              Primary source
            </div>
            <div className="grid gap-2 sm:grid-cols-2" data-testid="design-primary-source-list">
              {projects.map((project) => {
                const isSelected = project.id === primaryProjectId;
                return (
                  <button
                    key={project.id}
                    type="button"
                    onClick={() => setPrimaryProjectId(project.id)}
                    className="min-h-16 rounded-lg border px-3 py-2 text-left"
                    style={{
                      borderColor: isSelected ? "var(--accent-border)" : "var(--overlay-weak)",
                      background: isSelected ? "var(--accent-muted)" : "transparent",
                    }}
                    data-testid={`design-primary-source-${project.id}`}
                  >
                    <div className="flex items-center gap-2">
                      <FolderGit2 className="h-4 w-4 shrink-0" style={{ color: "var(--accent-primary)" }} />
                      <span className="min-w-0 flex-1 truncate text-[13px] font-medium" style={{ color: "var(--text-primary)" }}>
                        {project.name}
                      </span>
                      {isSelected && <Check className="h-4 w-4 shrink-0" style={{ color: "var(--accent-primary)" }} />}
                    </div>
                    <div className="mt-1 truncate text-[11px]" style={{ color: "var(--text-muted)" }}>
                      {project.workingDirectory}
                    </div>
                  </button>
                );
              })}
            </div>
            <PathTextarea
              id="design-primary-paths"
              label="Primary source paths"
              value={primaryPaths}
              onChange={setPrimaryPaths}
              testId="design-primary-paths"
            />
          </section>

          <section className="space-y-2">
            <div className="text-[12px] font-medium" style={{ color: "var(--text-muted)" }}>
              Reference sources
            </div>
            <div className="space-y-2" data-testid="design-reference-source-list">
              {referenceProjects.map((project) => {
                const isChecked = referenceProjectIds.includes(project.id);
                return (
                  <div
                    key={project.id}
                    className="rounded-lg border px-3 py-2"
                    style={{ borderColor: "var(--overlay-weak)" }}
                  >
                    <label className="flex items-start gap-2">
                      <Checkbox
                        checked={isChecked}
                        onCheckedChange={(checked) => toggleReferenceProject(project.id, checked === true)}
                        data-testid={`design-reference-source-${project.id}`}
                      />
                      <span className="min-w-0 flex-1">
                        <span className="block truncate text-[13px] font-medium" style={{ color: "var(--text-primary)" }}>
                          {project.name}
                        </span>
                        <span className="block truncate text-[11px]" style={{ color: "var(--text-muted)" }}>
                          {project.workingDirectory}
                        </span>
                      </span>
                    </label>
                    {isChecked && (
                      <PathTextarea
                        id={`design-reference-paths-${project.id}`}
                        label="Reference source paths"
                        value={referencePaths[project.id] ?? ""}
                        onChange={(value) =>
                          setReferencePaths((current) => ({ ...current, [project.id]: value }))
                        }
                        testId={`design-reference-paths-${project.id}`}
                      />
                    )}
                  </div>
                );
              })}
            </div>
          </section>
        </div>

        {createError && (
          <div
            role="alert"
            className="mx-6 mb-4 rounded-md border px-3 py-2 text-[12px]"
            style={{
              borderColor: "var(--status-error)",
              background: "color-mix(in srgb, var(--status-error) 12%, transparent)",
              color: "var(--text-primary)",
            }}
            data-testid="design-source-create-error"
          >
            {createError}
          </div>
        )}

        <DialogFooter>
          <Button type="button" variant="ghost" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button
            type="button"
            className="gap-2"
            disabled={!primaryProject || isCreating}
            onClick={createDesignSystem}
            data-testid="design-create-from-sources"
          >
            <Plus className="h-4 w-4" />
            Create
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function PathTextarea({
  id,
  label,
  value,
  onChange,
  testId,
}: {
  id: string;
  label: string;
  value: string;
  onChange: (value: string) => void;
  testId: string;
}) {
  return (
    <div className="pt-2">
      <label className="text-[11px] font-medium" style={{ color: "var(--text-muted)" }} htmlFor={id}>
        {label}
      </label>
      <textarea
        id={id}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className="mt-1 min-h-16 w-full rounded-md border bg-transparent px-3 py-2 text-[12px] outline-none"
        style={{ borderColor: "var(--overlay-weak)", color: "var(--text-primary)" }}
        placeholder="frontend/src, frontend/src/components"
        data-testid={testId}
      />
    </div>
  );
}

function parsePathList(value: string): string[] {
  const seen = new Set<string>();
  return value
    .split(/[\n,]/)
    .map((path) => path.trim())
    .filter((path) => {
      if (!path || seen.has(path)) {
        return false;
      }
      seen.add(path);
      return true;
    });
}
