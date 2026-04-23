import { useEffect, useMemo, useState, type ComponentType } from "react";
import {
  ArrowUp,
  Bot,
  Cpu,
  FolderOpen,
  Loader2,
  Plus,
  Sparkles,
} from "lucide-react";

import type { Project } from "@/types/project";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import { withAlpha } from "@/lib/theme-colors";
import { cn } from "@/lib/utils";
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
    <div className="relative flex h-full w-full items-center justify-center overflow-hidden px-6 py-10 sm:px-10">
      <div className="pointer-events-none absolute inset-0 overflow-hidden">
        <div
          className="absolute inset-0"
          style={{
            backgroundImage: `
              linear-gradient(${withAlpha("var(--text-primary)", 4)} 1px, transparent 1px),
              linear-gradient(90deg, ${withAlpha("var(--text-primary)", 4)} 1px, transparent 1px)
            `,
            backgroundSize: "72px 72px",
            opacity: 0.18,
          }}
        />
        <div
          className="absolute left-1/2 top-[12%] h-[280px] w-[min(920px,88vw)] -translate-x-1/2 rounded-full blur-3xl"
          style={{
            background: `radial-gradient(circle, ${withAlpha("var(--accent-primary)", 20)} 0%, transparent 72%)`,
            opacity: 0.9,
          }}
        />
        <div
          className="absolute bottom-[-10%] left-1/2 h-[220px] w-[min(720px,80vw)] -translate-x-1/2 rounded-full blur-3xl"
          style={{
            background: `radial-gradient(circle, ${withAlpha("var(--text-primary)", 8)} 0%, transparent 72%)`,
            opacity: 0.55,
          }}
        />
      </div>

      <div className="relative z-10 flex w-full max-w-[1120px] flex-col items-center">
        <div className="max-w-[760px] text-center">
          <div
            className="mb-5 inline-flex items-center gap-2 rounded-full border px-4 py-2 text-[11px] font-medium uppercase tracking-[0.18em]"
            style={{
              color: "var(--text-secondary)",
              background: withAlpha("var(--bg-surface)", 84),
              borderColor: "var(--overlay-faint)",
              boxShadow: "var(--shadow-xs)",
            }}
          >
            <Sparkles className="h-3.5 w-3.5" style={{ color: "var(--accent-primary)" }} />
            Agent Workspace
          </div>
          <h2
            className="text-[clamp(2.5rem,5vw,4.5rem)] font-semibold tracking-[-0.06em] leading-[0.96]"
            style={{ color: "var(--text-primary)" }}
          >
            What should the agent tackle next?
          </h2>
          <p
            className="mx-auto mt-4 max-w-[620px] text-[15px] leading-relaxed"
            style={{ color: "var(--text-secondary)" }}
          >
            Project, provider, and model live right in the composer. Write the first prompt once,
            then launch the conversation directly.
          </p>
        </div>

        <div
          className="relative mt-10 w-full max-w-[1040px]"
          data-testid="agents-start-composer"
        >
          <div
            className="absolute inset-x-10 bottom-4 h-24 rounded-full blur-3xl"
            style={{
              background: `radial-gradient(circle, ${withAlpha("var(--accent-primary)", 18)} 0%, transparent 72%)`,
              opacity: 0.85,
            }}
          />
          <div
            className="relative overflow-hidden rounded-[34px] border"
            style={{
              background:
                "linear-gradient(180deg, color-mix(in srgb, var(--bg-surface) 96%, transparent), color-mix(in srgb, var(--bg-surface) 90%, var(--bg-base) 10%))",
              borderColor: "var(--overlay-faint)",
              boxShadow: "var(--shadow-lg)",
            }}
          >
            <div
              className="absolute inset-x-0 top-0 h-px"
              style={{
                background: `linear-gradient(90deg, transparent, ${withAlpha("var(--text-primary)", 18)}, transparent)`,
              }}
            />
            <Textarea
              value={content}
              onChange={(event) => setContent(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter" && !event.shiftKey) {
                  event.preventDefault();
                  void handleSubmit();
                }
              }}
              placeholder="Ask the agent to plan, build, debug, or review something"
              className="min-h-[220px] resize-none border-0 bg-transparent px-7 pb-5 pt-7 text-[18px] leading-[1.45] shadow-none focus-visible:ring-0 sm:text-[23px] md:min-h-[240px]"
              style={{
                color: "var(--text-primary)",
                boxShadow: "none",
              }}
              data-testid="agents-start-textarea"
              autoFocus
            />

            <div className="px-7 pb-5">
              <div
                className="flex flex-wrap items-center gap-x-2 gap-y-1 text-[11px] font-medium"
                style={{ color: "var(--text-muted)" }}
              >
                <span>Press Enter to start</span>
                <span aria-hidden="true" style={{ color: "var(--overlay-moderate)" }}>
                  •
                </span>
                <span>Shift + Enter for a new line</span>
              </div>
            </div>

            <div
              className="border-t px-4 py-4 sm:px-5"
              style={{
                borderColor: "var(--overlay-faint)",
                background:
                  "linear-gradient(180deg, color-mix(in srgb, var(--bg-base) 28%, transparent), color-mix(in srgb, var(--bg-base) 58%, transparent))",
              }}
            >
              <div className="flex flex-col gap-3 xl:flex-row xl:items-end xl:justify-between">
                <div className="flex flex-1 flex-wrap items-stretch gap-2">
                  <ComposerSelectPill
                    icon={FolderOpen}
                    label="Project"
                    value={projectId}
                    onValueChange={setProjectId}
                    disabled={projects.length === 0}
                    placeholder={projects.length === 0 ? "No projects yet" : "Select project"}
                    testId="agents-start-project"
                    className="min-w-[220px] flex-[1.3_1_280px]"
                    options={projects.map((project) => ({
                      id: project.id,
                      label: project.name,
                    }))}
                  />

                  <ComposerSelectPill
                    icon={Bot}
                    label="Provider"
                    value={provider}
                    onValueChange={(value) => handleProviderChange(value as AgentProvider)}
                    placeholder="Select provider"
                    testId="agents-start-provider"
                    className="min-w-[180px] flex-[0.75_1_190px]"
                    options={AGENT_PROVIDER_OPTIONS}
                  />

                  <ComposerSelectPill
                    icon={Cpu}
                    label="Model"
                    value={modelId}
                    onValueChange={setModelId}
                    placeholder="Select model"
                    testId="agents-start-model"
                    className="min-w-[220px] flex-[1_1_240px]"
                    options={modelOptions}
                  />

                  <Button
                    type="button"
                    variant="ghost"
                    className="h-[56px] shrink-0 rounded-[18px] border px-4 text-[14px] font-medium"
                    style={{
                      color: "var(--text-secondary)",
                      background: "var(--overlay-faint)",
                      borderColor: "var(--overlay-weak)",
                    }}
                    onClick={onCreateProject}
                    data-testid="agents-start-new-project"
                  >
                    <Plus className="h-4 w-4" />
                    New Project
                  </Button>
                </div>

                <div className="flex items-center justify-end">
                  <Button
                    type="button"
                    className="h-[56px] min-w-[156px] rounded-[20px] px-5 text-[14px] font-semibold tracking-[-0.01em]"
                    style={{
                      background: canSubmit
                        ? "var(--accent-primary)"
                        : withAlpha("var(--accent-primary)", 40),
                      color: "var(--text-on-accent)",
                      boxShadow: canSubmit ? "var(--shadow-md)" : "none",
                    }}
                    onClick={() => void handleSubmit()}
                    disabled={!canSubmit}
                    data-testid="agents-start-submit"
                  >
                    {isSubmitting ? (
                      <>
                        <Loader2 className="h-4 w-4 animate-spin" />
                        Starting...
                      </>
                    ) : (
                      <>
                        <ArrowUp className="h-4 w-4" />
                        Start Agent
                      </>
                    )}
                  </Button>
                </div>
              </div>
            </div>
          </div>

          {error && (
            <div
              className="mx-auto mt-4 inline-flex max-w-full items-center gap-2 rounded-full border px-4 py-2 text-[13px]"
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

interface ComposerSelectPillProps {
  icon: ComponentType<{ className?: string }>;
  label: string;
  value: string;
  onValueChange: (value: string) => void;
  options: Array<{ id: string; label: string }>;
  placeholder: string;
  testId: string;
  disabled?: boolean;
  className?: string;
}

function ComposerSelectPill({
  icon: Icon,
  label,
  value,
  onValueChange,
  options,
  placeholder,
  testId,
  disabled = false,
  className,
}: ComposerSelectPillProps) {
  return (
    <div
      className={cn(
        "flex min-h-[56px] items-center gap-3 rounded-[18px] border px-3.5 py-2.5",
        className
      )}
      style={{
        background: "var(--overlay-faint)",
        borderColor: "var(--overlay-weak)",
      }}
    >
      <div
        className="flex h-9 w-9 shrink-0 items-center justify-center rounded-full"
        style={{
          background: withAlpha("var(--text-primary)", 8),
          color: "var(--text-secondary)",
        }}
      >
        <Icon className="h-4 w-4" />
      </div>
      <div className="min-w-0 flex-1">
        <div
          className="mb-1 text-[10px] font-medium uppercase tracking-[0.16em]"
          style={{
            color: "var(--text-muted)",
          }}
        >
          {label}
        </div>
        <Select
          {...(value ? { value } : {})}
          onValueChange={onValueChange}
          disabled={disabled}
        >
          <SelectTrigger
            className="h-auto border-0 bg-transparent px-0 py-0 text-[15px] font-medium shadow-none focus:ring-0"
            style={{
              color: value ? "var(--text-primary)" : "var(--text-secondary)",
            }}
            data-testid={testId}
            aria-label={label}
          >
            <SelectValue placeholder={placeholder} />
          </SelectTrigger>
          <SelectContent>
            {options.map((option) => (
              <SelectItem key={option.id} value={option.id}>
                {option.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
    </div>
  );
}
