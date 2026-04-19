/**
 * CollapsedQuickAdd - "+" button with Popover for adding tasks from a collapsed column
 *
 * Renders a small dashed-border plus button in collapsed draft/backlog columns.
 * Click opens a Radix Popover (side=right) containing InlineTaskAdd.
 * e.stopPropagation() prevents triggering column expand.
 * On task creation, Popover closes (column auto-expands via count reactivity).
 */

import { useState, useCallback } from "react";
import { Plus } from "lucide-react";
import {
  Popover,
  PopoverTrigger,
  PopoverContent,
} from "@/components/ui/popover";
import { InlineTaskAdd } from "../InlineTaskAdd";

interface CollapsedQuickAddProps {
  projectId: string;
  columnId: string;
}

export function CollapsedQuickAdd({ projectId, columnId }: CollapsedQuickAddProps) {
  const [open, setOpen] = useState(false);

  const handleCreated = useCallback(() => {
    setOpen(false);
  }, []);

  const handleTriggerClick = useCallback((e: React.MouseEvent) => {
    e.stopPropagation();
  }, []);

  const handleTriggerKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === "Enter" || e.key === " ") {
      e.stopPropagation();
    }
  }, []);

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <button
          data-testid="collapsed-quick-add"
          aria-label="Add task"
          onClick={handleTriggerClick}
          onKeyDown={handleTriggerKeyDown}
          className="collapsed-quick-add-btn"
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            width: "28px",
            height: "28px",
            marginTop: "8px",
            borderRadius: "6px",
            border: "1.5px dashed var(--overlay-moderate)",
            backgroundColor: "transparent",
            cursor: "pointer",
            transition: "all 180ms cubic-bezier(0.4, 0, 0.2, 1)",
            padding: 0,
          }}
        >
          <Plus
            className="collapsed-quick-add-icon"
            style={{
              width: "14px",
              height: "14px",
              strokeWidth: 2.5,
              color: "var(--text-muted)",
              transition: "color 180ms ease",
            }}
          />
        </button>
      </PopoverTrigger>
      <PopoverContent
        side="right"
        sideOffset={8}
        className="p-0 border-0 bg-transparent shadow-none"
        style={{ width: "280px" }}
        onClick={(e) => e.stopPropagation()}
        onKeyDown={(e) => e.stopPropagation()}
      >
        <InlineTaskAdd
          projectId={projectId}
          columnId={columnId}
          onCreated={handleCreated}
        />
      </PopoverContent>

      {/* Hover styles for the trigger button */}
      <style>{`
        .collapsed-quick-add-btn:hover {
          border-color: var(--accent-primary) !important;
          background-color: color-mix(in srgb, var(--accent-primary) 4%, transparent) !important;
        }
        .collapsed-quick-add-btn:hover .collapsed-quick-add-icon {
          color: var(--accent-primary) !important;
        }
      `}</style>
    </Popover>
  );
}
