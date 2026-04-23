import { useEffect, useMemo, useState, type ComponentType, type ReactNode } from "react";
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
    <div className="relative flex h-full w-full items-center justify-center overflow-hidden px-6 py-8 sm:px-8">
      <div className="pointer-events-none absolute inset-0 overflow-hidden">
        <div
          className="absolute inset-0"
          style={{
            backgroundImage: `
              linear-gradient(${withAlpha("var(--text-primary)", 4)} 1px, transparent 1px),
              linear-gradient(90deg, ${withAlpha("var(--text-primary)", 4)} 1px, transparent 1px)
            `,
            backgroundSize: "64px 64px",
            opacity: 0.12,
          }}
        />
        <div
          className="absolute left-1/2 top-[14%] h-[220px] w-[min(760px,78vw)] -translate-x-1/2 rounded-full blur-3xl"
          style={{
            background: `radial-gradient(circle, ${withAlpha("var(--accent-primary)", 14)} 0%, transparent 72%)`,
            opacity: 0.55,
          }}
        />
        <div
          className="absolute bottom-[-14%] left-1/2 h-[180px] w-[min(620px,72vw)] -translate-x-1/2 rounded-full blur-3xl"
          style={{
            background: `radial-gradient(circle, ${withAlpha("var(--text-primary)", 6)} 0%, transparent 72%)`,
            opacity: 0.3,
          }}
        />
      </div>

      <div className="relative z-10 flex w-full max-w-[920px] flex-col items-center">
        <div className="max-w-[620px] text-center">
          <div
            className="mb-3 inline-flex items-center gap-2 rounded-full border px-3 py-1 text-[10px] font-medium uppercase tracking-[0.16em]"
            style={{
              color: "var(--text-secondary)",
              background: withAlpha("var(--overlay-faint)", 92),
              borderColor: "var(--overlay-weak)",
            }}
          >
            <Sparkles className="h-3.5 w-3.5" style={{ color: "var(--accent-primary)" }} />
            Agent Workspace
          </div>
          <h2
            className="text-[clamp(1.9rem,3.4vw,2.9rem)] font-semibold tracking-[-0.05em] leading-[1.02]"
            style={{ color: "var(--text-primary)" }}
          >
            Start an agent conversation
          </h2>
          <p
            className="mx-auto mt-3 max-w-[520px] text-[13px] leading-relaxed"
            style={{ color: "var(--text-secondary)" }}
          >
            Choose the project and runtime, then send the first prompt directly from the
            canvas.
          </p>
        </div>

        <div
          className="relative mt-6 w-full max-w-[820px]"
          data-testid="agents-start-composer"
        >
          <div
            className="relative overflow-hidden rounded-[28px] border"
            style={{
              background:
                "linear-gradient(180deg, color-mix(in srgb, var(--bg-surface) 96%, transparent), color-mix(in srgb, var(--bg-surface) 92%, var(--bg-base) 8%))",
              borderColor: "var(--overlay-weak)",
              boxShadow: "var(--shadow-md)",
            }}
          >
            <div
              className="absolute inset-x-0 top-0 h-px"
              style={{
                background: `linear-gradient(90deg, transparent, ${withAlpha("var(--text-primary)", 14)}, transparent)`,
              }}
            />
            <textarea
              value={content}
              onChange={(event) => setContent(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter" && !event.shiftKey) {
                  event.preventDefault();
                  void handleSubmit();
                }
              }}
              placeholder="Ask the agent to plan, build, debug, or review something"
              className="block min-h-[124px] w-full resize-none border-0 bg-transparent px-5 pb-2.5 pt-[18px] text-[15px] leading-[1.45] shadow-none outline-none ring-0 focus:outline-none focus:ring-0 focus-visible:outline-none focus-visible:ring-0 sm:text-[16px] md:min-h-[136px]"
              style={{
                color: "var(--text-primary)",
                boxShadow: "none",
                outline: "none",
                WebkitAppearance: "none",
                appearance: "none",
              }}
              data-testid="agents-start-textarea"
              autoFocus
            />

            <div className="px-5 pb-2.5">
              <div
                className="flex flex-wrap items-center gap-x-2 gap-y-1 text-[10px] font-medium"
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
              className="border-t px-3.5 py-2.5"
              style={{
                borderColor: "var(--overlay-faint)",
                background:
                  "linear-gradient(180deg, color-mix(in srgb, var(--bg-base) 24%, transparent), color-mix(in srgb, var(--bg-base) 54%, transparent))",
              }}
            >
              <div className="flex items-center gap-1.5">
                <div className="flex min-w-0 flex-1 items-stretch gap-1.5">
                  <ComposerSelectPill
                    icon={FolderOpen}
                    label="Project"
                    value={projectId}
                    onValueChange={setProjectId}
                    disabled={projects.length === 0}
                    placeholder={projects.length === 0 ? "No projects yet" : "Select project"}
                    testId="agents-start-project"
                    className="min-w-[260px] flex-[1.4_1_312px]"
                    options={projects.map((project) => ({
                      id: project.id,
                      label: project.name,
                    }))}
                    endAction={
                      <Button
                        type="button"
                        variant="ghost"
                        className="h-7 shrink-0 rounded-[10px] px-2 text-[10px] font-medium"
                        style={{
                          color: "var(--text-secondary)",
                          background: withAlpha("var(--text-primary)", 6),
                        }}
                        onClick={onCreateProject}
                        data-testid="agents-start-new-project"
                      >
                        <Plus className="h-3.5 w-3.5" />
                        New
                      </Button>
                    }
                  />

                  <ComposerSelectPill
                    icon={Bot}
                    label="Provider"
                    value={provider}
                    onValueChange={(value) => handleProviderChange(value as AgentProvider)}
                    placeholder="Select provider"
                    testId="agents-start-provider"
                    className="min-w-[154px] flex-[0.72_1_170px]"
                    options={AGENT_PROVIDER_OPTIONS}
                  />

                  <ComposerSelectPill
                    icon={Cpu}
                    label="Model"
                    value={modelId}
                    onValueChange={setModelId}
                    placeholder="Select model"
                    testId="agents-start-model"
                    className="min-w-[176px] flex-[0.9_1_194px]"
                    options={modelOptions}
                  />
                </div>

                <Button
                  type="button"
                  className="h-10 shrink-0 rounded-[14px] px-4 text-[12px] font-semibold tracking-[-0.01em]"
                  style={{
                    minWidth: "122px",
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
  endAction?: ReactNode;
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
  endAction,
}: ComposerSelectPillProps) {
  return (
    <div
      className={cn(
        "flex min-h-10 items-center gap-2 rounded-[13px] border px-2.5 py-1.5",
        className
      )}
      style={{
        background: "var(--overlay-faint)",
        borderColor: "var(--overlay-weak)",
      }}
    >
      <div className="flex min-w-0 flex-1 items-center gap-2">
        <div
          className="flex h-[26px] w-[26px] shrink-0 items-center justify-center rounded-full"
          style={{
            background: withAlpha("var(--text-primary)", 8),
            color: "var(--text-secondary)",
          }}
        >
          <Icon className="h-[13px] w-[13px]" />
        </div>
        <div className="min-w-0 flex-1">
          <div
            className="mb-0.5 text-[8px] font-medium uppercase tracking-[0.16em]"
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
              className="h-auto border-0 bg-transparent px-0 py-0 text-[12px] font-medium shadow-none focus:ring-0"
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
      {endAction && (
        <>
          <div
            className="h-6 w-px shrink-0"
            style={{ background: "var(--overlay-weak)" }}
          />
          <div className="shrink-0">{endAction}</div>
        </>
      )}
    </div>
  );
}
