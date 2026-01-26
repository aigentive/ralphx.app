/**
 * InlineTaskAdd - Ghost card for quick task creation on column hover
 *
 * Shows a dashed-border "ghost card" when hovering over draft or backlog columns.
 * Clicking expands to an inline form for fast task creation.
 *
 * Design spec: specs/plans/task-crud-archive-search.md - Part 3: Inline Quick-Add
 * - Collapsed: dashed border, muted text, "+ Add task"
 * - Expanded: input field with auto-focus, Enter to create, Escape to cancel
 * - "More options" link opens full TaskCreationForm modal
 */

import { useState, useRef, useEffect } from "react";
import { Plus } from "lucide-react";
import { useTaskMutation } from "@/hooks/useTaskMutation";
import { useUiStore } from "@/stores/uiStore";
import type { Task } from "@/types/task";

interface InlineTaskAddProps {
  /** Project ID for task creation */
  projectId: string;
  /** Column ID for default status (draft or backlog) */
  columnId: string;
  /** Optional callback when task is created */
  onCreated?: (task: Task) => void;
}

/**
 * InlineTaskAdd Component
 *
 * Two states:
 * 1. Collapsed (ghost card): dashed border, "+ Add task" text
 * 2. Expanded (form): input field with Enter to create, Escape to cancel, "More options" link
 */
export function InlineTaskAdd({ projectId, columnId, onCreated }: InlineTaskAddProps) {
  const [isExpanded, setIsExpanded] = useState(false);
  const [title, setTitle] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);
  const openModal = useUiStore((state) => state.openModal);

  const { createMutation } = useTaskMutation(projectId);

  // Auto-focus input when expanded
  useEffect(() => {
    if (isExpanded && inputRef.current) {
      inputRef.current.focus();
    }
  }, [isExpanded]);

  const handleExpand = () => {
    setIsExpanded(true);
  };

  const handleCollapse = () => {
    setIsExpanded(false);
    setTitle("");
  };

  const handleCreate = () => {
    if (!title.trim()) {
      handleCollapse();
      return;
    }

    createMutation.mutate(
      {
        projectId,
        title: title.trim(),
        category: "feature",
        priority: 3, // Medium priority
      },
      {
        onSuccess: (createdTask) => {
          handleCollapse();
          onCreated?.(createdTask);
        },
      }
    );
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") {
      e.preventDefault();
      handleCreate();
    } else if (e.key === "Escape") {
      e.preventDefault();
      handleCollapse();
    }
  };

  const handleMoreOptions = () => {
    openModal("task-create", {
      projectId,
      defaultTitle: title,
      defaultStatus: columnId,
    });
    handleCollapse();
  };

  // Collapsed state: ghost card with dashed border
  if (!isExpanded) {
    return (
      <button
        data-testid="inline-task-add-collapsed"
        onClick={handleExpand}
        className="w-full p-3 border-2 border-dashed rounded-lg transition-colors hover:border-opacity-30"
        style={{
          borderColor: "var(--border-subtle)",
          backgroundColor: "transparent",
        }}
        onMouseEnter={(e) => {
          e.currentTarget.style.borderColor = "var(--accent-primary)";
        }}
        onMouseLeave={(e) => {
          e.currentTarget.style.borderColor = "var(--border-subtle)";
        }}
      >
        <div className="flex items-center gap-2" style={{ color: "var(--text-muted)" }}>
          <Plus className="w-4 h-4" />
          <span className="text-sm">Add task</span>
        </div>
      </button>
    );
  }

  // Expanded state: inline form with input
  return (
    <div
      data-testid="inline-task-add-expanded"
      className="w-full p-3 rounded-lg"
      style={{
        backgroundColor: "var(--bg-elevated)",
        border: "1px solid var(--border-default)",
      }}
    >
      <input
        ref={inputRef}
        data-testid="inline-task-add-input"
        type="text"
        value={title}
        onChange={(e) => setTitle(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder="Task title..."
        disabled={createMutation.isPending}
        className="w-full text-sm bg-transparent outline-none"
        style={{
          color: "var(--text-primary)",
          border: "none",
        }}
      />
      <div className="flex items-center justify-between mt-2 text-xs">
        <button
          data-testid="inline-task-add-more-options"
          onClick={handleMoreOptions}
          disabled={createMutation.isPending}
          className="hover:underline"
          style={{ color: "var(--text-muted)" }}
        >
          More options
        </button>
        <button
          data-testid="inline-task-add-cancel"
          onClick={handleCollapse}
          disabled={createMutation.isPending}
          className="hover:underline"
          style={{ color: "var(--text-muted)" }}
        >
          Cancel
        </button>
      </div>
    </div>
  );
}
