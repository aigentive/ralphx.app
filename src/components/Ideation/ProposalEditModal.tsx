/**
 * ProposalEditModal - Modal for editing task proposal details
 * Allows editing title, description, category, steps, acceptance criteria,
 * priority override, and complexity.
 */

import { useState, useCallback, useEffect, useRef } from "react";
import type { TaskProposal, UpdateProposalInput, Priority, Complexity } from "@/types/ideation";

const CATEGORIES = [
  { value: "setup", label: "Setup" },
  { value: "feature", label: "Feature" },
  { value: "integration", label: "Integration" },
  { value: "styling", label: "Styling" },
  { value: "testing", label: "Testing" },
  { value: "documentation", label: "Documentation" },
];

const PRIORITIES: { value: Priority | ""; label: string }[] = [
  { value: "", label: "Auto" },
  { value: "critical", label: "Critical" },
  { value: "high", label: "High" },
  { value: "medium", label: "Medium" },
  { value: "low", label: "Low" },
];

const COMPLEXITIES: { value: Complexity; label: string }[] = [
  { value: "trivial", label: "Trivial" },
  { value: "simple", label: "Simple" },
  { value: "moderate", label: "Moderate" },
  { value: "complex", label: "Complex" },
  { value: "very_complex", label: "Very Complex" },
];

interface ProposalEditModalProps {
  proposal: TaskProposal | null;
  onSave: (proposalId: string, data: UpdateProposalInput) => void;
  onCancel: () => void;
  isSaving?: boolean;
}

export function ProposalEditModal({
  proposal,
  onSave,
  onCancel,
  isSaving = false,
}: ProposalEditModalProps) {
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [category, setCategory] = useState("");
  const [steps, setSteps] = useState<string[]>([]);
  const [acceptanceCriteria, setAcceptanceCriteria] = useState<string[]>([]);
  const [userPriority, setUserPriority] = useState<Priority | "">("");
  const [complexity, setComplexity] = useState<Complexity>("moderate");

  const titleInputRef = useRef<HTMLInputElement>(null);

  // Initialize form state when proposal changes
  useEffect(() => {
    if (proposal) {
      setTitle(proposal.title);
      setDescription(proposal.description ?? "");
      setCategory(proposal.category);
      setSteps([...proposal.steps]);
      setAcceptanceCriteria([...proposal.acceptanceCriteria]);
      setUserPriority(proposal.userPriority ?? "");
      setComplexity(proposal.estimatedComplexity);
    }
  }, [proposal]);

  // Focus title input when modal opens
  useEffect(() => {
    if (proposal && titleInputRef.current) {
      titleInputRef.current.focus();
    }
  }, [proposal]);

  // Handle Escape key to close modal
  useEffect(() => {
    if (!proposal) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onCancel();
      }
    };

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [proposal, onCancel]);

  const handleOverlayClick = useCallback(() => {
    onCancel();
  }, [onCancel]);

  const handleContentClick = useCallback((e: React.MouseEvent) => {
    e.stopPropagation();
  }, []);

  const handleStepChange = useCallback((index: number, value: string) => {
    setSteps((prev) => prev.map((s, i) => (i === index ? value : s)));
  }, []);

  const handleAddStep = useCallback(() => {
    setSteps((prev) => [...prev, ""]);
  }, []);

  const handleRemoveStep = useCallback((index: number) => {
    setSteps((prev) => prev.filter((_, i) => i !== index));
  }, []);

  const handleCriterionChange = useCallback((index: number, value: string) => {
    setAcceptanceCriteria((prev) => prev.map((c, i) => (i === index ? value : c)));
  }, []);

  const handleAddCriterion = useCallback(() => {
    setAcceptanceCriteria((prev) => [...prev, ""]);
  }, []);

  const handleRemoveCriterion = useCallback((index: number) => {
    setAcceptanceCriteria((prev) => prev.filter((_, i) => i !== index));
  }, []);

  const handleSave = useCallback(() => {
    if (!proposal || !title.trim()) return;

    const data: UpdateProposalInput = {
      title: title.trim(),
      description: description.trim() || undefined,
      category,
      steps: steps.filter((s) => s.trim() !== ""),
      acceptanceCriteria: acceptanceCriteria.filter((c) => c.trim() !== ""),
      userPriority: userPriority || undefined,
      complexity,
    };

    onSave(proposal.id, data);
  }, [
    proposal,
    title,
    description,
    category,
    steps,
    acceptanceCriteria,
    userPriority,
    complexity,
    onSave,
  ]);

  if (!proposal) return null;

  const canSave = title.trim().length > 0 && !isSaving;

  const inputClasses =
    "w-full rounded-md px-3 py-2 text-sm border focus:outline-none focus:ring-2 focus:border-transparent";

  return (
    <div
      data-testid="proposal-edit-modal"
      className="fixed inset-0 z-50 flex items-center justify-center"
      role="dialog"
      aria-labelledby="modal-title"
      aria-modal="true"
    >
      <div
        data-testid="modal-overlay"
        className="absolute inset-0"
        style={{ backgroundColor: "rgba(0, 0, 0, 0.5)" }}
        onClick={handleOverlayClick}
      />
      <div
        data-testid="modal-content"
        className="relative w-full max-w-lg max-h-[90vh] overflow-y-auto p-6 rounded-lg shadow-lg"
        style={{ backgroundColor: "var(--bg-elevated)", borderColor: "var(--border-subtle)" }}
        onClick={handleContentClick}
      >
        <h2
          id="modal-title"
          className="text-lg font-semibold mb-4"
          style={{ color: "var(--text-primary)" }}
        >
          Edit Proposal
        </h2>

        <div className="space-y-4">
          {/* Title Input */}
          <div>
            <label
              htmlFor="proposal-title"
              className="block text-sm font-medium mb-1"
              style={{ color: "var(--text-primary)" }}
            >
              Title
            </label>
            <input
              ref={titleInputRef}
              id="proposal-title"
              type="text"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              className={inputClasses}
              style={{
                backgroundColor: "var(--bg-base)",
                borderColor: "var(--border-subtle)",
                color: "var(--text-primary)",
              }}
            />
          </div>

          {/* Description Textarea */}
          <div>
            <label
              htmlFor="proposal-description"
              className="block text-sm font-medium mb-1"
              style={{ color: "var(--text-primary)" }}
            >
              Description
            </label>
            <textarea
              id="proposal-description"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              rows={3}
              className={`${inputClasses} resize-none`}
              style={{
                backgroundColor: "var(--bg-base)",
                borderColor: "var(--border-subtle)",
                color: "var(--text-primary)",
              }}
            />
          </div>

          {/* Category Selector */}
          <div>
            <label
              htmlFor="proposal-category"
              className="block text-sm font-medium mb-1"
              style={{ color: "var(--text-primary)" }}
            >
              Category
            </label>
            <select
              id="proposal-category"
              value={category}
              onChange={(e) => setCategory(e.target.value)}
              className={inputClasses}
              style={{
                backgroundColor: "var(--bg-base)",
                borderColor: "var(--border-subtle)",
                color: "var(--text-primary)",
              }}
            >
              {CATEGORIES.map((cat) => (
                <option key={cat.value} value={cat.value}>
                  {cat.label}
                </option>
              ))}
            </select>
          </div>

          {/* Steps Editor */}
          <div>
            <div className="flex items-center justify-between mb-1">
              <span
                className="text-sm font-medium"
                style={{ color: "var(--text-primary)" }}
              >
                Steps
              </span>
              <button
                type="button"
                onClick={handleAddStep}
                aria-label="Add step"
                className="p-1 rounded hover:bg-[--bg-hover] transition-colors"
                style={{ color: "var(--text-secondary)" }}
              >
                <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                  <path d="M8 2v12M2 8h12" stroke="currentColor" strokeWidth="2" fill="none" />
                </svg>
              </button>
            </div>
            {steps.length === 0 ? (
              <p
                className="text-sm italic"
                style={{ color: "var(--text-muted)" }}
              >
                No steps added
              </p>
            ) : (
              <div className="space-y-2">
                {steps.map((step, index) => (
                  <div key={index} className="flex items-center gap-2">
                    <input
                      data-testid="step-input"
                      type="text"
                      value={step}
                      onChange={(e) => handleStepChange(index, e.target.value)}
                      aria-label={`Step ${index + 1}`}
                      className={`${inputClasses} flex-1`}
                      style={{
                        backgroundColor: "var(--bg-base)",
                        borderColor: "var(--border-subtle)",
                        color: "var(--text-primary)",
                      }}
                    />
                    <button
                      type="button"
                      onClick={() => handleRemoveStep(index)}
                      aria-label={`Remove step ${index + 1}`}
                      className="p-1 rounded hover:bg-[--bg-hover] transition-colors"
                      style={{ color: "var(--text-secondary)" }}
                    >
                      <svg width="14" height="14" viewBox="0 0 14 14" fill="currentColor">
                        <path d="M2 2l10 10M12 2L2 12" stroke="currentColor" strokeWidth="2" fill="none" />
                      </svg>
                    </button>
                  </div>
                ))}
              </div>
            )}
          </div>

          {/* Acceptance Criteria Editor */}
          <div>
            <div className="flex items-center justify-between mb-1">
              <span
                className="text-sm font-medium"
                style={{ color: "var(--text-primary)" }}
              >
                Acceptance Criteria
              </span>
              <button
                type="button"
                onClick={handleAddCriterion}
                aria-label="Add criterion"
                className="p-1 rounded hover:bg-[--bg-hover] transition-colors"
                style={{ color: "var(--text-secondary)" }}
              >
                <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                  <path d="M8 2v12M2 8h12" stroke="currentColor" strokeWidth="2" fill="none" />
                </svg>
              </button>
            </div>
            {acceptanceCriteria.length === 0 ? (
              <p
                className="text-sm italic"
                style={{ color: "var(--text-muted)" }}
              >
                No acceptance criteria added
              </p>
            ) : (
              <div className="space-y-2">
                {acceptanceCriteria.map((criterion, index) => (
                  <div key={index} className="flex items-center gap-2">
                    <input
                      data-testid="criterion-input"
                      type="text"
                      value={criterion}
                      onChange={(e) => handleCriterionChange(index, e.target.value)}
                      aria-label={`Acceptance criterion ${index + 1}`}
                      className={`${inputClasses} flex-1`}
                      style={{
                        backgroundColor: "var(--bg-base)",
                        borderColor: "var(--border-subtle)",
                        color: "var(--text-primary)",
                      }}
                    />
                    <button
                      type="button"
                      onClick={() => handleRemoveCriterion(index)}
                      aria-label={`Remove criterion ${index + 1}`}
                      className="p-1 rounded hover:bg-[--bg-hover] transition-colors"
                      style={{ color: "var(--text-secondary)" }}
                    >
                      <svg width="14" height="14" viewBox="0 0 14 14" fill="currentColor">
                        <path d="M2 2l10 10M12 2L2 12" stroke="currentColor" strokeWidth="2" fill="none" />
                      </svg>
                    </button>
                  </div>
                ))}
              </div>
            )}
          </div>

          {/* Priority Override Selector */}
          <div>
            <label
              htmlFor="proposal-priority"
              className="block text-sm font-medium mb-1"
              style={{ color: "var(--text-primary)" }}
            >
              Priority Override
            </label>
            <select
              id="proposal-priority"
              value={userPriority}
              onChange={(e) => setUserPriority(e.target.value as Priority | "")}
              className={inputClasses}
              style={{
                backgroundColor: "var(--bg-base)",
                borderColor: "var(--border-subtle)",
                color: "var(--text-primary)",
              }}
            >
              {PRIORITIES.map((p) => (
                <option key={p.value} value={p.value}>
                  {p.value === ""
                    ? `Auto (${proposal.suggestedPriority})`
                    : p.label}
                </option>
              ))}
            </select>
          </div>

          {/* Complexity Selector */}
          <div>
            <label
              htmlFor="proposal-complexity"
              className="block text-sm font-medium mb-1"
              style={{ color: "var(--text-primary)" }}
            >
              Complexity
            </label>
            <select
              id="proposal-complexity"
              value={complexity}
              onChange={(e) => setComplexity(e.target.value as Complexity)}
              className={inputClasses}
              style={{
                backgroundColor: "var(--bg-base)",
                borderColor: "var(--border-subtle)",
                color: "var(--text-primary)",
              }}
            >
              {COMPLEXITIES.map((c) => (
                <option key={c.value} value={c.value}>
                  {c.label}
                </option>
              ))}
            </select>
          </div>
        </div>

        {/* Footer with buttons */}
        <div className="flex justify-end gap-3 mt-6">
          <button
            type="button"
            onClick={onCancel}
            className="px-4 py-2 rounded text-sm font-medium transition-colors"
            style={{
              backgroundColor: "var(--bg-hover)",
              color: "var(--text-primary)",
            }}
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={handleSave}
            disabled={!canSave}
            className="px-4 py-2 rounded text-sm font-medium transition-colors"
            style={{
              backgroundColor: canSave ? "var(--accent-primary)" : "var(--bg-hover)",
              color: canSave ? "var(--bg-base)" : "var(--text-secondary)",
              cursor: canSave ? "pointer" : "not-allowed",
              opacity: isSaving ? 0.7 : 1,
            }}
          >
            {isSaving ? "Saving..." : "Save"}
          </button>
        </div>
      </div>
    </div>
  );
}
