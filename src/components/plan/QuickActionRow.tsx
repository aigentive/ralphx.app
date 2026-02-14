/**
 * QuickActionRow - Renders quick action with multi-state flow
 *
 * Supports four states:
 * - idle: Button row with icon + label + query description (animated enter/exit)
 * - confirming: Inline confirmation prompt with Confirm/Cancel buttons
 * - creating: Spinner + creatingLabel (all controls disabled)
 * - success: Check icon + successLabel + view button
 *
 * The confirming/creating/success states replace the candidate list using
 * AnimatePresence mode="wait" for crossfade transitions.
 */

import { motion, AnimatePresence } from "framer-motion";
import { Check, Loader2, type LucideIcon } from "lucide-react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";

// ============================================================================
// Types
// ============================================================================

export type QuickActionFlowState = "idle" | "confirming" | "creating" | "success";

export interface QuickAction {
  /** Unique identifier for the action (e.g., "ideation", "create-task") */
  id: string;
  /** Display label (e.g., "Start new ideation session") */
  label: string;
  /** Icon component from lucide-react */
  icon: LucideIcon;
  /** Generate description text based on current query */
  description: (query: string) => string;
  /** Determine if action should be visible for given query */
  isVisible: (query: string) => boolean;
  /** Execute the action. Returns entity ID on success. */
  execute: (query: string) => Promise<string>;
  /** Label shown during creation */
  creatingLabel: string;
  /** Label shown on success */
  successLabel: string;
  /** Button text on success */
  viewLabel: string;
  /** Navigate to the created entity */
  navigateTo: (entityId: string) => void;
}

interface QuickActionRowProps {
  /** The quick action definition */
  action: QuickAction;
  /** Current flow state */
  flowState: QuickActionFlowState;
  /** Current search query */
  searchQuery: string;
  /** Whether this row is currently highlighted */
  isHighlighted: boolean;
  /** Mouse enter handler for highlight tracking */
  onMouseEnter: () => void;
  /** Called when user selects this action (idle state) */
  onSelect: () => void;
  /** Called when user confirms the action (confirming state) */
  onConfirm: () => void;
  /** Called when user cancels confirmation (confirming state) */
  onCancel: () => void;
  /** Called when user clicks view entity button (success state) */
  onViewEntity: () => void;
  /** Ref for highlighted item (for scroll-into-view) */
  highlightedRef: React.RefObject<HTMLButtonElement> | null;
}

// ============================================================================
// Component
// ============================================================================

export function QuickActionRow({
  action,
  flowState,
  searchQuery,
  isHighlighted,
  onMouseEnter,
  onSelect,
  onConfirm,
  onCancel,
  onViewEntity,
  highlightedRef,
}: QuickActionRowProps) {
  const Icon = action.icon;

  // Idle state: button row with animated enter/exit
  if (flowState === "idle") {
    return (
      <motion.div
        initial={{ height: 0, opacity: 0 }}
        animate={{ height: "auto", opacity: 1 }}
        exit={{ height: 0, opacity: 0 }}
        transition={{ duration: 0.2 }}
        data-testid="quick-action-idle"
      >
        <button
          ref={isHighlighted ? highlightedRef : null}
          onClick={onSelect}
          onMouseEnter={onMouseEnter}
          className={cn(
            "w-full text-left px-3 py-2 rounded-lg flex items-center gap-3",
            "transition-all duration-150 origin-center",
            "hover:scale-[1.01]",
            "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
          )}
          style={{
            background: isHighlighted
              ? "hsla(14 100% 60% / 0.16)"
              : "transparent",
            border: isHighlighted
              ? "1px solid hsla(14 100% 60% / 0.35)"
              : "1px solid transparent",
          }}
        >
          <Icon
            className="h-4 w-4 shrink-0"
            style={{ color: isHighlighted ? "hsl(14 100% 66%)" : "hsl(220 10% 62%)" }}
          />
          <div className="flex-1 min-w-0">
            <div
              className="text-[13px] font-medium leading-tight"
              style={{ color: isHighlighted ? "hsl(14 100% 66%)" : "hsl(220 10% 90%)" }}
            >
              {action.label}
            </div>
            <div
              className="text-xs leading-tight mt-0.5"
              style={{ color: "hsl(220 10% 62%)" }}
            >
              {action.description(searchQuery)}
            </div>
          </div>
        </button>
      </motion.div>
    );
  }

  // Confirming/creating/success states: replace candidate list with crossfade
  return (
    <div data-testid="quick-action-content">
      <AnimatePresence mode="wait">
        {flowState === "confirming" && (
          <motion.div
            key="confirming"
            initial={{ opacity: 0, y: -10 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: 10 }}
            transition={{ duration: 0.15 }}
            className="p-4 flex flex-col gap-3"
          >
            <p className="text-sm" style={{ color: "hsl(220 10% 90%)" }}>
              Start <span className="font-medium">{action.label.toLowerCase()}</span> with:{" "}
              <span className="font-medium" style={{ color: "hsl(14 100% 66%)" }}>
                &quot;{searchQuery}&quot;
              </span>
              ?
            </p>
            <div className="flex gap-2">
              <Button
                size="sm"
                onClick={onConfirm}
                className="flex-1"
                style={{
                  backgroundColor: "hsl(14 100% 60%)",
                  color: "hsl(220 10% 10%)",
                }}
              >
                Confirm
              </Button>
              <Button
                size="sm"
                variant="outline"
                onClick={onCancel}
                className="flex-1"
              >
                Cancel
              </Button>
            </div>
          </motion.div>
        )}

        {flowState === "creating" && (
          <motion.div
            key="creating"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.15 }}
            className="p-8 flex flex-col items-center justify-center gap-3"
          >
            <Loader2
              className="h-5 w-5 animate-spin"
              style={{ color: "hsl(14 100% 60%)" }}
            />
            <p className="text-sm" style={{ color: "hsl(220 10% 90%)" }}>
              {action.creatingLabel}
            </p>
          </motion.div>
        )}

        {flowState === "success" && (
          <motion.div
            key="success"
            initial={{ opacity: 0, scale: 0.95 }}
            animate={{ opacity: 1, scale: 1 }}
            exit={{ opacity: 0, scale: 0.95 }}
            transition={{ duration: 0.2 }}
            className="p-4 flex flex-col items-center gap-3"
          >
            <div className="flex items-center gap-2">
              <Check
                className="h-5 w-5"
                style={{ color: "hsl(14 100% 60%)" }}
              />
              <p className="text-sm font-medium" style={{ color: "hsl(220 10% 90%)" }}>
                {action.successLabel}
              </p>
            </div>
            <Button
              size="sm"
              onClick={onViewEntity}
              className="w-full"
              style={{
                backgroundColor: "hsl(14 100% 60%)",
                color: "hsl(220 10% 10%)",
              }}
            >
              {action.viewLabel}
            </Button>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
