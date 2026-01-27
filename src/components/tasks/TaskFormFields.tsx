/**
 * TaskFormFields - Shared form field components for task forms
 *
 * Extracted from TaskEditForm to be reused by TaskCreationForm.
 * Provides consistent Refined Studio styling across all task forms.
 *
 * Design spec: specs/design/refined-studio-patterns.md
 */

import { useId } from "react";
import { TASK_CATEGORIES } from "@/types/task";
import { AlertCircle } from "lucide-react";

// ============================================================================
// Shared Styles
// ============================================================================

export const inputBaseStyles = `
  w-full h-10 px-3 rounded-lg text-[13px]
  bg-white/[0.03] border border-white/[0.08]
  text-white/90 placeholder:text-white/30
  transition-all duration-150
  focus:outline-none focus:border-[#ff6b35]/50 focus:bg-white/[0.05]
  focus:shadow-[0_0_0_3px_rgba(255,107,53,0.1)]
  disabled:opacity-50 disabled:cursor-not-allowed
`.replace(/\n/g, ' ').trim();

export const selectBaseStyles = `
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

export const textareaBaseStyles = `
  w-full px-3 py-2.5 rounded-lg text-[13px] leading-relaxed
  bg-white/[0.03] border border-white/[0.08]
  text-white/90 placeholder:text-white/30
  transition-all duration-150 resize-none
  focus:outline-none focus:border-[#ff6b35]/50 focus:bg-white/[0.05]
  focus:shadow-[0_0_0_3px_rgba(255,107,53,0.1)]
  disabled:opacity-50 disabled:cursor-not-allowed
`.replace(/\n/g, ' ').trim();

export const labelStyles = "block text-[12px] font-medium text-white/50 uppercase tracking-wide mb-2";

export const buttonPrimaryStyles = `
  h-10 px-4 rounded-lg text-[13px] font-medium
  bg-[#ff6b35] text-white
  transition-all duration-150
  hover:bg-[#ff8050] hover:shadow-[0_4px_12px_rgba(255,107,53,0.3)]
  focus:outline-none focus:shadow-[0_0_0_3px_rgba(255,107,53,0.3)]
  disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:shadow-none
  flex items-center justify-center gap-2
`.replace(/\n/g, ' ').trim();

export const buttonSecondaryStyles = `
  h-10 px-4 rounded-lg text-[13px] font-medium
  bg-transparent border border-white/[0.1] text-white/70
  transition-all duration-150
  hover:bg-white/[0.05] hover:border-white/[0.15] hover:text-white/90
  focus:outline-none focus:shadow-[0_0_0_3px_rgba(255,255,255,0.05)]
  disabled:opacity-50 disabled:cursor-not-allowed
`.replace(/\n/g, ' ').trim();

// ============================================================================
// Form Field Components
// ============================================================================

interface TaskFormFieldsProps {
  title: string;
  setTitle: (value: string) => void;
  category: string;
  setCategory: (value: string) => void;
  description: string;
  setDescription: (value: string) => void;
  priority: number;
  setPriority: (value: number) => void;
  disabled?: boolean;
  /** Optional validation error to display */
  validationError?: string | null;
}

/**
 * Reusable form fields for task creation and editing
 */
export function TaskFormFields({
  title,
  setTitle,
  category,
  setCategory,
  description,
  setDescription,
  priority,
  setPriority,
  disabled = false,
  validationError,
}: TaskFormFieldsProps) {
  const baseId = useId();

  return (
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
          disabled={disabled}
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
            disabled={disabled}
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
            disabled={disabled}
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
          disabled={disabled}
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
    </div>
  );
}

// ============================================================================
// Form Actions Component
// ============================================================================

interface TaskFormActionsProps {
  onCancel: () => void;
  onSubmit?: () => void;
  submitLabel: string;
  submitLoadingLabel: string;
  isSubmitting: boolean;
  isDisabled: boolean;
}

/**
 * Reusable form action buttons (Cancel / Submit)
 */
export function TaskFormActions({
  onCancel,
  submitLabel,
  submitLoadingLabel,
  isSubmitting,
  isDisabled,
}: TaskFormActionsProps) {
  return (
    <div
      className="flex justify-end gap-3 pt-4 mt-auto border-t"
      style={{ borderColor: "rgba(255,255,255,0.06)" }}
    >
      <button
        type="button"
        onClick={onCancel}
        disabled={isSubmitting}
        className={buttonSecondaryStyles}
      >
        Cancel
      </button>
      <button
        type="submit"
        disabled={isSubmitting || isDisabled}
        className={buttonPrimaryStyles}
      >
        {isSubmitting ? submitLoadingLabel : submitLabel}
      </button>
    </div>
  );
}
