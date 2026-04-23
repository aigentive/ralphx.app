import { useEffect, useMemo, useState } from "react";

import type { Project } from "@/types/project";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
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

interface AgentsStartComposerProps {
  projects: Project[];
  defaultProjectId: string | null;
  defaultRuntime: AgentRuntimeSelection | null;
  isLoadingProjects: boolean;
  isSubmitting: boolean;
  onCreateProject: () => void;
  onSubmit: (input: {
    projectId: string;
    content: string;
    runtime: AgentRuntimeSelection;
  }) => Promise<void>;
}

export function AgentsStartComposer({
  projects,
  defaultProjectId,
  defaultRuntime,
  isLoadingProjects,
  isSubmitting,
  onCreateProject,
  onSubmit,
}: AgentsStartComposerProps) {
  const [projectId, setProjectId] = useState(defaultProjectId ?? "");
  const [provider, setProvider] = useState<AgentProvider>(
    normalizeRuntimeSelection(defaultRuntime).provider
  );
  const [modelId, setModelId] = useState(normalizeRuntimeSelection(defaultRuntime).modelId);
  const [content, setContent] = useState("");
  const [error, setError] = useState<string | null>(null);

  const normalizedRuntime = useMemo(
    () => normalizeRuntimeSelection(defaultRuntime ?? DEFAULT_AGENT_RUNTIME),
    [defaultRuntime]
  );

  useEffect(() => {
    setProjectId(defaultProjectId ?? projects[0]?.id ?? "");
  }, [defaultProjectId, projects]);

  useEffect(() => {
    setProvider(normalizedRuntime.provider);
    setModelId(normalizedRuntime.modelId);
  }, [normalizedRuntime]);

  const modelOptions = AGENT_MODEL_OPTIONS[provider];
  const canSubmit =
    !isSubmitting && !isLoadingProjects && projectId.trim().length > 0 && content.trim().length > 0;

  const handleProviderChange = (nextProvider: AgentProvider) => {
    setProvider(nextProvider);
    setModelId(defaultModelForProvider(nextProvider));
  };

  const handleSubmit = async () => {
    const trimmedContent = content.trim();
    if (!projectId) {
      setError("Project is required");
      return;
    }
    if (!trimmedContent) {
      setError("Prompt is required");
      return;
    }

    setError(null);
    try {
      await onSubmit({
        projectId,
        content: trimmedContent,
        runtime: { provider, modelId },
      });
      setContent("");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to start agent conversation");
    }
  };

  return (
    <div className="flex h-full w-full items-center justify-center px-8 py-10">
      <div className="w-full max-w-[880px]">
        <div className="mb-6 text-center">
          <h2
            className="text-[30px] font-semibold tracking-[-0.03em]"
            style={{ color: "var(--text-primary)" }}
          >
            Start a new agent conversation
          </h2>
          <p className="mt-2 text-sm" style={{ color: "var(--text-muted)" }}>
            Pick the project and runtime, then send the first prompt directly.
          </p>
        </div>

        <div
          className="rounded-[28px] border p-4 shadow-[var(--shadow-lg)]"
          style={{
            background:
              "linear-gradient(180deg, color-mix(in srgb, var(--bg-surface) 96%, transparent), var(--bg-surface))",
            borderColor: "var(--overlay-weak)",
          }}
          data-testid="agents-start-composer"
        >
          <Textarea
            value={content}
            onChange={(event) => setContent(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter" && !event.shiftKey) {
                event.preventDefault();
                void handleSubmit();
              }
            }}
            placeholder="Describe what you want the agent to work on"
            className="min-h-[150px] resize-none border-0 bg-transparent px-2 py-3 text-[18px] shadow-none focus-visible:ring-0"
            style={{
              color: "var(--text-primary)",
              boxShadow: "none",
            }}
            data-testid="agents-start-textarea"
            autoFocus
          />

          <div
            className="mt-3 rounded-[20px] border px-4 py-3"
            style={{
              background: "color-mix(in srgb, var(--bg-base) 62%, transparent)",
              borderColor: "var(--overlay-faint)",
            }}
          >
            <div className="grid gap-3 md:grid-cols-[minmax(0,1.3fr)_minmax(180px,0.9fr)_minmax(200px,1fr)_auto_auto]">
              <div className="space-y-2">
                <Label htmlFor="agents-start-project">Project</Label>
                <Select value={projectId} onValueChange={setProjectId} disabled={projects.length === 0}>
                  <SelectTrigger
                    id="agents-start-project"
                    className="h-10"
                    data-testid="agents-start-project"
                  >
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
              </div>

              <div className="space-y-2">
                <Label htmlFor="agents-start-provider">Provider</Label>
                <Select
                  value={provider}
                  onValueChange={(value) => handleProviderChange(value as AgentProvider)}
                >
                  <SelectTrigger
                    id="agents-start-provider"
                    className="h-10"
                    data-testid="agents-start-provider"
                  >
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
                <Label htmlFor="agents-start-model">Model</Label>
                <Select value={modelId} onValueChange={setModelId}>
                  <SelectTrigger
                    id="agents-start-model"
                    className="h-10"
                    data-testid="agents-start-model"
                  >
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

              <div className="flex items-end">
                <Button
                  type="button"
                  variant="outline"
                  className="h-10 w-full md:w-auto"
                  onClick={onCreateProject}
                  data-testid="agents-start-new-project"
                >
                  New Project
                </Button>
              </div>

              <div className="flex items-end">
                <Button
                  type="button"
                  className="h-10 w-full min-w-[132px] md:w-auto"
                  onClick={() => void handleSubmit()}
                  disabled={!canSubmit}
                  data-testid="agents-start-submit"
                >
                  {isSubmitting ? "Starting..." : "Start Agent"}
                </Button>
              </div>
            </div>
          </div>

          {error && (
            <div
              className="mt-3 rounded-xl border px-3 py-2 text-sm"
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
      </div>
    </div>
  );
}
