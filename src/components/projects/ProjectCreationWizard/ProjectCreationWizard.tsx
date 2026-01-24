/**
 * ProjectCreationWizard - Modal for creating a new project with Git Mode selection
 *
 * Supports two Git modes:
 * - Local: Work directly in the user's current branch
 * - Worktree: Create an isolated worktree for RalphX to work in
 */

import { useState, useEffect, useCallback, useMemo } from "react";
import type { GitMode, CreateProject } from "@/types/project";

// ============================================================================
// Props Interface
// ============================================================================

export interface ProjectCreationWizardProps {
  /** Whether the modal is open */
  isOpen: boolean;
  /** Callback when the modal is closed */
  onClose: () => void;
  /** Callback when a project is created */
  onCreate: (project: CreateProject) => void;
  /** Callback to open folder picker, returns selected path or null if cancelled */
  onBrowseFolder?: () => Promise<string | null>;
  /** Callback to fetch available branches for base branch dropdown */
  onFetchBranches?: (workingDirectory: string) => Promise<string[]>;
  /** Whether creation is in progress */
  isCreating?: boolean;
  /** Error message to display */
  error?: string | null;
}

// ============================================================================
// Icons
// ============================================================================

function FolderIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
      <path
        d="M2 4a1 1 0 011-1h3.586a1 1 0 01.707.293L8 4h5a1 1 0 011 1v7a1 1 0 01-1 1H3a1 1 0 01-1-1V4z"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function CloseIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
      <path
        d="M12 4L4 12M4 4l8 8"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
      />
    </svg>
  );
}

function WarningIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
      <path
        d="M7 1L13 12H1L7 1z"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <path d="M7 5v3M7 10v.5" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
    </svg>
  );
}

function GitBranchIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
      <circle cx="3.5" cy="3.5" r="1.5" stroke="currentColor" strokeWidth="1.2" />
      <circle cx="3.5" cy="10.5" r="1.5" stroke="currentColor" strokeWidth="1.2" />
      <circle cx="10.5" cy="6" r="1.5" stroke="currentColor" strokeWidth="1.2" />
      <path d="M3.5 5V9M9 6H5.5C5.5 6 5.5 3.5 3.5 3.5" stroke="currentColor" strokeWidth="1.2" />
    </svg>
  );
}

function ChevronDownIcon() {
  return (
    <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
      <path
        d="M3 4.5L6 7.5L9 4.5"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

// ============================================================================
// Form State Interface
// ============================================================================

interface FormState {
  name: string;
  workingDirectory: string;
  gitMode: GitMode;
  worktreeBranch: string;
  baseBranch: string;
}

interface FormErrors {
  name?: string;
  workingDirectory?: string;
  worktreeBranch?: string;
  baseBranch?: string;
}

// ============================================================================
// Helper Functions
// ============================================================================

/**
 * Generate default branch name from project name
 * Format: ralphx/<project-name-slug>
 */
function generateBranchName(projectName: string): string {
  const slug = projectName
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");
  return slug ? `ralphx/${slug}` : "ralphx/feature";
}

/**
 * Generate worktree path from working directory and branch
 * Format: ~/ralphx-worktrees/<project-folder-name>
 */
function generateWorktreePath(workingDirectory: string): string {
  const folderName = workingDirectory.split("/").pop() || "project";
  return `~/ralphx-worktrees/${folderName}`;
}

/**
 * Validate the form and return errors
 */
function validateForm(form: FormState): FormErrors {
  const errors: FormErrors = {};

  if (!form.name.trim()) {
    errors.name = "Project name is required";
  }

  if (!form.workingDirectory.trim()) {
    errors.workingDirectory = "Working directory is required";
  }

  if (form.gitMode === "worktree") {
    if (!form.worktreeBranch.trim()) {
      errors.worktreeBranch = "Branch name is required";
    } else if (!/^[a-zA-Z0-9/_-]+$/.test(form.worktreeBranch)) {
      errors.worktreeBranch = "Branch name contains invalid characters";
    }

    if (!form.baseBranch.trim()) {
      errors.baseBranch = "Base branch is required";
    }
  }

  return errors;
}

// ============================================================================
// Sub-components
// ============================================================================

interface InputFieldProps {
  label: string;
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  error?: string | undefined;
  testId: string;
  autoFocus?: boolean;
  disabled?: boolean;
  suffix?: React.ReactNode;
}

function InputField({
  label,
  value,
  onChange,
  placeholder,
  error,
  testId,
  autoFocus,
  disabled,
  suffix,
}: InputFieldProps) {
  return (
    <div className="space-y-1.5">
      <label
        className="block text-sm font-medium"
        style={{ color: "var(--text-secondary)" }}
      >
        {label}
      </label>
      <div className="flex gap-2">
        <input
          data-testid={testId}
          type="text"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={placeholder}
          autoFocus={autoFocus}
          disabled={disabled}
          className="flex-1 px-3 py-2 rounded-lg text-sm border transition-colors"
          style={{
            backgroundColor: "var(--bg-base)",
            borderColor: error ? "var(--status-error)" : "var(--border-subtle)",
            color: "var(--text-primary)",
            opacity: disabled ? 0.5 : 1,
          }}
        />
        {suffix}
      </div>
      {error && (
        <p
          data-testid={`${testId}-error`}
          className="text-xs"
          style={{ color: "var(--status-error)" }}
        >
          {error}
        </p>
      )}
    </div>
  );
}

interface SelectFieldProps {
  label: string;
  value: string;
  onChange: (value: string) => void;
  options: string[];
  error?: string | undefined;
  testId: string;
  disabled?: boolean;
  loading?: boolean;
}

function SelectField({
  label,
  value,
  onChange,
  options,
  error,
  testId,
  disabled,
  loading,
}: SelectFieldProps) {
  return (
    <div className="space-y-1.5">
      <label
        className="block text-sm font-medium"
        style={{ color: "var(--text-secondary)" }}
      >
        {label}
      </label>
      <div className="relative">
        <select
          data-testid={testId}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          disabled={disabled || loading}
          className="w-full px-3 py-2 rounded-lg text-sm border transition-colors appearance-none pr-8"
          style={{
            backgroundColor: "var(--bg-base)",
            borderColor: error ? "var(--status-error)" : "var(--border-subtle)",
            color: "var(--text-primary)",
            opacity: disabled || loading ? 0.5 : 1,
          }}
        >
          {loading ? (
            <option>Loading branches...</option>
          ) : options.length === 0 ? (
            <option>No branches available</option>
          ) : (
            options.map((opt) => (
              <option key={opt} value={opt}>
                {opt}
              </option>
            ))
          )}
        </select>
        <span
          className="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none"
          style={{ color: "var(--text-muted)" }}
        >
          <ChevronDownIcon />
        </span>
      </div>
      {error && (
        <p
          data-testid={`${testId}-error`}
          className="text-xs"
          style={{ color: "var(--status-error)" }}
        >
          {error}
        </p>
      )}
    </div>
  );
}

interface RadioOptionProps {
  value: GitMode;
  selected: boolean;
  onSelect: (value: GitMode) => void;
  label: string;
  description: string;
  warning?: string;
  testId: string;
  children?: React.ReactNode;
}

function RadioOption({
  value,
  selected,
  onSelect,
  label,
  description,
  warning,
  testId,
  children,
}: RadioOptionProps) {
  return (
    <label
      data-testid={testId}
      data-selected={selected ? "true" : "false"}
      className="flex gap-3 p-3 rounded-lg cursor-pointer transition-colors"
      style={{
        backgroundColor: selected ? "var(--bg-elevated)" : "transparent",
        border: `1px solid ${selected ? "var(--accent-primary)" : "var(--border-subtle)"}`,
      }}
    >
      <input
        type="radio"
        name="gitMode"
        value={value}
        checked={selected}
        onChange={() => onSelect(value)}
        className="sr-only"
      />
      <span
        className="mt-0.5 w-4 h-4 rounded-full border-2 flex items-center justify-center flex-shrink-0"
        style={{
          borderColor: selected ? "var(--accent-primary)" : "var(--border-subtle)",
        }}
      >
        {selected && (
          <span
            className="w-2 h-2 rounded-full"
            style={{ backgroundColor: "var(--accent-primary)" }}
          />
        )}
      </span>
      <div className="flex-1 min-w-0">
        <div className="text-sm font-medium" style={{ color: "var(--text-primary)" }}>
          {label}
        </div>
        <div className="text-xs mt-0.5" style={{ color: "var(--text-muted)" }}>
          {description}
        </div>
        {warning && (
          <div
            className="flex items-center gap-1.5 text-xs mt-1.5"
            style={{ color: "var(--status-warning)" }}
          >
            <WarningIcon />
            <span>{warning}</span>
          </div>
        )}
        {selected && children && <div className="mt-3 space-y-3">{children}</div>}
      </div>
    </label>
  );
}

// ============================================================================
// Main Component
// ============================================================================

export function ProjectCreationWizard({
  isOpen,
  onClose,
  onCreate,
  onBrowseFolder,
  onFetchBranches,
  isCreating = false,
  error = null,
}: ProjectCreationWizardProps) {
  // Form state
  const [form, setForm] = useState<FormState>({
    name: "",
    workingDirectory: "",
    gitMode: "local",
    worktreeBranch: "ralphx/feature",
    baseBranch: "main",
  });

  // Available branches for dropdown
  const [branches, setBranches] = useState<string[]>(["main", "master"]);
  const [loadingBranches, setLoadingBranches] = useState(false);

  // Touched fields for validation display
  const [touched, setTouched] = useState<Record<string, boolean>>({});

  // Track if form was submitted (to show all errors)
  const [submitted, setSubmitted] = useState(false);

  // Validate form
  const errors = useMemo(() => validateForm(form), [form]);
  const hasErrors = Object.keys(errors).length > 0;

  // Generate worktree path
  const worktreePath = useMemo(
    () => generateWorktreePath(form.workingDirectory),
    [form.workingDirectory]
  );

  // Update branch name when project name changes
  useEffect(() => {
    if (form.name && form.gitMode === "worktree") {
      setForm((prev) => ({
        ...prev,
        worktreeBranch: generateBranchName(prev.name),
      }));
    }
  }, [form.name, form.gitMode]);

  // Fetch branches when working directory changes
  useEffect(() => {
    if (form.workingDirectory && onFetchBranches) {
      setLoadingBranches(true);
      onFetchBranches(form.workingDirectory)
        .then((fetchedBranches) => {
          if (fetchedBranches.length > 0) {
            setBranches(fetchedBranches);
            // Set default base branch
            const defaultBranch = fetchedBranches.find(
              (b) => b === "main" || b === "master"
            );
            if (defaultBranch) {
              setForm((prev) => ({ ...prev, baseBranch: defaultBranch }));
            }
          }
        })
        .finally(() => setLoadingBranches(false));
    }
  }, [form.workingDirectory, onFetchBranches]);

  // Reset form when modal opens
  useEffect(() => {
    if (isOpen) {
      setForm({
        name: "",
        workingDirectory: "",
        gitMode: "local",
        worktreeBranch: "ralphx/feature",
        baseBranch: "main",
      });
      setTouched({});
      setSubmitted(false);
      setBranches(["main", "master"]);
    }
  }, [isOpen]);

  // Handle folder browse
  const handleBrowse = useCallback(async () => {
    if (onBrowseFolder) {
      const path = await onBrowseFolder();
      if (path) {
        setForm((prev) => ({ ...prev, workingDirectory: path }));
        setTouched((prev) => ({ ...prev, workingDirectory: true }));
      }
    }
  }, [onBrowseFolder]);

  // Handle form submission
  const handleSubmit = useCallback(() => {
    // Mark as submitted to show all errors
    setSubmitted(true);

    // Mark all fields as touched to show errors
    setTouched({
      name: true,
      workingDirectory: true,
      worktreeBranch: true,
      baseBranch: true,
    });

    // Check validation - use the errors object directly instead of hasErrors
    // because hasErrors might not reflect the current form state
    const currentErrors = validateForm(form);
    if (Object.keys(currentErrors).length > 0) return;

    const project: CreateProject = {
      name: form.name.trim(),
      workingDirectory: form.workingDirectory.trim(),
      gitMode: form.gitMode,
    };

    if (form.gitMode === "worktree") {
      project.worktreeBranch = form.worktreeBranch.trim();
      project.baseBranch = form.baseBranch.trim();
      project.worktreePath = worktreePath;
    }

    onCreate(project);
  }, [form, onCreate, worktreePath]);

  // Don't render if not open
  if (!isOpen) return null;

  return (
    <div
      data-testid="project-creation-wizard"
      className="fixed inset-0 z-50 flex items-center justify-center"
    >
      {/* Backdrop */}
      <div
        data-testid="wizard-overlay"
        className="absolute inset-0 transition-opacity"
        style={{ backgroundColor: "rgba(0, 0, 0, 0.5)" }}
        onClick={onClose}
      />

      {/* Modal */}
      <div
        data-testid="wizard-modal"
        className="relative w-full max-w-lg mx-4 rounded-xl shadow-2xl"
        style={{
          backgroundColor: "var(--bg-surface)",
          border: "1px solid var(--border-subtle)",
        }}
      >
        {/* Header */}
        <div
          className="flex items-center justify-between px-6 py-4 border-b"
          style={{ borderColor: "var(--border-subtle)" }}
        >
          <h2
            className="text-lg font-semibold"
            style={{ color: "var(--text-primary)" }}
          >
            Create New Project
          </h2>
          <button
            data-testid="wizard-close"
            onClick={onClose}
            disabled={isCreating}
            className="p-1 rounded transition-colors hover:bg-white/5"
            style={{ color: "var(--text-muted)" }}
          >
            <CloseIcon />
          </button>
        </div>

        {/* Content */}
        <div className="px-6 py-5 space-y-5">
          {/* Project Name */}
          <InputField
            label="Project Name"
            value={form.name}
            onChange={(value) => setForm((prev) => ({ ...prev, name: value }))}
            placeholder="My Awesome Project"
            error={(touched.name || submitted) ? errors.name : undefined}
            testId="project-name-input"
            autoFocus
            disabled={isCreating}
          />

          {/* Working Directory */}
          <InputField
            label="Folder"
            value={form.workingDirectory}
            onChange={(value) =>
              setForm((prev) => ({ ...prev, workingDirectory: value }))
            }
            placeholder="/Users/dev/my-app"
            error={(touched.workingDirectory || submitted) ? errors.workingDirectory : undefined}
            testId="folder-input"
            disabled={isCreating}
            suffix={
              onBrowseFolder && (
                <button
                  data-testid="browse-button"
                  onClick={handleBrowse}
                  disabled={isCreating}
                  className="px-3 py-2 rounded-lg text-sm font-medium transition-colors flex items-center gap-2"
                  style={{
                    backgroundColor: "var(--bg-elevated)",
                    color: "var(--text-primary)",
                  }}
                >
                  <FolderIcon />
                  Browse
                </button>
              )
            }
          />

          {/* Divider */}
          <div
            className="h-px"
            style={{ backgroundColor: "var(--border-subtle)" }}
          />

          {/* Git Mode Selection */}
          <div className="space-y-3">
            <label
              className="block text-sm font-medium"
              style={{ color: "var(--text-secondary)" }}
            >
              Git Mode
            </label>

            {/* Local Mode */}
            <RadioOption
              value="local"
              selected={form.gitMode === "local"}
              onSelect={(value) => setForm((prev) => ({ ...prev, gitMode: value }))}
              label="Local (default)"
              description="Work directly in your current branch"
              warning="Your uncommitted changes may be affected"
              testId="git-mode-local"
            />

            {/* Worktree Mode */}
            <RadioOption
              value="worktree"
              selected={form.gitMode === "worktree"}
              onSelect={(value) => setForm((prev) => ({ ...prev, gitMode: value }))}
              label="Isolated Worktree (recommended when actively coding)"
              description="Creates separate worktree for RalphX to work in. Your branch stays untouched."
              testId="git-mode-worktree"
            >
              {/* Worktree-specific fields */}
              <div className="space-y-3">
                <InputField
                  label="Branch name"
                  value={form.worktreeBranch}
                  onChange={(value) =>
                    setForm((prev) => ({ ...prev, worktreeBranch: value }))
                  }
                  placeholder="ralphx/feature-name"
                  error={(touched.worktreeBranch || submitted) ? errors.worktreeBranch : undefined}
                  testId="worktree-branch-input"
                  disabled={isCreating}
                />

                <SelectField
                  label="Base branch"
                  value={form.baseBranch}
                  onChange={(value) =>
                    setForm((prev) => ({ ...prev, baseBranch: value }))
                  }
                  options={branches}
                  error={(touched.baseBranch || submitted) ? errors.baseBranch : undefined}
                  testId="base-branch-select"
                  disabled={isCreating}
                  loading={loadingBranches}
                />

                {/* Worktree Path Display */}
                <div
                  data-testid="worktree-path-display"
                  className="flex items-center gap-2 px-3 py-2 rounded-lg"
                  style={{ backgroundColor: "var(--bg-base)" }}
                >
                  <GitBranchIcon />
                  <div className="flex-1 min-w-0">
                    <div
                      className="text-xs font-medium"
                      style={{ color: "var(--text-muted)" }}
                    >
                      Worktree location
                    </div>
                    <div
                      className="text-sm truncate"
                      style={{ color: "var(--text-primary)" }}
                    >
                      {worktreePath}
                    </div>
                  </div>
                </div>
              </div>
            </RadioOption>
          </div>

          {/* Error Message */}
          {error && (
            <div
              data-testid="wizard-error"
              className="flex items-center gap-2 px-3 py-2 rounded-lg"
              style={{
                backgroundColor: "rgba(239, 68, 68, 0.1)",
                color: "var(--status-error)",
              }}
            >
              <WarningIcon />
              <span className="text-sm">{error}</span>
            </div>
          )}
        </div>

        {/* Footer */}
        <div
          className="flex items-center justify-end gap-3 px-6 py-4 border-t"
          style={{ borderColor: "var(--border-subtle)" }}
        >
          <button
            data-testid="cancel-button"
            onClick={onClose}
            disabled={isCreating}
            className="px-4 py-2 rounded-lg text-sm font-medium transition-colors"
            style={{
              backgroundColor: "var(--bg-elevated)",
              color: "var(--text-primary)",
            }}
          >
            Cancel
          </button>
          <button
            data-testid="create-button"
            onClick={handleSubmit}
            disabled={isCreating}
            className="px-4 py-2 rounded-lg text-sm font-medium transition-colors"
            style={{
              backgroundColor:
                isCreating || (submitted && hasErrors)
                  ? "var(--bg-hover)"
                  : "var(--accent-primary)",
              color: isCreating || (submitted && hasErrors) ? "var(--text-muted)" : "#fff",
              cursor: isCreating ? "not-allowed" : "pointer",
            }}
          >
            {isCreating ? "Creating..." : "Create Project"}
          </button>
        </div>
      </div>
    </div>
  );
}
