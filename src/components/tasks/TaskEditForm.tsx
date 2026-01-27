/**
 * TaskEditForm - Form for editing existing tasks
 *
 * Features:
 * - Edit title, category, description, and priority
 * - Pre-populated with existing task data
 * - Form validation with Zod schema
 * - onSave callback for parent to handle mutation
 *
 * Design spec: specs/design/refined-studio-patterns.md
 * - Refined Studio aesthetic with consistent form controls
 * - Glass effect styling on inputs
 */

import { useState, useCallback, useId, type FormEvent } from "react";
import { TASK_CATEGORIES, UpdateTaskSchema, type Task, type UpdateTask } from "@/types/task";
import { ACTIVE_STATUSES } from "@/types/status";
import { Loader2, Plus, AlertCircle } from "lucide-react";
import { StepList } from "./StepList";
import { useStepMutations } from "@/hooks/useStepMutations";

// ============================================================================
// Types
// ============================================================================

export interface TaskEditFormProps {
  /** Task to edit */
  task: Task;
  /** Callback when save is triggered (parent handles mutation) */
  onSave: (data: UpdateTask) => void;
  /** Callback when form is cancelled */
  onCancel: () => void;
  /** Whether the save operation is in progress */
  isSaving: boolean;
}

// ============================================================================
// Shared Styles
// ============================================================================

const inputBaseStyles = `
  w-full h-10 px-3 rounded-lg text-[13px]
  bg-white/[0.03] border border-white/[0.08]
  text-white/90 placeholder:text-white/30
  transition-all duration-150
  focus:outline-none focus:border-[#ff6b35]/50 focus:bg-white/[0.05]
  focus:shadow-[0_0_0_3px_rgba(255,107,53,0.1)]
  disabled:opacity-50 disabled:cursor-not-allowed
`.replace(/\n/g, ' ').trim();

const selectBaseStyles = `
  w-full h-10 px-3 rounded-lg text-[13px]
  bg-white/[0.03] border border-white/[0.08]
  text-white/90 cursor-pointer
  transition-all duration-150
  focus:outline-none focus:border-[#ff6b35]/50 focus:bg-white/[0.05]
  focus:shadow-[0_0_0_3px_rgba(255,107,53,0.1)]
  disabled:opacity-50 disabled:cursor-not-allowed
  appearance-none
  bg-[url('data:image/svg+xml;charset=utf-8,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20width%3D%2216%22%20height%3D%2216%22%20viewBox%3D%220%200%2024%2024%22%20fill%3D%22none%22%20stroke%3D%22rgba(255%2C255%2C255%2C0.4)%22%20stroke-width%3D%222%22%3E%3Cpath%20d%3D%22M6%209l6%206%206-6%22%2F%3E%3C%2Fsvg%3E')]
  bg-[length:16px_16px] bg-[right_12px_center] bg-no-repeat
  pr-10
`.replace(/\n/g, ' ').trim();

const textareaBaseStyles = `
  w-full px-3 py-2.5 rounded-lg text-[13px] leading-relaxed
  bg-white/[0.03] border border-white/[0.08]
  text-white/90 placeholder:text-white/30
  transition-all duration-150 resize-none
  focus:outline-none focus:border-[#ff6b35]/50 focus:bg-white/[0.05]
  focus:shadow-[0_0_0_3px_rgba(255,107,53,0.1)]
  disabled:opacity-50 disabled:cursor-not-allowed
`.replace(/\n/g, ' ').trim();

const labelStyles = "block text-[12px] font-medium text-white/50 uppercase tracking-wide mb-2";

const buttonPrimaryStyles = `
  h-10 px-4 rounded-lg text-[13px] font-medium
  bg-[#ff6b35] text-white
  transition-all duration-150
  hover:bg-[#ff8050] hover:shadow-[0_4px_12px_rgba(255,107,53,0.3)]
  focus:outline-none focus:shadow-[0_0_0_3px_rgba(255,107,53,0.3)]
  disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:shadow-none
  flex items-center justify-center gap-2
`.replace(/\n/g, ' ').trim();

const buttonSecondaryStyles = `
  h-10 px-4 rounded-lg text-[13px] font-medium
  bg-transparent border border-white/[0.1] text-white/70
  transition-all duration-150
  hover:bg-white/[0.05] hover:border-white/[0.15] hover:text-white/90
  focus:outline-none focus:shadow-[0_0_0_3px_rgba(255,255,255,0.05)]
  disabled:opacity-50 disabled:cursor-not-allowed
`.replace(/\n/g, ' ').trim();

// ============================================================================
// Component
// ============================================================================

export function TaskEditForm({
  task,
  onSave,
  onCancel,
  isSaving,
}: TaskEditFormProps) {
  const baseId = useId();

  // Form state - pre-populate with task data
  const [title, setTitle] = useState(task.title);
  const [category, setCategory] = useState(task.category);
  const [description, setDescription] = useState(task.description || "");
  const [priority, setPriority] = useState(task.priority);
  const [validationError, setValidationError] = useState<string | null>(null);

  // Step editor state
  const [newStepTitle, setNewStepTitle] = useState("");
  const [isAddingStep, setIsAddingStep] = useState(false);
  const { create: createStep } = useStepMutations(task.id);

  // Check if task is executing (steps are editable only when not executing)
  const isExecuting = ACTIVE_STATUSES.includes(task.internalStatus);

  const handleSubmit = useCallback(
    (e: FormEvent) => {
      e.preventDefault();
      setValidationError(null);

      // Build update data (only include changed fields)
      const updateData: UpdateTask = {};

      if (title.trim() !== task.title) {
        updateData.title = title.trim();
      }

      if (category !== task.category) {
        updateData.category = category;
      }

      const descValue = description.trim() || null;
      if (descValue !== task.description) {
        updateData.description = descValue;
      }

      if (priority !== task.priority) {
        updateData.priority = priority;
      }

      // Validate with Zod schema
      const result = UpdateTaskSchema.safeParse(updateData);
      if (!result.success) {
        setValidationError(result.error.issues[0]?.message || "Validation failed");
        return;
      }

      // If no fields changed, just cancel
      if (Object.keys(updateData).length === 0) {
        onCancel();
        return;
      }

      onSave(updateData);
    },
    [title, category, description, priority, task, onSave, onCancel]
  );

  const hasChanges =
    title.trim() !== task.title ||
    category !== task.category ||
    (description.trim() || null) !== task.description ||
    priority !== task.priority;

  const handleAddStep = useCallback(async () => {
    if (!newStepTitle.trim()) return;

    setIsAddingStep(true);
    try {
      await createStep.mutateAsync({
        title: newStepTitle.trim(),
      });
      setNewStepTitle("");
    } finally {
      setIsAddingStep(false);
    }
  }, [newStepTitle, createStep]);

  return (
    <form onSubmit={handleSubmit} className="flex flex-col flex-1">
      {/* Form Fields */}
      <div className="space-y-5">
        {/* Title Field */}
        <div>
          <label htmlFor={`${baseId}-title`} className={labelStyles}>
            Title
          </label>
          <input
            type="text"
            id={`${baseId}-title`}
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            disabled={isSaving}
            placeholder="Enter task title"
            className={inputBaseStyles}
          />
        </div>

        {/* Category & Priority Row */}
      <div className="grid grid-cols-2 gap-4">
        {/* Category Field */}
        <div>
          <label htmlFor={`${baseId}-category`} className={labelStyles}>
            Category
          </label>
          <select
            id={`${baseId}-category`}
            value={category}
            onChange={(e) => setCategory(e.target.value)}
            disabled={isSaving}
            className={selectBaseStyles}
          >
            {TASK_CATEGORIES.map((cat) => (
              <option key={cat} value={cat} className="bg-[#1a1a1a] text-white">
                {cat.charAt(0).toUpperCase() + cat.slice(1)}
              </option>
            ))}
          </select>
        </div>

        {/* Priority Field */}
        <div>
          <label htmlFor={`${baseId}-priority`} className={labelStyles}>
            Priority
          </label>
          <select
            id={`${baseId}-priority`}
            value={priority}
            onChange={(e) => setPriority(Number(e.target.value))}
            disabled={isSaving}
            className={selectBaseStyles}
          >
            <option value={1} className="bg-[#1a1a1a] text-white">P1 - Critical</option>
            <option value={2} className="bg-[#1a1a1a] text-white">P2 - High</option>
            <option value={3} className="bg-[#1a1a1a] text-white">P3 - Medium</option>
            <option value={4} className="bg-[#1a1a1a] text-white">P4 - Low</option>
          </select>
        </div>
      </div>

      {/* Description Field */}
      <div>
        <label htmlFor={`${baseId}-description`} className={labelStyles}>
          Description
        </label>
        <textarea
          id={`${baseId}-description`}
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          disabled={isSaving}
          rows={4}
          placeholder="Enter task description (optional)"
          className={textareaBaseStyles}
        />
      </div>

      {/* Error Display */}
      {validationError && (
        <div
          className="flex items-center gap-2.5 px-3.5 py-3 rounded-lg text-[13px]"
          style={{
            background: "linear-gradient(135deg, rgba(239,68,68,0.12) 0%, rgba(239,68,68,0.05) 100%)",
            border: "1px solid rgba(239,68,68,0.25)",
          }}
        >
          <AlertCircle className="w-4 h-4 text-red-400 shrink-0" />
          <span className="text-red-300">{validationError}</span>
        </div>
      )}

      {/* Steps Section */}
      <div
        className="rounded-lg p-4"
        style={{
          background: "linear-gradient(180deg, rgba(255,255,255,0.02) 0%, rgba(255,255,255,0.01) 100%)",
          border: "1px solid rgba(255,255,255,0.06)",
        }}
      >
        <div className="flex items-center justify-between mb-3">
          <label className={labelStyles + " mb-0"}>
            Steps
          </label>
          {isExecuting && (
            <span className="text-[11px] text-white/40 italic">
              Cannot edit while executing
            </span>
          )}
        </div>

        {/* Step List */}
        <div className="mb-3">
          <StepList taskId={task.id} editable={!isExecuting && !isSaving} />
        </div>

        {/* Add Step Input */}
        {!isExecuting && (
          <div className="flex gap-2">
            <input
              type="text"
              value={newStepTitle}
              onChange={(e) => setNewStepTitle(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && !e.shiftKey) {
                  e.preventDefault();
                  handleAddStep();
                }
              }}
              disabled={isSaving || isAddingStep}
              placeholder="Add a new step..."
              className={inputBaseStyles}
            />
            <button
              type="button"
              onClick={handleAddStep}
              disabled={isSaving || isAddingStep || !newStepTitle.trim()}
              className={buttonPrimaryStyles + " shrink-0"}
            >
              {isAddingStep ? (
                <Loader2 className="w-4 h-4 animate-spin" />
              ) : (
                <Plus className="w-4 h-4" />
              )}
              <span className="hidden sm:inline">{isAddingStep ? "Adding..." : "Add"}</span>
            </button>
          </div>
        )}
        </div>
      </div>

      {/* Form Actions - Bottom aligned */}
      <div
        className="flex justify-end gap-3 pt-4 mt-auto border-t"
        style={{ borderColor: "rgba(255,255,255,0.06)" }}
      >
        <button
          type="button"
          onClick={onCancel}
          disabled={isSaving}
          className={buttonSecondaryStyles}
        >
          Cancel
        </button>
        <button
          type="submit"
          disabled={isSaving || !title.trim() || !hasChanges}
          className={buttonPrimaryStyles}
        >
          {isSaving && <Loader2 className="w-4 h-4 animate-spin" />}
          {isSaving ? "Saving..." : "Save Changes"}
        </button>
      </div>
    </form>
  );
}
