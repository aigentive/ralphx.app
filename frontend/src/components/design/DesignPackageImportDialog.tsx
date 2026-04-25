import { Check, FolderGit2, Loader2, PackagePlus } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import type { ImportDesignSystemPackageInput } from "@/lib/tauri";
import type { Project } from "@/types/project";

interface DesignPackageImportDialogProps {
  isOpen: boolean;
  projects: Project[];
  focusedProjectId: string | null;
  isImporting?: boolean;
  onOpenChange: (open: boolean) => void;
  onImport: (input: ImportDesignSystemPackageInput) => void;
}

export function DesignPackageImportDialog({
  isOpen,
  projects,
  focusedProjectId,
  isImporting = false,
  onOpenChange,
  onImport,
}: DesignPackageImportDialogProps) {
  const initialProjectId = useMemo(
    () =>
      projects.find((project) => project.id === focusedProjectId)?.id ??
      projects[0]?.id ??
      "",
    [focusedProjectId, projects],
  );
  const [attachProjectId, setAttachProjectId] = useState(initialProjectId);
  const [packagePath, setPackagePath] = useState("");
  const [name, setName] = useState("");
  const selectedProject = projects.find((project) => project.id === attachProjectId) ?? null;

  useEffect(() => {
    if (!isOpen) {
      return;
    }
    setAttachProjectId(initialProjectId);
    setPackagePath("");
    setName("");
  }, [initialProjectId, isOpen]);

  const choosePackage = async () => {
    const selected = await open({
      multiple: false,
      filters: [{ name: "RalphX Design Package", extensions: ["zip", "json"] }],
    });
    if (typeof selected === "string") {
      setPackagePath(selected);
    }
  };

  const importPackage = () => {
    const selectedPackagePath = packagePath.trim();
    if (!selectedProject || !selectedPackagePath) {
      return;
    }
    onImport({
      packagePath: selectedPackagePath,
      attachProjectId: selectedProject.id,
      ...(name.trim() ? { name: name.trim() } : {}),
    });
  };

  return (
    <Dialog open={isOpen} onOpenChange={onOpenChange}>
      <DialogContent
        className="max-w-[620px] overflow-hidden"
        data-testid="design-package-import-dialog"
        onOpenAutoFocus={(event) => event.preventDefault()}
      >
        <DialogHeader>
          <DialogTitle>Import package</DialogTitle>
          <DialogDescription>
            Attach an exported RalphX design package to a project.
          </DialogDescription>
        </DialogHeader>

        <div className="max-h-[68vh] overflow-y-auto px-6 py-4 space-y-5">
          <section className="space-y-2">
            <label className="text-[12px] font-medium" style={{ color: "var(--text-muted)" }} htmlFor="design-package-path">
              Package file
            </label>
            <div className="flex gap-2">
              <input
                id="design-package-path"
                value={packagePath}
                readOnly
                className="h-9 min-w-0 flex-1 rounded-md border bg-transparent px-3 text-[13px] outline-none"
                style={{ borderColor: "var(--overlay-weak)", color: "var(--text-primary)" }}
                data-testid="design-import-package-path"
              />
              <Button
                type="button"
                variant="outline"
                className="h-9 shrink-0 gap-2"
                onClick={choosePackage}
                data-testid="design-import-package-choose-file"
              >
                <PackagePlus className="h-4 w-4" />
                Choose
              </Button>
            </div>
          </section>

          <section className="space-y-2">
            <label className="text-[12px] font-medium" style={{ color: "var(--text-muted)" }} htmlFor="design-import-name">
              Name
            </label>
            <input
              id="design-import-name"
              value={name}
              onChange={(event) => setName(event.target.value)}
              className="h-9 w-full rounded-md border bg-transparent px-3 text-[13px] outline-none"
              style={{ borderColor: "var(--overlay-weak)", color: "var(--text-primary)" }}
              data-testid="design-import-name"
            />
          </section>

          <section className="space-y-2">
            <div className="text-[12px] font-medium" style={{ color: "var(--text-muted)" }}>
              Attach project
            </div>
            <div className="grid gap-2 sm:grid-cols-2" data-testid="design-import-project-list">
              {projects.map((project) => {
                const isSelected = project.id === attachProjectId;
                return (
                  <button
                    key={project.id}
                    type="button"
                    onClick={() => setAttachProjectId(project.id)}
                    className="min-h-16 rounded-lg border px-3 py-2 text-left"
                    style={{
                      borderColor: isSelected ? "var(--accent-border)" : "var(--overlay-weak)",
                      background: isSelected ? "var(--accent-muted)" : "transparent",
                    }}
                    data-testid={`design-import-project-${project.id}`}
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
          </section>
        </div>

        <DialogFooter>
          <Button type="button" variant="ghost" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button
            type="button"
            className="gap-2"
            disabled={!selectedProject || !packagePath.trim() || isImporting}
            onClick={importPackage}
            data-testid="design-import-package-submit"
          >
            {isImporting ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <PackagePlus className="h-4 w-4" />
            )}
            Import
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
