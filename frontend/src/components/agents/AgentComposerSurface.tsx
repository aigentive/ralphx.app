import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ComponentType,
  type ReactNode,
} from "react";
import {
  ArrowUp,
  Bot,
  Cpu,
  FolderOpen,
  Loader2,
  Plus,
  Square,
} from "lucide-react";

import type { AgentStatus } from "@/stores/chatStore";
import type { AgentProvider } from "@/stores/agentSessionStore";
import { Button } from "@/components/ui/button";
import { ChatAttachmentPicker } from "@/components/Chat/ChatAttachmentPicker";
import {
  ChatAttachmentGallery,
  type ChatAttachment as ComposerAttachment,
} from "@/components/Chat/ChatAttachmentGallery";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { withAlpha } from "@/lib/theme-colors";
import { cn } from "@/lib/utils";

interface ComposerOption {
  id: string;
  label: string;
}

interface ProjectFieldConfig {
  value: string;
  onValueChange: (value: string) => void;
  options: ComposerOption[];
  placeholder: string;
  disabled?: boolean;
  endAction?: ReactNode;
  testId?: string;
  className?: string;
}

interface ProviderFieldConfig {
  value: AgentProvider;
  onValueChange: (value: AgentProvider) => void;
  options: Array<{ id: AgentProvider; label: string }>;
  disabled?: boolean;
  testId?: string;
  className?: string;
}

interface ModelFieldConfig {
  value: string;
  onValueChange: (value: string) => void;
  options: ComposerOption[];
  disabled?: boolean;
  testId?: string;
  className?: string;
}

export interface AgentComposerQuestionMode {
  optionCount: number;
  multiSelect: boolean;
  onMatchedOptions: (indices: number[]) => void;
}

export interface AgentComposerSurfaceProps {
  project: ProjectFieldConfig;
  provider: ProviderFieldConfig;
  model: ModelFieldConfig;
  onSend: (message: string) => Promise<void> | void;
  onStop?: (() => Promise<unknown> | void) | undefined;
  placeholder?: string;
  isSubmitting?: boolean;
  agentStatus?: AgentStatus;
  value?: string;
  onChange?: (value: string) => void;
  isReadOnly?: boolean;
  autoFocus?: boolean;
  showHelperText?: boolean;
  questionMode?: AgentComposerQuestionMode;
  hasQueuedMessages?: boolean;
  onEditLastQueued?: (() => void) | undefined;
  attachments?: ComposerAttachment[];
  enableAttachments?: boolean;
  onFilesSelected?: ((files: File[]) => void | Promise<unknown>) | undefined;
  onRemoveAttachment?: ((id: string) => void | Promise<unknown>) | undefined;
  attachmentsUploading?: boolean;
  dataTestId?: string;
  textareaTestId?: string;
  actionTestId?: string;
  submitLabel?: string;
  submittingLabel?: string;
  className?: string;
}

export function AgentComposerSurface({
  project,
  provider,
  model,
  onSend,
  onStop,
  placeholder = "Ask the agent to plan, build, debug, or review something",
  isSubmitting = false,
  agentStatus = "idle",
  value: controlledValue,
  onChange: onChangeProp,
  isReadOnly = false,
  autoFocus = false,
  showHelperText = true,
  questionMode,
  hasQueuedMessages = false,
  onEditLastQueued,
  attachments = [],
  enableAttachments = false,
  onFilesSelected,
  onRemoveAttachment,
  attachmentsUploading = false,
  dataTestId,
  textareaTestId,
  actionTestId,
  submitLabel = "Send",
  submittingLabel = "Sending...",
  className,
}: AgentComposerSurfaceProps) {
  const isControlled = controlledValue !== undefined;
  const [internalValue, setInternalValue] = useState("");
  const [isFocused, setIsFocused] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const value = isControlled ? controlledValue : internalValue;
  const isAgentAlive = agentStatus !== "idle";
  const canQueue = !isReadOnly && isAgentAlive;
  const shouldShowStop = Boolean(onStop) && isAgentAlive && value.trim().length === 0;
  const canSubmit = value.trim().length > 0 && !isReadOnly && (!isSubmitting || canQueue);
  const effectivePlaceholder = isReadOnly
    ? "Viewing historical state (read-only)"
    : questionMode
      ? `Type 1-${questionMode.optionCount} or a custom response...`
      : placeholder;

  useEffect(() => {
    if (autoFocus && textareaRef.current) {
      textareaRef.current.focus();
    }
  }, [autoFocus]);

  useEffect(() => {
    const textarea = textareaRef.current;
    if (!textarea) {
      return;
    }

    textarea.style.height = "auto";
    const nextHeight = Math.min(textarea.scrollHeight, 220);
    textarea.style.height = `${Math.max(nextHeight, 116)}px`;
  }, [value]);

  const matchOptionsFromInput = useCallback(
    (input: string) => {
      if (!questionMode) {
        return;
      }

      const trimmed = input.trim();
      if (!trimmed) {
        questionMode.onMatchedOptions([]);
        return;
      }

      if (questionMode.multiSelect) {
        const parts = trimmed.split(",").map((segment) => segment.trim());
        const allNumeric = parts.every((part) => /^\d+$/.test(part));
        if (!allNumeric) {
          questionMode.onMatchedOptions([]);
          return;
        }
        const indices = parts
          .map((part) => parseInt(part, 10))
          .filter((index) => index >= 1 && index <= questionMode.optionCount)
          .map((index) => index - 1);
        questionMode.onMatchedOptions(indices);
        return;
      }

      if (!/^\d+$/.test(trimmed)) {
        questionMode.onMatchedOptions([]);
        return;
      }

      const optionNumber = parseInt(trimmed, 10);
      if (optionNumber >= 1 && optionNumber <= questionMode.optionCount) {
        questionMode.onMatchedOptions([optionNumber - 1]);
        return;
      }

      questionMode.onMatchedOptions([]);
    },
    [questionMode]
  );

  const setValue = useCallback(
    (nextValue: string) => {
      if (isControlled) {
        onChangeProp?.(nextValue);
      } else {
        setInternalValue(nextValue);
      }
      matchOptionsFromInput(nextValue);
    },
    [isControlled, matchOptionsFromInput, onChangeProp]
  );

  const clearValue = useCallback(() => {
    if (isControlled) {
      onChangeProp?.("");
    } else {
      setInternalValue("");
    }
    questionMode?.onMatchedOptions([]);
  }, [isControlled, onChangeProp, questionMode]);

  const handleSend = useCallback(async () => {
    const trimmedValue = value.trim();
    if (!trimmedValue) {
      if (shouldShowStop) {
        await onStop?.();
      }
      return;
    }

    if ((isSubmitting && !canQueue) || isReadOnly) {
      return;
    }

    if (questionMode || isControlled) {
      await onSend(trimmedValue);
      return;
    }

    clearValue();
    try {
      await onSend(trimmedValue);
    } catch {
      // Errors surface through the parent; preserve the current interaction model.
    }
  }, [
    canQueue,
    clearValue,
    isControlled,
    isReadOnly,
    isSubmitting,
    onSend,
    onStop,
    questionMode,
    shouldShowStop,
    value,
  ]);

  const handleKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if (event.key === "Escape") {
        event.preventDefault();
        (event.target as HTMLTextAreaElement).blur();
        return;
      }

      if (event.key === "Enter" && !event.shiftKey) {
        event.preventDefault();
        void handleSend();
        return;
      }

      if (event.key === "ArrowUp" && !value && hasQueuedMessages) {
        event.preventDefault();
        onEditLastQueued?.();
      }
    },
    [handleSend, hasQueuedMessages, onEditLastQueued, value]
  );

  const helperText = useMemo(() => {
    if (!showHelperText) {
      return null;
    }
    return (
      <div
        className="flex flex-wrap items-center gap-x-2 gap-y-1 text-[10px] font-medium"
        style={{ color: "var(--text-muted)" }}
      >
        <span>{shouldShowStop ? "Stop the active run" : "Press Enter to send"}</span>
        <span aria-hidden="true" style={{ color: "var(--overlay-moderate)" }}>
          •
        </span>
        <span>Shift + Enter for a new line</span>
      </div>
    );
  }, [shouldShowStop, showHelperText]);

  return (
    <div
      data-testid={dataTestId}
      className={cn("mx-auto w-full max-w-full", className)}
    >
      <div
        className="overflow-hidden rounded-[22px] border transition-colors"
        style={{
          background: "var(--bg-surface)",
          borderColor: isFocused ? "var(--accent-border)" : "var(--overlay-weak)",
          boxShadow: "var(--shadow-sm)",
        }}
      >
        <textarea
          ref={textareaRef}
          data-testid={textareaTestId}
          value={value}
          onChange={(event) => setValue(event.target.value)}
          onKeyDown={handleKeyDown}
          onFocus={() => setIsFocused(true)}
          onBlur={() => setIsFocused(false)}
          disabled={isReadOnly || (isSubmitting && !canQueue)}
          placeholder={effectivePlaceholder}
          className="block min-h-[116px] w-full resize-none border-0 bg-transparent px-5 pb-2 pt-4 text-[15px] leading-[1.5] shadow-none outline-none ring-0 focus:outline-none focus:ring-0 focus-visible:outline-none focus-visible:ring-0 sm:text-[16px]"
          style={{
            color: "var(--text-primary)",
            boxShadow: "none",
            outline: "none",
            WebkitAppearance: "none",
            appearance: "none",
          }}
          aria-label="Message input"
        />

        {(attachments.length > 0 || attachmentsUploading || helperText) && (
          <div className="px-5 pb-3">
            {attachments.length > 0 && (
              <div className="pb-3">
                <ChatAttachmentGallery
                  attachments={attachments}
                  {...(onRemoveAttachment ? { onRemove: onRemoveAttachment } : {})}
                  uploading={attachmentsUploading}
                  compact
                />
              </div>
            )}
            {helperText}
          </div>
        )}

        <div
          className="border-t px-3.5 py-2"
          style={{
            borderColor: "var(--overlay-faint)",
            background: "color-mix(in srgb, var(--bg-base) 16%, var(--bg-surface) 84%)",
          }}
        >
          <div className="flex flex-wrap items-center gap-2 md:flex-nowrap">
            {enableAttachments && (
              <div className="shrink-0">
                <ChatAttachmentPicker
                  {...(onFilesSelected ? { onFilesSelected } : {})}
                  disabled={isReadOnly || (isSubmitting && !canQueue)}
                />
              </div>
            )}

            <div className="flex min-w-0 flex-1 items-stretch gap-2">
              <ComposerSelectPill
                icon={FolderOpen}
                label="Project"
                value={project.value}
                onValueChange={project.onValueChange}
                placeholder={project.placeholder}
                options={project.options}
                {...(project.disabled !== undefined ? { disabled: project.disabled } : {})}
                {...(project.testId ? { testId: project.testId } : {})}
                className={project.className ?? "max-w-[260px] flex-none"}
                {...(project.endAction ? { endAction: project.endAction } : {})}
              />

              <ComposerDualSelectPill
                provider={provider}
                model={model}
                className="max-w-[340px] flex-none"
              />
            </div>

            <Button
              type="button"
              className="h-10 shrink-0 rounded-[12px] px-4 text-[12px] font-semibold tracking-[-0.01em]"
              style={{
                minWidth: shouldShowStop ? "100px" : "118px",
                background:
                  shouldShowStop || canSubmit
                    ? "var(--accent-primary)"
                    : withAlpha("var(--accent-primary)", 40),
                color: "var(--text-on-accent)",
                boxShadow: "none",
              }}
              onClick={() => {
                if (shouldShowStop) {
                  void onStop?.();
                  return;
                }
                void handleSend();
              }}
              disabled={shouldShowStop ? false : !canSubmit}
              data-testid={actionTestId}
              aria-label={shouldShowStop ? "Stop agent" : submitLabel}
            >
              {shouldShowStop ? (
                <>
                  <Square className="h-3.5 w-3.5 fill-current" />
                  Stop
                </>
              ) : isSubmitting && !canQueue ? (
                <>
                  <Loader2 className="h-4 w-4 animate-spin" />
                  {submittingLabel}
                </>
              ) : (
                <>
                  <ArrowUp className="h-4 w-4" />
                  {submitLabel}
                </>
              )}
            </Button>
          </div>
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
  options: ComposerOption[];
  placeholder: string;
  testId?: string;
  disabled?: boolean;
  className?: string;
  endAction?: ReactNode;
}

interface ComposerSelectFieldProps {
  icon: ComponentType<{ className?: string }>;
  label: string;
  value: string;
  onValueChange: (value: string) => void;
  options: ComposerOption[];
  placeholder: string;
  testId?: string;
  disabled?: boolean;
  onOpenChange?: (open: boolean) => void;
  fieldClassName?: string;
}

function ComposerSelectField({
  icon: Icon,
  label,
  value,
  onValueChange,
  options,
  placeholder,
  testId,
  disabled = false,
  onOpenChange,
  fieldClassName,
}: ComposerSelectFieldProps) {
  const triggerRef = useRef<HTMLButtonElement | null>(null);

  return (
    <div className={cn("flex min-w-0 items-center gap-2", fieldClassName)}>
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
        <Select
          {...(value ? { value } : {})}
          onValueChange={onValueChange}
          disabled={disabled}
          onOpenChange={(open) => {
            onOpenChange?.(open);
            if (!open) {
              requestAnimationFrame(() => {
                triggerRef.current?.blur();
              });
            }
          }}
        >
          <SelectTrigger
            ref={triggerRef}
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
  const [isOpen, setIsOpen] = useState(false);

  return (
    <div
      className={cn(
        "inline-flex min-h-10 max-w-full items-center gap-2 rounded-[12px] border px-2.5 py-1.5 transition-[border-color,box-shadow] focus-within:border-transparent focus-within:shadow-[0_0_0_1px_var(--accent-border)]",
        isOpen && "border-transparent shadow-[0_0_0_1px_var(--accent-border)]",
        className
      )}
      style={{
        background: "color-mix(in srgb, var(--bg-base) 24%, var(--bg-surface) 76%)",
        borderColor: "var(--overlay-weak)",
      }}
    >
      <ComposerSelectField
        icon={Icon}
        label={label}
        value={value}
        onValueChange={onValueChange}
        options={options}
        placeholder={placeholder}
        {...(testId ? { testId } : {})}
        {...(disabled !== undefined ? { disabled } : {})}
        onOpenChange={setIsOpen}
      />
      {endAction && (
        <>
          <div className="h-6 w-px shrink-0" style={{ background: "var(--overlay-weak)" }} />
          <div className="shrink-0">{endAction}</div>
        </>
      )}
    </div>
  );
}

function ComposerDualSelectPill({
  provider,
  model,
  className,
}: {
  provider: ProviderFieldConfig;
  model: ModelFieldConfig;
  className?: string;
}) {
  const [providerOpen, setProviderOpen] = useState(false);
  const [modelOpen, setModelOpen] = useState(false);

  return (
    <div
      className={cn(
        "inline-flex min-h-10 max-w-full items-center gap-3 rounded-[12px] border px-2.5 py-1.5 transition-[border-color,box-shadow] focus-within:border-transparent focus-within:shadow-[0_0_0_1px_var(--accent-border)]",
        (providerOpen || modelOpen) && "border-transparent shadow-[0_0_0_1px_var(--accent-border)]",
        className
      )}
      style={{
        background: "color-mix(in srgb, var(--bg-base) 24%, var(--bg-surface) 76%)",
        borderColor: "var(--overlay-weak)",
      }}
    >
      <ComposerSelectField
        icon={Bot}
        label="Provider"
        value={provider.value}
        onValueChange={(value) => provider.onValueChange(value as AgentProvider)}
        options={provider.options}
        placeholder="Select provider"
        {...(provider.disabled !== undefined ? { disabled: provider.disabled } : {})}
        {...(provider.testId ? { testId: provider.testId } : {})}
        {...(provider.className ? { fieldClassName: provider.className } : {})}
        onOpenChange={setProviderOpen}
      />

      <div className="h-6 w-px shrink-0" style={{ background: "var(--overlay-weak)" }} />

      <ComposerSelectField
        icon={Cpu}
        label="Model"
        value={model.value}
        onValueChange={model.onValueChange}
        options={model.options}
        placeholder="Select model"
        {...(model.disabled !== undefined ? { disabled: model.disabled } : {})}
        {...(model.testId ? { testId: model.testId } : {})}
        {...(model.className ? { fieldClassName: model.className } : {})}
        onOpenChange={setModelOpen}
      />
    </div>
  );
}

export function AgentComposerProjectCreateButton({
  onClick,
  testId,
}: {
  onClick: () => void;
  testId?: string;
}) {
  return (
    <Button
      type="button"
      variant="ghost"
      className="h-7 shrink-0 rounded-[10px] px-2 text-[10px] font-medium"
      style={{
        color: "var(--text-secondary)",
        background: "transparent",
      }}
      onClick={onClick}
      data-testid={testId}
    >
      <Plus className="h-3.5 w-3.5" />
      New
    </Button>
  );
}
