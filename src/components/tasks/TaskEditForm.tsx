/**
 * TaskEditForm - Form for editing existing tasks
 *
 * Features:
 * - Edit title, category, description, and priority
 * - Pre-populated with existing task data
 * - Form validation with Zod schema
 * - onSave callback for parent to handle mutation
 */

import { useState, useCallback, useId, type FormEvent } from "react";
import { TASK_CATEGORIES, UpdateTaskSchema, type Task, type UpdateTask } from "@/types/task";
import { Loader2 } from "lucide-react";

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

  return (
    <form onSubmit={handleSubmit} className="space-y-4">
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
          disabled={isSaving}
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
          disabled={isSaving}
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
          disabled={isSaving}
          rows={4}
          placeholder="Enter task description (optional)"
          className="mt-1 block w-full rounded-md px-3 py-2 text-sm bg-[--bg-elevated] border border-[--border-subtle] text-[--text-primary] placeholder-[--text-muted] focus:outline-none focus:ring-2 focus:ring-[--accent-primary] focus:border-transparent disabled:opacity-50 disabled:cursor-not-allowed resize-none"
        />
      </div>

      {/* Priority Field */}
      <div>
        <label
          htmlFor={`${baseId}-priority`}
          className="block text-sm font-medium text-[--text-primary]"
        >
          Priority
        </label>
        <select
          id={`${baseId}-priority`}
          value={priority}
          onChange={(e) => setPriority(Number(e.target.value))}
          disabled={isSaving}
          className="mt-1 block w-full rounded-md px-3 py-2 text-sm bg-[--bg-elevated] border border-[--border-subtle] text-[--text-primary] focus:outline-none focus:ring-2 focus:ring-[--accent-primary] focus:border-transparent disabled:opacity-50 disabled:cursor-not-allowed"
        >
          <option value={0}>P0 - Critical</option>
          <option value={1}>P1 - High</option>
          <option value={2}>P2 - Medium</option>
          <option value={3}>P3 - Low</option>
        </select>
      </div>

      {/* Error Display */}
      {validationError && (
        <div className="p-3 rounded bg-[--status-error] bg-opacity-10 text-[--status-error] text-sm">
          {validationError}
        </div>
      )}

      {/* Form Actions */}
      <div className="flex justify-end gap-3 pt-2">
        <button
          type="button"
          onClick={onCancel}
          disabled={isSaving}
          className="px-4 py-2 text-sm font-medium text-[--text-secondary] bg-transparent border border-[--border-subtle] rounded-md hover:bg-[--bg-hover] focus:outline-none focus:ring-2 focus:ring-[--accent-primary] disabled:opacity-50 disabled:cursor-not-allowed"
        >
          Cancel
        </button>
        <button
          type="submit"
          disabled={isSaving || !title.trim() || !hasChanges}
          className="px-4 py-2 text-sm font-medium text-white bg-[--accent-primary] rounded-md hover:bg-[--accent-hover] focus:outline-none focus:ring-2 focus:ring-[--accent-primary] focus:ring-offset-2 focus:ring-offset-[--bg-base] disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
        >
          {isSaving && <Loader2 className="w-4 h-4 animate-spin" />}
          {isSaving ? "Saving..." : "Save Changes"}
        </button>
      </div>
    </form>
  );
}
