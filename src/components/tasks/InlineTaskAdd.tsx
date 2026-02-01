/**
 * InlineTaskAdd - Quick Add Task widget with macOS Tahoe styling
 *
 * A native-feeling inline task creation widget that appears on column hover.
 * Features frosted glass aesthetics, refined typography, and smooth spring animations.
 *
 * Keyboard shortcuts:
 * - Enter: Create task (from title or description)
 * - Tab: Reveal description field
 * - Shift+Tab: Return to title from description
 * - Shift+Enter: New line in description
 * - Escape: Cancel and collapse
 */

import { useState, useRef, useEffect, useCallback } from "react";
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
  /** Callback to notify parent when expanded state changes */
  onExpandedChange?: (expanded: boolean) => void;
}

export function InlineTaskAdd({ projectId, columnId: _columnId, onCreated, onExpandedChange }: InlineTaskAddProps) {
  const [isExpanded, setIsExpanded] = useState(false);
  const [isHovered, setIsHovered] = useState(false);
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [showDescription, setShowDescription] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const descriptionRef = useRef<HTMLTextAreaElement>(null);
  const openTaskCreation = useUiStore((state) => state.openTaskCreation);

  const { createMutation } = useTaskMutation(projectId);

  const setExpanded = useCallback((expanded: boolean) => {
    setIsExpanded(expanded);
    onExpandedChange?.(expanded);
  }, [onExpandedChange]);

  useEffect(() => {
    if (isExpanded && inputRef.current) {
      inputRef.current.focus();
    }
  }, [isExpanded]);

  useEffect(() => {
    if (showDescription && descriptionRef.current) {
      descriptionRef.current.focus();
    }
  }, [showDescription]);

  const handleExpand = () => {
    setExpanded(true);
  };

  const handleCollapse = () => {
    setExpanded(false);
    setTitle("");
    setDescription("");
    setShowDescription(false);
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
        priority: 3,
        ...(description.trim() && { description: description.trim() }),
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
    } else if (e.key === "Tab" && !e.shiftKey) {
      e.preventDefault();
      setShowDescription(true);
    }
  };

  const handleDescriptionKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleCreate();
    } else if (e.key === "Escape") {
      e.preventDefault();
      handleCollapse();
    } else if (e.key === "Tab" && e.shiftKey) {
      e.preventDefault();
      inputRef.current?.focus();
    }
  };

  const handleMoreOptions = () => {
    openTaskCreation(projectId, title);
    handleCollapse();
  };

  // Collapsed state: Ghost card with refined hover
  if (!isExpanded) {
    return (
      <button
        data-testid="inline-task-add-collapsed"
        onClick={handleExpand}
        onMouseEnter={() => setIsHovered(true)}
        onMouseLeave={() => setIsHovered(false)}
        className="group w-full relative overflow-hidden"
        style={{
          padding: "10px 12px",
          borderRadius: "10px",
          border: `1.5px dashed ${isHovered ? "hsl(14 100% 60%)" : "hsla(220 10% 100% / 0.15)"}`,
          backgroundColor: isHovered ? "hsla(14 100% 60% / 0.04)" : "transparent",
          transition: "all 180ms cubic-bezier(0.4, 0, 0.2, 1)",
          cursor: "pointer",
        }}
      >
        <div
          className="flex items-center gap-2"
          style={{
            color: isHovered ? "hsl(14 100% 60%)" : "hsl(220 10% 45%)",
            transition: "color 180ms ease",
          }}
        >
          <div
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              width: "18px",
              height: "18px",
              borderRadius: "5px",
              backgroundColor: isHovered ? "hsla(14 100% 60% / 0.12)" : "transparent",
              transition: "background-color 180ms ease",
            }}
          >
            <Plus
              style={{
                width: "13px",
                height: "13px",
                strokeWidth: 2.5,
              }}
            />
          </div>
          <span
            style={{
              fontSize: "13px",
              fontWeight: 500,
              letterSpacing: "-0.01em",
            }}
          >
            Add task
          </span>
        </div>
      </button>
    );
  }

  // Expanded state: Flat card with Tahoe styling
  return (
    <div
      data-testid="inline-task-add-expanded"
      className="w-full relative"
      style={{
        padding: "12px",
        borderRadius: "10px",
        backgroundColor: "hsl(220 10% 12%)",
        border: "1px solid hsla(220 10% 100% / 0.06)",
        boxShadow: "0 2px 8px hsla(220 10% 0% / 0.2)",
        animation: "fadeInScale 180ms cubic-bezier(0.34, 1.56, 0.64, 1)",
      }}
    >
      {/* Title input row */}
      <div className="flex items-center gap-2 overflow-hidden">
        <input
          ref={inputRef}
          data-testid="inline-task-add-input"
          type="text"
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          onKeyDown={handleKeyDown}
          onBlur={(e) => {
            const relatedTarget = e.relatedTarget as HTMLElement | null;
            if (relatedTarget?.closest('[data-testid="inline-task-add-expanded"]')) {
              return;
            }
            if (!title.trim() && !description.trim()) {
              handleCollapse();
            }
          }}
          placeholder="Task title"
          disabled={createMutation.isPending}
          className="flex-1 min-w-0 bg-transparent outline-none ring-0 focus:ring-0 focus:outline-none focus-visible:outline-none border-0"
          style={{
            color: "hsl(220 10% 90%)",
            fontSize: "13px",
            fontWeight: 500,
            letterSpacing: "-0.01em",
            boxShadow: "none",
            outline: "none",
          }}
        />

        {/* Tab hint - keyboard badge style */}
        {!showDescription && (
          <div
            className="flex items-center gap-0.5 shrink-0"
            style={{
              padding: "2px 5px",
              borderRadius: "4px",
              backgroundColor: "hsla(220 10% 100% / 0.04)",
              border: "1px solid hsla(220 10% 100% / 0.06)",
            }}
          >
            <span
              style={{
                fontSize: "9px",
                fontWeight: 600,
                color: "hsl(220 10% 40%)",
                letterSpacing: "0.02em",
                textTransform: "uppercase",
              }}
            >
              Tab
            </span>
            <span
              style={{
                fontSize: "9px",
                color: "hsl(220 10% 40%)",
                opacity: 0.5,
              }}
            >
              +desc
            </span>
          </div>
        )}
      </div>

      {/* Description textarea with smooth reveal */}
      <div
        style={{
          display: "grid",
          gridTemplateRows: showDescription ? "1fr" : "0fr",
          transition: "grid-template-rows 200ms cubic-bezier(0.4, 0, 0.2, 1)",
        }}
      >
        <div style={{ overflow: "hidden" }}>
          <textarea
            ref={descriptionRef}
            data-testid="inline-task-add-description"
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            onKeyDown={handleDescriptionKeyDown}
            onBlur={(e) => {
              const relatedTarget = e.relatedTarget as HTMLElement | null;
              if (relatedTarget?.closest('[data-testid="inline-task-add-expanded"]')) {
                return;
              }
              if (!title.trim() && !description.trim()) {
                handleCollapse();
              }
            }}
            placeholder="Add a description..."
            disabled={createMutation.isPending}
            rows={2}
            className="w-full bg-transparent outline-none ring-0 focus:ring-0 focus:outline-none focus-visible:outline-none border-0 resize-none"
            style={{
              color: "hsl(220 10% 65%)",
              fontSize: "12px",
              lineHeight: 1.5,
              letterSpacing: "-0.006em",
              marginTop: "8px",
              paddingTop: "8px",
              borderTop: "1px solid hsla(220 10% 100% / 0.06)",
              boxShadow: "none",
              outline: "none",
            }}
          />
        </div>
      </div>

      {/* Action row */}
      <div
        className="flex items-center justify-between gap-3"
        style={{
          marginTop: "10px",
          paddingTop: "10px",
          borderTop: "1px solid hsla(220 10% 100% / 0.04)",
        }}
      >
        {/* Left: More options */}
        <button
          data-testid="inline-task-add-more-options"
          onClick={handleMoreOptions}
          onMouseDown={(e) => e.preventDefault()}
          disabled={createMutation.isPending}
          className="group/btn flex items-center shrink-0"
          style={{
            padding: "4px 6px",
            marginLeft: "-6px",
            borderRadius: "4px",
            backgroundColor: "transparent",
            border: "none",
            cursor: "pointer",
            transition: "background-color 150ms ease",
            whiteSpace: "nowrap",
          }}
          onMouseEnter={(e) => {
            e.currentTarget.style.backgroundColor = "hsla(220 10% 100% / 0.05)";
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.backgroundColor = "transparent";
          }}
        >
          <span
            style={{
              fontSize: "11px",
              fontWeight: 500,
              color: "hsl(14 100% 60%)",
              letterSpacing: "-0.006em",
            }}
          >
            More
          </span>
        </button>

        {/* Right: Actions */}
        <div className="flex items-center gap-1.5">
          {/* Cancel */}
          <button
            data-testid="inline-task-add-cancel"
            onClick={handleCollapse}
            onMouseDown={(e) => e.preventDefault()}
            disabled={createMutation.isPending}
            style={{
              padding: "4px 8px",
              borderRadius: "5px",
              backgroundColor: "transparent",
              border: "none",
              fontSize: "11px",
              fontWeight: 500,
              color: "hsl(220 10% 50%)",
              cursor: "pointer",
              transition: "all 150ms ease",
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.backgroundColor = "hsla(220 10% 100% / 0.05)";
              e.currentTarget.style.color = "hsl(220 10% 70%)";
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.backgroundColor = "transparent";
              e.currentTarget.style.color = "hsl(220 10% 50%)";
            }}
          >
            Cancel
          </button>

          {/* Create button */}
          <button
            onClick={handleCreate}
            onMouseDown={(e) => e.preventDefault()}
            disabled={createMutation.isPending || !title.trim()}
            style={{
              padding: "4px 10px",
              borderRadius: "5px",
              backgroundColor: title.trim() ? "hsl(14 100% 60%)" : "hsla(220 10% 100% / 0.06)",
              border: "none",
              fontSize: "11px",
              fontWeight: 600,
              color: title.trim() ? "white" : "hsl(220 10% 40%)",
              cursor: title.trim() ? "pointer" : "default",
              transition: "all 150ms ease",
              opacity: createMutation.isPending ? 0.6 : 1,
            }}
            onMouseEnter={(e) => {
              if (title.trim() && !createMutation.isPending) {
                e.currentTarget.style.backgroundColor = "hsl(14 100% 55%)";
              }
            }}
            onMouseLeave={(e) => {
              if (title.trim()) {
                e.currentTarget.style.backgroundColor = "hsl(14 100% 60%)";
              }
            }}
          >
            <span className="flex items-center gap-1">
              {createMutation.isPending ? (
                "..."
              ) : (
                <>
                  Create
                  <kbd
                    style={{
                      display: "inline-flex",
                      alignItems: "center",
                      padding: "1px 3px",
                      borderRadius: "2px",
                      backgroundColor: title.trim() ? "hsla(0 0% 100% / 0.15)" : "hsla(220 10% 100% / 0.08)",
                      fontSize: "9px",
                      fontWeight: 500,
                      opacity: 0.7,
                    }}
                  >
                    ↵
                  </kbd>
                </>
              )}
            </span>
          </button>
        </div>
      </div>

      {/* Inline CSS animation */}
      <style>{`
        @keyframes fadeInScale {
          from {
            opacity: 0;
            transform: scale(0.96) translateY(-2px);
          }
          to {
            opacity: 1;
            transform: scale(1) translateY(0);
          }
        }
      `}</style>
    </div>
  );
}
