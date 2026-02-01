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
import { Plus, X, ChevronUp, ChevronDown } from "lucide-react";
import { TaskFormFields, TaskFormActions } from "./TaskFormFields";
import {
  inputBaseStyles,
  buttonPrimaryStyles,
  labelStyles,
} from "./TaskFormFields.constants";

// ============================================================================
// Types
// ============================================================================

export interface TaskCreationFormProps {
  /** Project ID to create the task in */
  projectId: string;
  /** Pre-fill the title field */
  defaultTitle?: string;
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
  onSuccess,
  onCancel,
}: TaskCreationFormProps) {
  const { createMutation } = useTaskMutation(projectId);

  // Form state
  const [title, setTitle] = useState(defaultTitle);
  const [category, setCategory] = useState("feature");
  const [description, setDescription] = useState("");
  const [priority, setPriority] = useState(3); // Default to P3 - Medium
  const [steps, setSteps] = useState<string[]>([]);
  const [newStepTitle, setNewStepTitle] = useState("");
  const [validationError, setValidationError] = useState<string | null>(null);

  // Steps management
  const addStep = useCallback(() => {
    if (!newStepTitle.trim()) return;
    setSteps((prev) => [...prev, newStepTitle.trim()]);
    setNewStepTitle("");
  }, [newStepTitle]);

  const removeStep = useCallback((index: number) => {
    setSteps((prev) => prev.filter((_, i) => i !== index));
  }, []);

  const moveStepUp = useCallback((index: number) => {
    if (index === 0) return;
    setSteps((prev) => {
      const newSteps = [...prev];
      const item = newSteps[index];
      if (item !== undefined) {
        newSteps.splice(index, 1);
        newSteps.splice(index - 1, 0, item);
      }
      return newSteps;
    });
  }, []);

  const moveStepDown = useCallback((index: number) => {
    setSteps((prev) => {
      if (index >= prev.length - 1) return prev;
      const newSteps = [...prev];
      const item = newSteps[index];
      if (item !== undefined) {
        newSteps.splice(index, 1);
        newSteps.splice(index + 1, 0, item);
      }
      return newSteps;
    });
  }, []);

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
        ...(steps.length > 0 && { steps }),
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
          setSteps([]);
          setNewStepTitle("");
          onSuccess?.();
        },
        onError: (error) => {
          setValidationError(error.message || "Failed to create task");
        },
      });
    },
    [title, category, description, priority, steps, projectId, createMutation, onSuccess]
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

      {/* Steps Section - no boxing, just spacing */}
      <div className="mt-6">
        <label className={labelStyles}>Steps (Optional)</label>

        {/* Steps List */}
        {steps.length > 0 && (
          <div className="space-y-2 mb-3">
            {steps.map((step, index) => (
              <div
                key={index}
                className="flex items-center gap-2 px-3 py-2 rounded-lg"
                style={{
                  backgroundColor: "hsla(220 10% 100% / 0.03)",
                  border: "1px solid hsla(220 10% 100% / 0.06)",
                }}
              >
                <span
                  className="text-[11px] font-medium w-5"
                  style={{ color: "hsl(220 10% 40%)" }}
                >
                  {index + 1}.
                </span>
                <span
                  className="flex-1 text-[13px] truncate"
                  style={{ color: "hsl(220 10% 80%)" }}
                >
                  {step}
                </span>
                <div className="flex items-center gap-1">
                  <button
                    type="button"
                    onClick={() => moveStepUp(index)}
                    disabled={isSubmitting || index === 0}
                    className="p-1 rounded disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
                    style={{ color: "hsl(220 10% 40%)" }}
                    title="Move up"
                  >
                    <ChevronUp className="w-3.5 h-3.5" />
                  </button>
                  <button
                    type="button"
                    onClick={() => moveStepDown(index)}
                    disabled={isSubmitting || index === steps.length - 1}
                    className="p-1 rounded disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
                    style={{ color: "hsl(220 10% 40%)" }}
                    title="Move down"
                  >
                    <ChevronDown className="w-3.5 h-3.5" />
                  </button>
                  <button
                    type="button"
                    onClick={() => removeStep(index)}
                    disabled={isSubmitting}
                    className="p-1 rounded disabled:opacity-30 disabled:cursor-not-allowed transition-colors hover:bg-[hsla(0_70%_55%/0.1)]"
                    style={{ color: "hsl(220 10% 40%)" }}
                    title="Remove step"
                  >
                    <X className="w-3.5 h-3.5" />
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}

        {/* Add Step Input */}
        <div className="flex gap-2">
          <input
            type="text"
            value={newStepTitle}
            onChange={(e) => setNewStepTitle(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                addStep();
              }
            }}
            disabled={isSubmitting}
            placeholder="Add a step..."
            className={inputBaseStyles}
          />
          <button
            type="button"
            onClick={addStep}
            disabled={isSubmitting || !newStepTitle.trim()}
            className={buttonPrimaryStyles + " shrink-0"}
          >
            <Plus className="w-4 h-4" />
            <span className="hidden sm:inline">Add</span>
          </button>
        </div>
      </div>

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
