import { useEffect, useMemo, useState } from "react";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { Project } from "@/types/project";
import type {
  AgentProvider,
  AgentRuntimeSelection,
} from "@/stores/agentSessionStore";
import {
  AGENT_MODEL_OPTIONS,
  AGENT_PROVIDER_OPTIONS,
  DEFAULT_AGENT_RUNTIME,
  defaultModelForProvider,
  normalizeRuntimeSelection,
} from "./agentOptions";

interface NewAgentDialogProps {
  open: boolean;
  projects: Project[];
  defaultProjectId: string | null;
  defaultRuntime: AgentRuntimeSelection | null;
  onOpenChange: (open: boolean) => void;
  onCreate: (input: {
    projectId: string;
    title: string;
    runtime: AgentRuntimeSelection;
  }) => Promise<void>;
  onCreateProject: () => void;
}

export function NewAgentDialog({
  open,
  projects,
  defaultProjectId,
  defaultRuntime,
  onOpenChange,
  onCreate,
  onCreateProject,
}: NewAgentDialogProps) {
  const [projectId, setProjectId] = useState(defaultProjectId ?? "");
  const [provider, setProvider] = useState<AgentProvider>(
    normalizeRuntimeSelection(defaultRuntime).provider
  );
  const [modelId, setModelId] = useState(normalizeRuntimeSelection(defaultRuntime).modelId);
  const [title, setTitle] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [isCreating, setIsCreating] = useState(false);

  const normalizedRuntime = useMemo(
    () => normalizeRuntimeSelection(defaultRuntime ?? DEFAULT_AGENT_RUNTIME),
    [defaultRuntime]
  );

  useEffect(() => {
    if (!open) {
      return;
    }
    setProjectId(defaultProjectId ?? projects[0]?.id ?? "");
    setProvider(normalizedRuntime.provider);
    setModelId(normalizedRuntime.modelId);
    setTitle("");
    setError(null);
  }, [defaultProjectId, normalizedRuntime, open, projects]);

  const modelOptions = AGENT_MODEL_OPTIONS[provider];

  const handleProviderChange = (nextProvider: AgentProvider) => {
    setProvider(nextProvider);
    setModelId(defaultModelForProvider(nextProvider));
  };

  const handleCreate = async () => {
    if (!projectId) {
      setError("Project is required");
      return;
    }

    setIsCreating(true);
    setError(null);
    try {
      await onCreate({
        projectId,
        title,
        runtime: { provider, modelId },
      });
      onOpenChange(false);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to create agent");
    } finally {
      setIsCreating(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-[520px]">
        <DialogHeader>
          <div>
            <DialogTitle>New Agent</DialogTitle>
            <DialogDescription className="sr-only">
              Create a project-scoped background agent session.
            </DialogDescription>
          </div>
        </DialogHeader>

        <div className="px-6 py-5 space-y-4">
          <div className="space-y-2">
            <Label htmlFor="agent-project">Project</Label>
            <div className="flex items-center gap-2">
              <Select value={projectId} onValueChange={setProjectId} disabled={projects.length === 0}>
                <SelectTrigger id="agent-project" className="h-9">
                  <SelectValue placeholder="Select project" />
                </SelectTrigger>
                <SelectContent>
                  {projects.map((project) => (
                    <SelectItem key={project.id} value={project.id}>
                      {project.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={() => {
                  onOpenChange(false);
                  onCreateProject();
                }}
              >
                New Project
              </Button>
            </div>
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-2">
              <Label htmlFor="agent-provider">Provider</Label>
              <Select value={provider} onValueChange={(value) => handleProviderChange(value as AgentProvider)}>
                <SelectTrigger id="agent-provider" className="h-9">
                  <SelectValue placeholder="Select provider" />
                </SelectTrigger>
                <SelectContent>
                  {AGENT_PROVIDER_OPTIONS.map((option) => (
                    <SelectItem key={option.id} value={option.id}>
                      {option.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            <div className="space-y-2">
              <Label htmlFor="agent-model">Model</Label>
              <Select value={modelId} onValueChange={setModelId}>
                <SelectTrigger id="agent-model" className="h-9">
                  <SelectValue placeholder="Select model" />
                </SelectTrigger>
                <SelectContent>
                  {modelOptions.map((option) => (
                    <SelectItem key={option.id} value={option.id}>
                      {option.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>

          <div className="space-y-2">
            <Label htmlFor="agent-title">Title</Label>
            <Input
              id="agent-title"
              value={title}
              onChange={(event) => setTitle(event.target.value)}
              placeholder="Optional"
            />
          </div>

          {error && (
            <div
              className="rounded-md border px-3 py-2 text-sm"
              style={{
                color: "var(--status-error)",
                background: "var(--status-error-muted)",
                borderColor: "var(--status-error-border)",
              }}
            >
              {error}
            </div>
          )}
        </div>

        <DialogFooter>
          <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button type="button" onClick={handleCreate} disabled={isCreating || !projectId}>
            {isCreating ? "Creating..." : "Create Agent"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
