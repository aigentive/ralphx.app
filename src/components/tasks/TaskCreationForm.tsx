/**
 * TaskCreationForm - Form for creating new tasks with QA toggle
 *
 * Features:
 * - Title, category, and description fields
 * - QA toggle checkbox for enabling QA on this task
 * - Submits via useTaskMutation hook
 */

import { useState, useCallback, useId, type FormEvent } from "react";
import { useTaskMutation } from "@/hooks/useTaskMutation";
import { TASK_CATEGORIES, type CreateTask } from "@/types/task";
import { Plus, X, ChevronUp, ChevronDown } from "lucide-react";

// ============================================================================
// Types
// ============================================================================

export interface TaskCreationFormProps {
  /** Project ID to create the task in */
  projectId: string;
  /** Callback when task is created successfully */
  onSuccess?: () => void;
  /** Callback when form is cancelled */
  onCancel?: () => void;
}

// ============================================================================
// Component
// ============================================================================

export function TaskCreationForm({
  projectId,
  onSuccess,
  onCancel,
}: TaskCreationFormProps) {
  const baseId = useId();
  const { createMutation } = useTaskMutation(projectId);

  // Form state
  const [title, setTitle] = useState("");
  const [category, setCategory] = useState("feature");
  const [description, setDescription] = useState("");
  const [needsQa, setNeedsQa] = useState(false);
  const [steps, setSteps] = useState<string[]>([]);

  const handleSubmit = useCallback(
    (e: FormEvent) => {
      e.preventDefault();

      if (!title.trim()) {
        return;
      }

      const taskData: CreateTask = {
        projectId,
        title: title.trim(),
        category,
        priority: 0,
        ...(description.trim() && { description: description.trim() }),
        ...(needsQa && { needsQa: true }),
        ...(steps.length > 0 && { steps: steps.filter((s) => s.trim()) }),
      };

      createMutation.mutate(taskData, {
        onSuccess: () => {
          // Reset form
          setTitle("");
          setCategory("feature");
          setDescription("");
          setNeedsQa(false);
          setSteps([]);
          onSuccess?.();
        },
      });
    },
    [
      title,
      category,
      description,
      needsQa,
      steps,
      projectId,
      createMutation,
      onSuccess,
    ]
  );

  const handleAddStep = useCallback(() => {
    setSteps([...steps, ""]);
  }, [steps]);

  const handleRemoveStep = useCallback(
    (index: number) => {
      setSteps(steps.filter((_, i) => i !== index));
    },
    [steps]
  );

  const handleUpdateStep = useCallback(
    (index: number, value: string) => {
      const newSteps = [...steps];
      newSteps[index] = value;
      setSteps(newSteps);
    },
    [steps]
  );

  const handleMoveStepUp = useCallback(
    (index: number) => {
      if (index === 0) return;
      const newSteps = [...steps];
      const temp = newSteps[index]!;
      newSteps[index] = newSteps[index - 1]!;
      newSteps[index - 1] = temp;
      setSteps(newSteps);
    },
    [steps]
  );

  const handleMoveStepDown = useCallback(
    (index: number) => {
      if (index === steps.length - 1) return;
      const newSteps = [...steps];
      const temp = newSteps[index]!;
      newSteps[index] = newSteps[index + 1]!;
      newSteps[index + 1] = temp;
      setSteps(newSteps);
    },
    [steps]
  );

  const isSubmitting = createMutation.isPending;

  return (
    <form onSubmit={handleSubmit} className="space-y-4">
      <h2 className="text-lg font-medium text-[--text-primary]">Create Task</h2>

      {/* Title Field */}
      <div>
        <label
          htmlFor={`${baseId}-title`}
          className="block text-sm font-medium text-[--text-primary]"
        >
          Title
        </label>
        <input
          type="text"
          id={`${baseId}-title`}
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          disabled={isSubmitting}
          placeholder="Enter task title"
          className="mt-1 block w-full rounded-md px-3 py-2 text-sm bg-[--bg-elevated] border border-[--border-subtle] text-[--text-primary] placeholder-[--text-muted] focus:outline-none focus:ring-2 focus:ring-[--accent-primary] focus:border-transparent disabled:opacity-50 disabled:cursor-not-allowed"
        />
      </div>

      {/* Category Field */}
      <div>
        <label
          htmlFor={`${baseId}-category`}
          className="block text-sm font-medium text-[--text-primary]"
        >
          Category
        </label>
        <select
          id={`${baseId}-category`}
          value={category}
          onChange={(e) => setCategory(e.target.value)}
          disabled={isSubmitting}
          className="mt-1 block w-full rounded-md px-3 py-2 text-sm bg-[--bg-elevated] border border-[--border-subtle] text-[--text-primary] focus:outline-none focus:ring-2 focus:ring-[--accent-primary] focus:border-transparent disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {TASK_CATEGORIES.map((cat) => (
            <option key={cat} value={cat}>
              {cat.charAt(0).toUpperCase() + cat.slice(1)}
            </option>
          ))}
        </select>
      </div>

      {/* Description Field */}
      <div>
        <label
          htmlFor={`${baseId}-description`}
          className="block text-sm font-medium text-[--text-primary]"
        >
          Description
        </label>
        <textarea
          id={`${baseId}-description`}
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          disabled={isSubmitting}
          rows={3}
          placeholder="Enter task description (optional)"
          className="mt-1 block w-full rounded-md px-3 py-2 text-sm bg-[--bg-elevated] border border-[--border-subtle] text-[--text-primary] placeholder-[--text-muted] focus:outline-none focus:ring-2 focus:ring-[--accent-primary] focus:border-transparent disabled:opacity-50 disabled:cursor-not-allowed resize-none"
        />
      </div>

      {/* Steps Section */}
      <div>
        <label className="block text-sm font-medium text-[--text-primary] mb-2">
          Steps
        </label>
        <div className="space-y-2">
          {steps.map((step, index) => (
            <div key={index} className="flex items-center gap-2">
              <div className="flex flex-col gap-1">
                <button
                  type="button"
                  onClick={() => handleMoveStepUp(index)}
                  disabled={index === 0 || isSubmitting}
                  className="p-1 text-[--text-muted] hover:text-[--text-primary] disabled:opacity-30 disabled:cursor-not-allowed"
                  aria-label="Move step up"
                >
                  <ChevronUp className="w-4 h-4" />
                </button>
                <button
                  type="button"
                  onClick={() => handleMoveStepDown(index)}
                  disabled={index === steps.length - 1 || isSubmitting}
                  className="p-1 text-[--text-muted] hover:text-[--text-primary] disabled:opacity-30 disabled:cursor-not-allowed"
                  aria-label="Move step down"
                >
                  <ChevronDown className="w-4 h-4" />
                </button>
              </div>
              <div className="flex-1 flex items-center gap-2">
                <span className="text-sm text-[--text-muted] w-6">
                  {index + 1}.
                </span>
                <input
                  type="text"
                  value={step}
                  onChange={(e) => handleUpdateStep(index, e.target.value)}
                  disabled={isSubmitting}
                  placeholder={`Step ${index + 1}`}
                  className="flex-1 rounded-md px-3 py-2 text-sm bg-[--bg-elevated] border border-[--border-subtle] text-[--text-primary] placeholder-[--text-muted] focus:outline-none focus:ring-2 focus:ring-[--accent-primary] focus:border-transparent disabled:opacity-50 disabled:cursor-not-allowed"
                />
              </div>
              <button
                type="button"
                onClick={() => handleRemoveStep(index)}
                disabled={isSubmitting}
                className="p-2 text-[--text-muted] hover:text-[--status-error] disabled:opacity-50 disabled:cursor-not-allowed"
                aria-label="Remove step"
              >
                <X className="w-4 h-4" />
              </button>
            </div>
          ))}
        </div>
        <button
          type="button"
          onClick={handleAddStep}
          disabled={isSubmitting}
          className="mt-2 flex items-center gap-2 px-3 py-2 text-sm font-medium text-[--text-secondary] bg-transparent border border-[--border-subtle] rounded-md hover:bg-[--bg-hover] focus:outline-none focus:ring-2 focus:ring-[--accent-primary] disabled:opacity-50 disabled:cursor-not-allowed"
        >
          <Plus className="w-4 h-4" />
          Add Step
        </button>
      </div>

      {/* QA Toggle */}
      <div className="flex items-start gap-3 py-2">
        <input
          type="checkbox"
          id={`${baseId}-qa`}
          checked={needsQa}
          onChange={(e) => setNeedsQa(e.target.checked)}
          disabled={isSubmitting}
          aria-describedby={`${baseId}-qa-desc`}
          className="mt-1 h-4 w-4 rounded border-[--border-subtle] text-[--accent-primary] focus:ring-[--accent-primary] focus:ring-offset-[--bg-base] disabled:opacity-50 disabled:cursor-not-allowed"
        />
        <div className="flex-1">
          <label
            htmlFor={`${baseId}-qa`}
            className="text-sm font-medium text-[--text-primary]"
          >
            Enable QA for this task
          </label>
          <p
            id={`${baseId}-qa-desc`}
            className="mt-0.5 text-xs text-[--text-muted]"
          >
            Runs acceptance criteria generation and browser testing after task completion.
            If unchecked, inherits from global QA settings.
          </p>
        </div>
      </div>

      {/* Error Display */}
      {createMutation.isError && (
        <div className="p-3 rounded bg-[--status-error] bg-opacity-10 text-[--status-error] text-sm">
          {createMutation.error?.message || "Failed to create task"}
        </div>
      )}

      {/* Form Actions */}
      <div className="flex justify-end gap-3 pt-2">
        <button
          type="button"
          onClick={onCancel}
          disabled={isSubmitting}
          className="px-4 py-2 text-sm font-medium text-[--text-secondary] bg-transparent border border-[--border-subtle] rounded-md hover:bg-[--bg-hover] focus:outline-none focus:ring-2 focus:ring-[--accent-primary] disabled:opacity-50 disabled:cursor-not-allowed"
        >
          Cancel
        </button>
        <button
          type="submit"
          disabled={isSubmitting || !title.trim()}
          className="px-4 py-2 text-sm font-medium text-white bg-[--accent-primary] rounded-md hover:bg-[--accent-hover] focus:outline-none focus:ring-2 focus:ring-[--accent-primary] focus:ring-offset-2 focus:ring-offset-[--bg-base] disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {isSubmitting ? "Creating..." : "Create"}
        </button>
      </div>
    </form>
  );
}
