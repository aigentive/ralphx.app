/**
 * ProposalEditModal - Modal for editing task proposal details
 * Allows editing title, description, category, steps, acceptance criteria,
 * priority override, and complexity.
 *
 * Uses shadcn/ui Dialog, Input, Textarea, Select, Button, Label components.
 */

import { useState, useCallback, useEffect, useRef } from "react";
import { Edit3, Plus, X, Loader2, Layers, CheckCircle } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogFooter,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";
import { ScrollArea } from "@/components/ui/scroll-area";
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

// Circled numbers for step prefixes (supports up to 10 steps)
const CIRCLED_NUMBERS = ["①", "②", "③", "④", "⑤", "⑥", "⑦", "⑧", "⑨", "⑩"];

/**
 * ComplexitySelector - Visual 5-dot complexity picker
 * Displays 5 circles representing trivial → very_complex
 * Orange fill for selected, transparent for others
 */
interface ComplexitySelectorProps {
  value: Complexity;
  onChange: (value: Complexity) => void;
  disabled?: boolean;
}

function ComplexitySelector({ value, onChange, disabled }: ComplexitySelectorProps) {
  const selectedIndex = COMPLEXITIES.findIndex((c) => c.value === value);

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center gap-2">
        {COMPLEXITIES.map((c, index) => {
          const isSelected = index <= selectedIndex;
          return (
            <button
              key={c.value}
              type="button"
              onClick={() => !disabled && onChange(c.value)}
              disabled={disabled}
              title={c.label}
              aria-label={`Set complexity to ${c.label}`}
              className={`
                w-4 h-4 rounded-full border transition-all duration-150
                ${isSelected
                  ? "bg-[#ff6b35] border-[#ff6b35]"
                  : "bg-transparent border-white/30 hover:border-[#ff6b35]/50"
                }
                ${!disabled ? "cursor-pointer hover:scale-125" : "cursor-not-allowed opacity-50"}
              `}
            />
          );
        })}
      </div>
      <span className="text-sm text-[var(--text-muted)] capitalize">
        {COMPLEXITIES[selectedIndex]?.label ?? "Moderate"}
      </span>
    </div>
  );
}

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

  const handleOpenChange = useCallback(
    (open: boolean) => {
      if (!open && !isSaving) {
        onCancel();
      }
    },
    [onCancel, isSaving]
  );

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

  // Glass effect styling for inputs - per Task 6 design specs + Task 8 micro-interactions
  const inputClasses = "bg-black/30 border border-white/[0.08] rounded-lg text-[var(--text-primary)] placeholder:text-[var(--text-muted)] transition-all duration-200 focus:border-[#ff6b35]/50 focus:ring-2 focus:ring-[#ff6b35]/10 focus:outline-none hover:scale-[1.01] focus:scale-[1.01]";
  const selectClasses = "w-full h-9 rounded-lg border px-3 py-2 text-sm bg-black/30 border-white/[0.08] text-[var(--text-primary)] transition-all duration-200 focus:outline-none focus:ring-2 focus:ring-[#ff6b35]/10 focus:border-[#ff6b35]/50 hover:scale-[1.01] focus:scale-[1.01]";

  return (
    <Dialog open={!!proposal} onOpenChange={handleOpenChange}>
      <DialogContent
        data-testid="proposal-edit-modal"
        className="max-w-2xl max-h-[90vh] animate-modal-slide-up"
        aria-labelledby="modal-title"
      >
        <DialogHeader>
          <div className="flex items-center gap-3">
            <div className="bg-[#ff6b35]/10 rounded-full p-1.5">
              <Edit3 className="w-5 h-5 text-[#ff6b35]" />
            </div>
            <div className="flex flex-col">
              <DialogTitle id="modal-title">Edit Proposal</DialogTitle>
              <p className="text-sm text-[var(--text-muted)]">Refine your task proposal</p>
            </div>
          </div>
        </DialogHeader>

        <ScrollArea className="max-h-[60vh]">
          <div data-testid="modal-content" className="px-6 py-4 space-y-4">
            {/* Title Input */}
            <div className="space-y-2 animate-stagger-fade-in" style={{ animationDelay: "0ms" }}>
              <Label htmlFor="proposal-title" className="text-[var(--text-primary)]">
                Title
              </Label>
              <Input
                ref={titleInputRef}
                id="proposal-title"
                type="text"
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                className={inputClasses}
                disabled={isSaving}
              />
            </div>

            {/* Description Textarea */}
            <div className="space-y-2 animate-stagger-fade-in" style={{ animationDelay: "50ms" }}>
              <Label htmlFor="proposal-description" className="text-[var(--text-primary)]">
                Description
              </Label>
              <Textarea
                id="proposal-description"
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                rows={3}
                className={`${inputClasses} resize-none`}
                disabled={isSaving}
              />
            </div>

            {/* Two-Column Metadata Panel with Glass Effect */}
            <div className="rounded-lg border border-white/[0.08] bg-white/[0.03] backdrop-blur-xl p-4 animate-stagger-fade-in" style={{ animationDelay: "100ms" }}>
              <div className="grid grid-cols-[1fr_auto_1fr] gap-4 items-start">
                {/* Left Column: Category + Priority Override */}
                <div className="space-y-4">
                  {/* Category Selector */}
                  <div className="space-y-2">
                    <Label htmlFor="proposal-category" className="text-[var(--text-primary)] text-sm">
                      Category
                    </Label>
                    <select
                      id="proposal-category"
                      value={category}
                      onChange={(e) => setCategory(e.target.value)}
                      className={selectClasses}
                      disabled={isSaving}
                    >
                      {CATEGORIES.map((cat) => (
                        <option key={cat.value} value={cat.value}>
                          {cat.label}
                        </option>
                      ))}
                    </select>
                  </div>

                  {/* Priority Override Selector */}
                  <div className="space-y-2">
                    <Label htmlFor="proposal-priority" className="text-[var(--text-primary)] text-sm">
                      Priority Override
                    </Label>
                    <select
                      id="proposal-priority"
                      value={userPriority}
                      onChange={(e) => setUserPriority(e.target.value as Priority | "")}
                      className={selectClasses}
                      disabled={isSaving}
                    >
                      {PRIORITIES.map((p) => (
                        <option key={p.value || "auto"} value={p.value}>
                          {p.value === "" ? `Auto (${proposal.suggestedPriority})` : p.label}
                        </option>
                      ))}
                    </select>
                  </div>
                </div>

                {/* Vertical Divider */}
                <div className="w-px self-stretch bg-white/[0.08]" />

                {/* Right Column: Visual Complexity Selector */}
                <div className="space-y-2">
                  <Label className="text-[var(--text-primary)] text-sm">
                    Complexity
                  </Label>
                  <ComplexitySelector
                    value={complexity}
                    onChange={setComplexity}
                    disabled={isSaving}
                  />
                </div>
              </div>
            </div>

            {/* Steps Editor with Glass Container */}
            <div className="space-y-2 animate-stagger-fade-in" style={{ animationDelay: "150ms" }}>
              <Label className="text-[var(--text-primary)]">Implementation Steps</Label>
              <div className="rounded-lg border border-white/[0.08] bg-white/[0.03] backdrop-blur-xl p-4">
                {steps.length === 0 ? (
                  <div className="flex flex-col items-center justify-center py-6 text-center">
                    <Layers className="w-8 h-8 text-[var(--text-muted)] mb-2" />
                    <p className="text-sm text-[var(--text-muted)]">No steps defined yet</p>
                    <p className="text-xs text-[var(--text-muted)]/60 mt-1">Add steps to outline the implementation</p>
                  </div>
                ) : (
                  <div className="space-y-2">
                    {steps.map((step, index) => (
                      <div key={index} className="group flex items-center gap-3">
                        <span className="text-[#ff6b35] text-lg font-medium w-6 flex-shrink-0">
                          {CIRCLED_NUMBERS[index] ?? `${index + 1}.`}
                        </span>
                        <Input
                          data-testid="step-input"
                          type="text"
                          value={step}
                          onChange={(e) => handleStepChange(index, e.target.value)}
                          aria-label={`Step ${index + 1}`}
                          className={`${inputClasses} flex-1`}
                          disabled={isSaving}
                        />
                        <Button
                          type="button"
                          variant="ghost"
                          size="icon-sm"
                          onClick={() => handleRemoveStep(index)}
                          aria-label={`Remove step ${index + 1}`}
                          disabled={isSaving}
                          className="opacity-0 group-hover:opacity-100 transition-opacity text-[var(--text-secondary)] hover:text-[var(--status-error)] hover:bg-[var(--status-error)]/10"
                        >
                          <X className="w-4 h-4" />
                        </Button>
                      </div>
                    ))}
                  </div>
                )}
                {/* Centered dashed-border add button */}
                <button
                  type="button"
                  onClick={handleAddStep}
                  disabled={isSaving}
                  aria-label="Add step"
                  className="mt-4 w-full py-2 px-4 rounded-md border border-dashed border-white/20 hover:border-[#ff6b35]/50 text-[var(--text-muted)] hover:text-[var(--text-primary)] transition-colors flex items-center justify-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  <Plus className="w-4 h-4" />
                  <span className="text-sm">Add another step</span>
                </button>
              </div>
            </div>

            {/* Acceptance Criteria Editor with Glass Container */}
            <div className="space-y-2 animate-stagger-fade-in" style={{ animationDelay: "200ms" }}>
              <Label className="text-[var(--text-primary)]">Acceptance Criteria</Label>
              <div className="rounded-lg border border-white/[0.08] bg-white/[0.03] backdrop-blur-xl p-4">
                {acceptanceCriteria.length === 0 ? (
                  <div className="flex flex-col items-center justify-center py-6 text-center">
                    <CheckCircle className="w-8 h-8 text-[var(--text-muted)] mb-2" />
                    <p className="text-sm text-[var(--text-muted)]">No acceptance criteria defined yet</p>
                    <p className="text-xs text-[var(--text-muted)]/60 mt-1">Add criteria to define success conditions</p>
                  </div>
                ) : (
                  <div className="space-y-2">
                    {acceptanceCriteria.map((criterion, index) => (
                      <div key={index} className="group flex items-center gap-3">
                        <span className="text-[#ff6b35] text-lg font-medium w-6 flex-shrink-0">✓</span>
                        <Input
                          data-testid="criterion-input"
                          type="text"
                          value={criterion}
                          onChange={(e) => handleCriterionChange(index, e.target.value)}
                          aria-label={`Acceptance criterion ${index + 1}`}
                          className={`${inputClasses} flex-1`}
                          disabled={isSaving}
                        />
                        <Button
                          type="button"
                          variant="ghost"
                          size="icon-sm"
                          onClick={() => handleRemoveCriterion(index)}
                          aria-label={`Remove criterion ${index + 1}`}
                          disabled={isSaving}
                          className="opacity-0 group-hover:opacity-100 transition-opacity text-[var(--text-secondary)] hover:text-[var(--status-error)] hover:bg-[var(--status-error)]/10"
                        >
                          <X className="w-4 h-4" />
                        </Button>
                      </div>
                    ))}
                  </div>
                )}
                {/* Centered dashed-border add button */}
                <button
                  type="button"
                  onClick={handleAddCriterion}
                  disabled={isSaving}
                  aria-label="Add criterion"
                  className="mt-4 w-full py-2 px-4 rounded-md border border-dashed border-white/20 hover:border-[#ff6b35]/50 text-[var(--text-muted)] hover:text-[var(--text-primary)] transition-colors flex items-center justify-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  <Plus className="w-4 h-4" />
                  <span className="text-sm">Add acceptance criterion</span>
                </button>
              </div>
            </div>

          </div>
        </ScrollArea>

        <DialogFooter>
          <Button
            data-testid="cancel-button"
            variant="ghost"
            onClick={onCancel}
            disabled={isSaving}
            className="text-[var(--text-primary)] hover:bg-[var(--bg-hover)]"
          >
            Cancel
          </Button>
          <Button
            data-testid="confirm-button"
            onClick={handleSave}
            disabled={!canSave}
            className="bg-[var(--accent-primary)] hover:bg-[var(--accent-primary)]/90 text-white active:scale-[0.98] transition-all hover:-translate-y-px hover:shadow-lg"
          >
            {isSaving && <Loader2 className="w-4 h-4 mr-2 animate-spin" />}
            {isSaving ? "Saving..." : "Save"}
          </Button>
        </DialogFooter>
      </DialogContent>

      {/* Modal entry animations */}
      <style>{`
        @keyframes modal-slide-up {
          from {
            opacity: 0;
            transform: translateY(20px) scale(0.98);
          }
          to {
            opacity: 1;
            transform: translateY(0) scale(1);
          }
        }

        @keyframes stagger-fade-in {
          from {
            opacity: 0;
            transform: translateY(10px);
          }
          to {
            opacity: 1;
            transform: translateY(0);
          }
        }

        .animate-modal-slide-up {
          animation: modal-slide-up 250ms cubic-bezier(0.16, 1, 0.3, 1) forwards;
        }

        .animate-stagger-fade-in {
          opacity: 0;
          animation: stagger-fade-in 200ms cubic-bezier(0.16, 1, 0.3, 1) forwards;
        }

        /* Ambient warm glow at modal corners - Task 8 */
        [data-testid="proposal-edit-modal"]::before {
          content: '';
          position: absolute;
          top: -50px;
          right: -50px;
          width: 200px;
          height: 200px;
          background: radial-gradient(circle, rgba(255, 107, 53, 0.08) 0%, transparent 70%);
          pointer-events: none;
          z-index: -1;
        }

        [data-testid="proposal-edit-modal"]::after {
          content: '';
          position: absolute;
          bottom: -50px;
          left: -50px;
          width: 200px;
          height: 200px;
          background: radial-gradient(circle, rgba(255, 107, 53, 0.05) 0%, transparent 70%);
          pointer-events: none;
          z-index: -1;
        }
      `}</style>
    </Dialog>
  );
}
