import { useEffect, useMemo, useState } from "react";
import { Sparkles } from "lucide-react";

import type { Project } from "@/types/project";
import { withAlpha } from "@/lib/theme-colors";
import type {
  AgentProvider,
  AgentRuntimeSelection,
} from "@/stores/agentSessionStore";
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
    files: File[];
  }) => Promise<void>;
}

const MAX_FILES = 5;
const MAX_FILE_SIZE = 10 * 1024 * 1024;

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
  const [attachments, setAttachments] = useState<PendingAttachment[]>([]);
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
          >
            Start your agent
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
