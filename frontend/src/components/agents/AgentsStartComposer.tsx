import { useEffect, useMemo, useState } from "react";
import { BrainCircuit, GitBranch, Sparkles, type LucideIcon } from "lucide-react";

import { getGitBranches, getGitCurrentBranch, getGitDefaultBranch } from "@/api/projects";
import type {
  AgentConversationBaseSelection,
  AgentConversationWorkspaceMode,
} from "@/api/chat";
import type { Project } from "@/types/project";
import { withAlpha } from "@/lib/theme-colors";
import { cn } from "@/lib/utils";
import type {
  AgentProvider,
  AgentRuntimeSelection,
} from "@/stores/agentSessionStore";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  AgentComposerProjectCreateButton,
  AgentComposerSurface,
  type AgentComposerSurfaceProps,
} from "./AgentComposerSurface";
import {
  AGENT_MODEL_OPTIONS,
  AGENT_PROVIDER_OPTIONS,
  DEFAULT_AGENT_RUNTIME,
  defaultModelForProvider,
  normalizeRuntimeSelection,
} from "./agentOptions";

interface PendingAttachment {
  id: string;
  file: File;
  fileName: string;
  fileSize: number;
  mimeType?: string;
}

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
    mode: AgentConversationWorkspaceMode;
    base: AgentConversationBaseSelection | null;
    files: File[];
  }) => Promise<void>;
}

const MAX_FILES = 5;
const MAX_FILE_SIZE = 10 * 1024 * 1024;
const STARTER_TYPING_WORDS = [
  "agent",
  "project",
  "plan",
  "idea",
  "build",
  "PR",
  "feature",
  "bugfix",
] as const;
const STARTER_TYPING_HOLD_MS = 1600;
const STARTER_TYPING_SPEED_MS = 72;
const STARTER_DELETING_SPEED_MS = 44;
const STARTER_TYPING_INITIAL_WORD = STARTER_TYPING_WORDS[0];

type StarterTypingPhase = "holding" | "typing" | "deleting";

interface StartFromOption {
  key: string;
  label: string;
  selection: AgentConversationBaseSelection;
}

const AGENT_MODE_OPTIONS: Array<{ id: AgentConversationWorkspaceMode; label: string }> = [
  { id: "edit", label: "Edit Agent" },
  { id: "ideation", label: "Ideation Mode" },
];

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
  const [mode, setMode] = useState<AgentConversationWorkspaceMode>("edit");
  const [startFromOptions, setStartFromOptions] = useState<StartFromOption[]>([]);
  const [selectedStartFromKey, setSelectedStartFromKey] = useState("");
  const [isLoadingStartFrom, setIsLoadingStartFrom] = useState(false);
  const [content, setContent] = useState("");
  const [attachments, setAttachments] = useState<PendingAttachment[]>([]);
  const [error, setError] = useState<string | null>(null);
  const animatedHeadingWord = useAnimatedStarterWord();

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
  const activeProject = useMemo(
    () => projects.find((project) => project.id === projectId) ?? null,
    [projectId, projects]
  );
  const selectedStartFrom =
    startFromOptions.find((option) => option.key === selectedStartFromKey) ?? null;
  const fallbackStartFrom = useMemo<AgentConversationBaseSelection | null>(() => {
    if (!activeProject) {
      return null;
    }
    const ref = activeProject.baseBranch ?? "main";
    return {
      kind: "project_default",
      ref,
      displayName: `Project default (${ref})`,
    };
  }, [activeProject]);

  const handleProviderChange = (nextProvider: AgentProvider) => {
    setProvider(nextProvider);
    setModelId(defaultModelForProvider(nextProvider));
  };

  const handleFilesSelected = (files: File[]) => {
    if (attachments.length + files.length > MAX_FILES) {
      setError(`Cannot upload more than ${MAX_FILES} files total`);
      return;
    }

    const oversizedFiles = files.filter((file) => file.size > MAX_FILE_SIZE);
    if (oversizedFiles.length > 0) {
      setError(
        `Files exceed 10MB limit: ${oversizedFiles.map((file) => file.name).join(", ")}`
      );
      return;
    }

    setError(null);
    setAttachments((current) => [
      ...current,
      ...files.map((file) => ({
        id:
          globalThis.crypto?.randomUUID?.() ??
          `${file.name}-${file.size}-${Date.now()}-${Math.random().toString(36).slice(2)}`,
        file,
        fileName: file.name,
        fileSize: file.size,
        ...(file.type ? { mimeType: file.type } : {}),
      })),
    ]);
  };

  useEffect(() => {
    if (!activeProject?.workingDirectory) {
      setStartFromOptions([]);
      setSelectedStartFromKey("");
      setIsLoadingStartFrom(false);
      return;
    }

    let cancelled = false;
    setIsLoadingStartFrom(true);

    async function loadStartFromOptions() {
      const workingDirectory = activeProject!.workingDirectory;
      const [defaultResult, currentResult, branchesResult] = await Promise.allSettled([
        getGitDefaultBranch(workingDirectory),
        getGitCurrentBranch(workingDirectory),
        getGitBranches(workingDirectory),
      ]);

      if (cancelled) {
        return;
      }

      const projectDefault =
        defaultResult.status === "fulfilled" && defaultResult.value
          ? defaultResult.value
          : activeProject!.baseBranch ?? "main";
      const currentBranch =
        currentResult.status === "fulfilled" && currentResult.value
          ? currentResult.value
          : projectDefault;
      const branches =
        branchesResult.status === "fulfilled" && Array.isArray(branchesResult.value)
          ? branchesResult.value
          : [projectDefault];
      const optionMap = new Map<string, StartFromOption>();

      optionMap.set(`project_default:${projectDefault}`, {
        key: `project_default:${projectDefault}`,
        label: `Project default (${projectDefault})`,
        selection: {
          kind: "project_default",
          ref: projectDefault,
          displayName: `Project default (${projectDefault})`,
        },
      });

      if (currentBranch && currentBranch !== projectDefault) {
        optionMap.set(`current_branch:${currentBranch}`, {
          key: `current_branch:${currentBranch}`,
          label: `Current branch (${currentBranch})`,
          selection: {
            kind: "current_branch",
            ref: currentBranch,
            displayName: `Current branch (${currentBranch})`,
          },
        });
      }

      branches
        .filter((branch) => branch && branch !== projectDefault && branch !== currentBranch)
        .forEach((branch) => {
          optionMap.set(`local_branch:${branch}`, {
            key: `local_branch:${branch}`,
            label: branch,
            selection: {
              kind: "local_branch",
              ref: branch,
              displayName: branch,
            },
          });
        });

      const options = Array.from(optionMap.values());
      setStartFromOptions(options);
      setSelectedStartFromKey(
        currentBranch && currentBranch !== projectDefault
          ? `current_branch:${currentBranch}`
          : `project_default:${projectDefault}`
      );
      setIsLoadingStartFrom(false);
    }

    void loadStartFromOptions().catch(() => {
      if (cancelled) {
        return;
      }
      const fallback = activeProject.baseBranch ?? "main";
      setStartFromOptions([
        {
          key: `project_default:${fallback}`,
          label: `Project default (${fallback})`,
          selection: {
            kind: "project_default",
            ref: fallback,
            displayName: `Project default (${fallback})`,
          },
        },
      ]);
      setSelectedStartFromKey(`project_default:${fallback}`);
      setIsLoadingStartFrom(false);
    });

    return () => {
      cancelled = true;
    };
  }, [activeProject]);

  const handleRemoveAttachment = (attachmentId: string) => {
    setAttachments((current) => current.filter((attachment) => attachment.id !== attachmentId));
  };

  const handleSubmit: AgentComposerSurfaceProps["onSend"] = async (message) => {
    if (!projectId) {
      setError("Project is required");
      return;
    }
    if (!message.trim()) {
      setError("Prompt is required");
      return;
    }

    setError(null);
    try {
      await onSubmit({
        projectId,
        content: message.trim(),
        runtime: { provider, modelId },
        mode,
        base: selectedStartFrom?.selection ?? fallbackStartFrom,
        files: attachments.map((attachment) => attachment.file),
      });
      setContent("");
      setAttachments([]);
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
            opacity: 0.07,
          }}
        />
        <div
          className="absolute left-1/2 top-[17%] h-[180px] w-[min(620px,72vw)] -translate-x-1/2 rounded-full blur-3xl"
          style={{
            background: `radial-gradient(circle, ${withAlpha("var(--accent-primary)", 8)} 0%, transparent 72%)`,
            opacity: 0.28,
          }}
        />
      </div>

      <div className="relative z-10 flex w-full max-w-[980px] flex-col items-center">
        <div className="max-w-[620px] text-center">
          <div
            className="mb-3 inline-flex items-center gap-2 rounded-full border px-3 py-1 text-[10px] font-medium uppercase tracking-[0.16em]"
            style={{
              color: "var(--text-secondary)",
              background: "var(--bg-surface)",
              borderColor: "var(--overlay-weak)",
            }}
          >
            <Sparkles className="h-3.5 w-3.5" style={{ color: "var(--accent-primary)" }} />
            Agent Workspace
          </div>
          <h2
            className="text-[clamp(1.9rem,3.4vw,2.9rem)] font-semibold tracking-[-0.05em] leading-[1.02]"
            style={{ color: "var(--text-primary)" }}
            data-testid="agents-start-heading"
          >
            <span className="inline-flex items-baseline justify-center whitespace-nowrap">
              <span>Start your&nbsp;</span>
              <span className="inline-flex items-baseline whitespace-nowrap">
                <span
                  data-testid="agents-start-heading-word"
                  style={{ color: "var(--accent-primary)" }}
                >
                  {animatedHeadingWord}
                </span>
                <span
                  aria-hidden="true"
                  className="animate-starter-caret ml-0.5 inline-block h-[0.9em] w-[2px] rounded-full align-middle"
                  style={{ background: "var(--accent-primary)" }}
                />
              </span>
            </span>
          </h2>
          <p
            className="mx-auto mt-3 max-w-[520px] text-[13px] leading-relaxed"
            style={{ color: "var(--text-secondary)" }}
          >
            Choose the project and runtime, then ask your agent for something amazing.
          </p>
        </div>

        <div className="mt-6 w-full">
          <AgentComposerSurface
            dataTestId="agents-start-composer"
            textareaTestId="agents-start-textarea"
            actionTestId="agents-start-submit"
            value={content}
            onChange={setContent}
            onSend={handleSubmit}
            placeholder="Ask the agent to plan, build, debug, or review something"
            isSubmitting={isSubmitting}
            autoFocus
            attachments={attachments}
            enableAttachments
            onFilesSelected={handleFilesSelected}
            onRemoveAttachment={handleRemoveAttachment}
            attachmentsUploading={isSubmitting && attachments.length > 0}
            submitLabel="Start Agent"
            submittingLabel="Starting..."
            workspaceControls={
              <>
                <StarterSelectPill
                  icon={GitBranch}
                  label="Start from"
                  value={selectedStartFromKey}
                  onValueChange={setSelectedStartFromKey}
                  options={startFromOptions}
                  placeholder={isLoadingStartFrom ? "Loading..." : "Base branch"}
                  disabled={isLoadingStartFrom || startFromOptions.length === 0}
                  testId="agents-start-base"
                  className="max-w-[240px] flex-none"
                />
                <StarterSelectPill
                  icon={BrainCircuit}
                  label="Mode"
                  value={mode}
                  onValueChange={(value) => setMode(value as AgentConversationWorkspaceMode)}
                  options={AGENT_MODE_OPTIONS}
                  placeholder="Mode"
                  testId="agents-start-mode"
                  className="max-w-[178px] flex-none"
                />
              </>
            }
            project={{
              value: projectId,
              onValueChange: setProjectId,
              options: projects.map((project) => ({
                id: project.id,
                label: project.name,
              })),
              placeholder: projects.length === 0 ? "No projects yet" : "Select project",
              disabled: isLoadingProjects || projects.length === 0,
              testId: "agents-start-project",
              className: "max-w-[300px] flex-none",
              endAction: (
                <AgentComposerProjectCreateButton
                  onClick={onCreateProject}
                  testId="agents-start-new-project"
                />
              ),
            }}
            provider={{
              value: provider,
              onValueChange: handleProviderChange,
              options: AGENT_PROVIDER_OPTIONS,
              testId: "agents-start-provider",
              className: "max-w-[172px] flex-none",
            }}
            model={{
              value: modelId,
              onValueChange: setModelId,
              options: modelOptions,
              testId: "agents-start-model",
              className: "max-w-[188px] flex-none",
            }}
          />

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

interface StarterSelectPillProps {
  icon: LucideIcon;
  label: string;
  value: string;
  onValueChange: (value: string) => void;
  options: Array<{ key?: string; id?: string; label: string }>;
  placeholder: string;
  disabled?: boolean;
  testId?: string;
  className?: string;
}

function StarterSelectPill({
  icon: Icon,
  label,
  value,
  onValueChange,
  options,
  placeholder,
  disabled = false,
  testId,
  className,
}: StarterSelectPillProps) {
  return (
    <div
      className={cn(
        "inline-flex min-h-10 max-w-full items-center gap-2 rounded-[12px] border px-2.5 py-1.5",
        className
      )}
      style={{
        background: "color-mix(in srgb, var(--bg-base) 24%, var(--bg-surface) 76%)",
        borderColor: "var(--overlay-weak)",
      }}
    >
      <div
        className="flex h-[24px] w-[24px] shrink-0 items-center justify-center rounded-full"
        style={{ color: "var(--text-secondary)" }}
      >
        <Icon className="h-[13px] w-[13px]" />
      </div>
      <div className="min-w-0">
        <div
          className="mb-0.5 text-[8px] font-medium uppercase tracking-[0.16em]"
          style={{ color: "var(--text-muted)" }}
        >
          {label}
        </div>
        <Select value={value} onValueChange={onValueChange} disabled={disabled}>
          <SelectTrigger
            className="h-auto w-auto min-w-0 border-0 bg-transparent px-0 py-0 text-[12px] font-medium shadow-none outline-none ring-0 focus:outline-none focus:ring-0 focus-visible:outline-none focus-visible:ring-0 [&>span]:max-w-full"
            style={{
              color: value ? "var(--text-primary)" : "var(--text-secondary)",
              boxShadow: "none",
              outline: "none",
              WebkitAppearance: "none",
              appearance: "none",
            }}
            data-testid={testId}
            data-theme-button-skip="true"
            aria-label={label}
          >
            <SelectValue placeholder={placeholder} />
          </SelectTrigger>
          <SelectContent>
            {options.map((option) => {
              const optionValue = option.key ?? option.id ?? option.label;
              return (
                <SelectItem key={optionValue} value={optionValue}>
                  {option.label}
                </SelectItem>
              );
            })}
          </SelectContent>
        </Select>
      </div>
    </div>
  );
}

function useAnimatedStarterWord() {
  const [wordIndex, setWordIndex] = useState(0);
  const [characterCount, setCharacterCount] = useState(
    STARTER_TYPING_INITIAL_WORD.length
  );
  const [phase, setPhase] = useState<StarterTypingPhase>("holding");
  const [prefersReducedMotion, setPrefersReducedMotion] = useState(false);

  useEffect(() => {
    if (typeof window === "undefined" || typeof window.matchMedia !== "function") {
      return;
    }

    const mediaQuery = window.matchMedia("(prefers-reduced-motion: reduce)");
    const handleChange = () => {
      setPrefersReducedMotion(mediaQuery.matches);
    };

    handleChange();

    if (typeof mediaQuery.addEventListener === "function") {
      mediaQuery.addEventListener("change", handleChange);
      return () => mediaQuery.removeEventListener("change", handleChange);
    }

    mediaQuery.addListener(handleChange);
    return () => mediaQuery.removeListener(handleChange);
  }, []);

  useEffect(() => {
    if (prefersReducedMotion) {
      return;
    }

    const currentWord = STARTER_TYPING_WORDS[wordIndex] ?? STARTER_TYPING_INITIAL_WORD;
    const timeoutMs =
      phase === "holding"
        ? STARTER_TYPING_HOLD_MS
        : phase === "typing"
          ? STARTER_TYPING_SPEED_MS
          : STARTER_DELETING_SPEED_MS;

    const timeout = window.setTimeout(() => {
      if (phase === "holding") {
        setPhase("deleting");
        return;
      }

      if (phase === "deleting") {
        if (characterCount > 0) {
          setCharacterCount((current) => current - 1);
          return;
        }

        setWordIndex((current) => (current + 1) % STARTER_TYPING_WORDS.length);
        setPhase("typing");
        return;
      }

      if (characterCount < currentWord.length) {
        setCharacterCount((current) => current + 1);
        return;
      }

      setPhase("holding");
    }, timeoutMs);

    return () => {
      window.clearTimeout(timeout);
    };
  }, [characterCount, phase, prefersReducedMotion, wordIndex]);

  useEffect(() => {
    if (prefersReducedMotion) {
      setWordIndex(0);
      setCharacterCount(STARTER_TYPING_INITIAL_WORD.length);
      setPhase("holding");
      return;
    }

    if (phase === "typing" && characterCount === 0) {
      return;
    }

    const currentWord = STARTER_TYPING_WORDS[wordIndex] ?? STARTER_TYPING_INITIAL_WORD;
    if (phase === "typing" && characterCount > currentWord.length) {
      setCharacterCount(currentWord.length);
    }
  }, [characterCount, phase, prefersReducedMotion, wordIndex]);

  if (prefersReducedMotion) {
    return STARTER_TYPING_INITIAL_WORD;
  }

  return (STARTER_TYPING_WORDS[wordIndex] ?? STARTER_TYPING_INITIAL_WORD).slice(
    0,
    characterCount
  );
}
