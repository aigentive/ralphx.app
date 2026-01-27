/**
 * TaskCreationForm - Form for creating new tasks
 *
 * Uses shared TaskFormFields for consistent Refined Studio styling.
 *
 * Design spec: specs/design/refined-studio-patterns.md
 */

import { useState, useCallback, type FormEvent } from "react";
import { useTaskMutation } from "@/hooks/useTaskMutation";
import { CreateTaskSchema, type CreateTask } from "@/types/task";
import { TaskFormFields, TaskFormActions } from "./TaskFormFields";

// ============================================================================
// Types
// ============================================================================

export interface TaskCreationFormProps {
  /** Project ID to create the task in */
  projectId: string;
  /** Pre-fill the title field */
  defaultTitle?: string;
  /** Pre-fill the status/category (e.g., "draft", "backlog") */
  defaultStatus?: string;
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
  defaultTitle = "",
  defaultStatus,
  onSuccess,
  onCancel,
}: TaskCreationFormProps) {
  const { createMutation } = useTaskMutation(projectId);

  // Form state
  const [title, setTitle] = useState(defaultTitle);
  const [category, setCategory] = useState("feature");
  const [description, setDescription] = useState("");
  const [priority, setPriority] = useState(3); // Default to P3 - Medium
  const [validationError, setValidationError] = useState<string | null>(null);

  // Note: defaultStatus is currently unused as tasks always start in "draft" status
  void defaultStatus;

  const handleSubmit = useCallback(
    (e: FormEvent) => {
      e.preventDefault();
      setValidationError(null);

      if (!title.trim()) {
        setValidationError("Title is required");
        return;
      }

      const taskData: CreateTask = {
        projectId,
        title: title.trim(),
        category,
        priority,
        ...(description.trim() && { description: description.trim() }),
      };

      // Validate with Zod schema
      const result = CreateTaskSchema.safeParse(taskData);
      if (!result.success) {
        setValidationError(result.error.issues[0]?.message || "Validation failed");
        return;
      }

      createMutation.mutate(taskData, {
        onSuccess: () => {
          // Reset form
          setTitle("");
          setCategory("feature");
          setDescription("");
          setPriority(3);
          onSuccess?.();
        },
        onError: (error) => {
          setValidationError(error.message || "Failed to create task");
        },
      });
    },
    [title, category, description, priority, projectId, createMutation, onSuccess]
  );

  const isSubmitting = createMutation.isPending;

  return (
    <form onSubmit={handleSubmit} className="flex flex-col flex-1">
      <TaskFormFields
        title={title}
        setTitle={setTitle}
        category={category}
        setCategory={setCategory}
        description={description}
        setDescription={setDescription}
        priority={priority}
        setPriority={setPriority}
        disabled={isSubmitting}
        validationError={validationError}
      />

      <TaskFormActions
        onCancel={onCancel ?? (() => {})}
        submitLabel="Create Task"
        submitLoadingLabel="Creating..."
        isSubmitting={isSubmitting}
        isDisabled={!title.trim()}
      />
    </form>
  );
}
