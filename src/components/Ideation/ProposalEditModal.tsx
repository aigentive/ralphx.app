/**
 * ProposalEditModal - Modal for editing task proposal details
 * Allows editing title, description, category, steps, acceptance criteria,
 * priority override, and complexity.
 *
 * Uses shadcn/ui Dialog, Input, Textarea, Select, Button, Label components.
 */

import { useState, useCallback, useEffect, useRef } from "react";
import { Edit3, Plus, X, Loader2 } from "lucide-react";
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

  const inputClasses = "bg-[var(--bg-base)] border-[var(--border-subtle)] text-[var(--text-primary)] placeholder:text-[var(--text-muted)] focus:ring-[var(--accent-primary)] focus:border-[var(--accent-primary)]";
  const selectClasses = "w-full h-9 rounded-md border px-3 py-2 text-sm bg-[var(--bg-base)] border-[var(--border-subtle)] text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent-primary)] focus:border-transparent";

  return (
    <Dialog open={!!proposal} onOpenChange={handleOpenChange}>
      <DialogContent
        data-testid="proposal-edit-modal"
        className="max-w-2xl max-h-[90vh]"
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
            <div className="space-y-2">
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
            <div className="space-y-2">
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
            <div className="rounded-lg border border-white/[0.08] bg-white/[0.03] backdrop-blur-xl p-4">
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

                {/* Right Column: Complexity (placeholder for visual selector in Task 3) */}
                <div className="space-y-2">
                  <Label htmlFor="proposal-complexity" className="text-[var(--text-primary)] text-sm">
                    Complexity
                  </Label>
                  <select
                    id="proposal-complexity"
                    value={complexity}
                    onChange={(e) => setComplexity(e.target.value as Complexity)}
                    className={selectClasses}
                    disabled={isSaving}
                  >
                    {COMPLEXITIES.map((c) => (
                      <option key={c.value} value={c.value}>
                        {c.label}
                      </option>
                    ))}
                  </select>
                </div>
              </div>
            </div>

            {/* Steps Editor */}
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <Label className="text-[var(--text-primary)]">Steps</Label>
                <Button
                  type="button"
                  variant="ghost"
                  size="icon-sm"
                  onClick={handleAddStep}
                  aria-label="Add step"
                  disabled={isSaving}
                  className="text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-hover)]"
                >
                  <Plus className="w-4 h-4" />
                </Button>
              </div>
              {steps.length === 0 ? (
                <p className="text-sm italic text-[var(--text-muted)]">No steps added</p>
              ) : (
                <div className="space-y-2">
                  {steps.map((step, index) => (
                    <div key={index} className="flex items-center gap-2">
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
                        className="text-[var(--text-secondary)] hover:text-[var(--status-error)] hover:bg-[var(--status-error)]/10"
                      >
                        <X className="w-4 h-4" />
                      </Button>
                    </div>
                  ))}
                </div>
              )}
            </div>

            {/* Acceptance Criteria Editor */}
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <Label className="text-[var(--text-primary)]">Acceptance Criteria</Label>
                <Button
                  type="button"
                  variant="ghost"
                  size="icon-sm"
                  onClick={handleAddCriterion}
                  aria-label="Add criterion"
                  disabled={isSaving}
                  className="text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-hover)]"
                >
                  <Plus className="w-4 h-4" />
                </Button>
              </div>
              {acceptanceCriteria.length === 0 ? (
                <p className="text-sm italic text-[var(--text-muted)]">No acceptance criteria added</p>
              ) : (
                <div className="space-y-2">
                  {acceptanceCriteria.map((criterion, index) => (
                    <div key={index} className="flex items-center gap-2">
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
                        className="text-[var(--text-secondary)] hover:text-[var(--status-error)] hover:bg-[var(--status-error)]/10"
                      >
                        <X className="w-4 h-4" />
                      </Button>
                    </div>
                  ))}
                </div>
              )}
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
            className="bg-[var(--accent-primary)] hover:bg-[var(--accent-primary)]/90 text-white active:scale-[0.98] transition-all"
          >
            {isSaving && <Loader2 className="w-4 h-4 mr-2 animate-spin" />}
            {isSaving ? "Saving..." : "Save"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
