/**
 * ReviewNotesModal - Modal for adding review notes with optional fix description
 * Used when approving/rejecting reviews or requesting changes
 */

import { useState } from "react";

interface ReviewNotesModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSubmit: (data: { notes: string; fixDescription?: string }) => void;
  title: string;
  showFixDescription?: boolean;
  notesLabel?: string;
  notesPlaceholder?: string;
  notesRequired?: boolean;
}

export function ReviewNotesModal({
  isOpen,
  onClose,
  onSubmit,
  title,
  showFixDescription = false,
  notesLabel = "Notes",
  notesPlaceholder = "Enter your review notes...",
  notesRequired = false,
}: ReviewNotesModalProps) {
  const [notes, setNotes] = useState("");
  const [fixDescription, setFixDescription] = useState("");

  if (!isOpen) return null;

  const handleSubmit = () => {
    const data: { notes: string; fixDescription?: string } = { notes };
    if (showFixDescription) {
      data.fixDescription = fixDescription;
    }
    onSubmit(data);
    setNotes("");
    setFixDescription("");
  };

  const handleCancel = () => {
    setNotes("");
    setFixDescription("");
    onClose();
  };

  const isSubmitDisabled = notesRequired && notes.trim() === "";

  const btnBase = "px-4 py-2 rounded text-sm font-medium transition-colors";

  return (
    <div data-testid="review-notes-modal" data-has-fix-description={showFixDescription ? "true" : "false"} className="fixed inset-0 z-50 flex items-center justify-center">
      <div data-testid="review-notes-modal-overlay" className="absolute inset-0" style={{ backgroundColor: "rgba(0, 0, 0, 0.5)" }} onClick={handleCancel} />
      <div data-testid="review-notes-modal-content" className="relative w-full max-w-md p-6 rounded-lg shadow-lg" style={{ backgroundColor: "var(--bg-elevated)", borderColor: "var(--border-subtle)" }}>
        <h2 data-testid="modal-title" className="text-lg font-semibold mb-4" style={{ color: "var(--text-primary)" }}>{title}</h2>
        <div className="space-y-4">
          <div>
            <label className="block text-sm font-medium mb-1" style={{ color: "var(--text-secondary)" }}>{notesLabel}</label>
            <textarea data-testid="notes-textarea" value={notes} onChange={(e) => setNotes(e.target.value)} placeholder={notesPlaceholder} rows={3} className="w-full px-3 py-2 rounded border text-sm resize-none" style={{ backgroundColor: "var(--bg-base)", borderColor: "var(--border-subtle)", color: "var(--text-primary)" }} />
          </div>
          {showFixDescription && (
            <div>
              <label className="block text-sm font-medium mb-1" style={{ color: "var(--text-secondary)" }}>Fix Description</label>
              <textarea data-testid="fix-description-textarea" value={fixDescription} onChange={(e) => setFixDescription(e.target.value)} placeholder="Describe what needs to be fixed..." rows={3} className="w-full px-3 py-2 rounded border text-sm resize-none" style={{ backgroundColor: "var(--bg-base)", borderColor: "var(--border-subtle)", color: "var(--text-primary)" }} />
            </div>
          )}
        </div>
        <div className="flex justify-end gap-3 mt-6">
          <button onClick={handleCancel} className={btnBase} style={{ backgroundColor: "var(--bg-hover)", color: "var(--text-primary)" }}>Cancel</button>
          <button onClick={handleSubmit} disabled={isSubmitDisabled} className={btnBase} style={{ backgroundColor: isSubmitDisabled ? "var(--bg-hover)" : "var(--status-success)", color: isSubmitDisabled ? "var(--text-secondary)" : "var(--bg-base)", cursor: isSubmitDisabled ? "not-allowed" : "pointer" }}>Submit</button>
        </div>
      </div>
    </div>
  );
}
