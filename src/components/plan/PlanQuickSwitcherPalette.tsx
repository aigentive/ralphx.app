/**
 * PlanQuickSwitcherPalette - Non-modal command palette for quick plan switching
 *
 * Features:
 * - Fixed positioned panel (top: 80px, centered)
 * - Glass morphism styling
 * - Keyboard-first interaction (up/down/enter/escape)
 * - Auto-focus search input on open
 * - Click outside to close
 * - No backdrop/modal overlay
 * - Uses framer-motion for enter/exit animations
 *
 * Refactored to use usePlanQuickSwitcher hook and extracted sub-components.
 * Now ~200 lines (thin rendering shell).
 */

/* eslint-disable react-hooks/refs */
// Disabled: ESLint incorrectly flags passing ref objects from hook as "accessing during render"
// We're not accessing .current, just passing ref objects to ref={} props which is valid React.

import { AnimatePresence, motion } from "framer-motion";
import { FileText, Loader2, AlertCircle, RefreshCw } from "lucide-react";
import { cn } from "@/lib/utils";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Button } from "@/components/ui/button";
import { usePlanQuickSwitcher } from "@/hooks/usePlanQuickSwitcher";
import { PlanCandidateItem } from "./PlanCandidateItem";
import { PlanClearAction } from "./PlanClearAction";
import type { SelectionSource } from "@/api/plan";

// ============================================================================
// Types
// ============================================================================

interface PlanQuickSwitcherPaletteProps {
  projectId: string;
  isOpen: boolean;
  onClose: () => void;
  /** Source attribution for selection analytics */
  selectionSource?: SelectionSource;
  /** Show clear active plan command at top of list when active plan exists */
  showClearAction?: boolean;
  /** Optional CSS selector used to anchor horizontal centering to a specific container */
  anchorSelector?: string;
}

// ============================================================================
// Component
// ============================================================================

export function PlanQuickSwitcherPalette(props: PlanQuickSwitcherPaletteProps) {
  const switcher = usePlanQuickSwitcher(props);

  const hasItems = switcher.filteredCandidates.length > 0 || switcher.canClearPlan;

  return (
    <AnimatePresence>
      {props.isOpen && (
        <motion.div
          initial="hidden"
          animate="visible"
          exit="exit"
          variants={{
            hidden: { opacity: 0, y: -20 },
            visible: {
              opacity: 1,
              y: 0,
              transition: {
                opacity: { duration: 0.15 },
                y: {
                  type: "spring",
                  stiffness: 400,
                  damping: 30,
                },
              },
            },
            exit: {
              opacity: 0,
              y: -20,
              transition: { duration: 0.1 },
            },
          }}
          className="fixed top-20 left-1/2 -translate-x-1/2 z-50 w-[420px]"
          ref={switcher.containerRef}
          data-quick-switcher-panel
          style={{
            left: switcher.anchorCenterX !== null ? `${switcher.anchorCenterX}px` : undefined,
          }}
        >
          <div
            className="text-popover-foreground"
            onClick={(e) => e.stopPropagation()}
            style={{
              borderRadius: "10px",
              background: "hsla(220 10% 10% / 0.92)",
              backdropFilter: "blur(20px) saturate(180%)",
              WebkitBackdropFilter: "blur(20px) saturate(180%)",
              border: "1px solid hsla(220 20% 100% / 0.08)",
              boxShadow: "0 4px 16px hsla(220 20% 0% / 0.4), 0 12px 32px hsla(220 20% 0% / 0.3)",
            }}
          >
            {/* Search input */}
            <div
              className="p-3"
              style={{ borderBottom: "1px solid hsla(220 20% 100% / 0.08)" }}
            >
              <input
                ref={switcher.inputRef}
                type="text"
                placeholder="Search plans... (Cmd+Shift+P)"
                value={switcher.searchQuery}
                onChange={(e) => switcher.setSearchQuery(e.target.value)}
                onKeyDown={switcher.handleKeyDown}
                className={cn(
                  "w-full bg-transparent border-0 text-sm placeholder:text-muted-foreground",
                  "outline-none ring-0 focus:ring-0 focus:outline-none focus-visible:outline-none",
                  "transition-colors"
                )}
                style={{
                  color: "hsl(220 10% 90%)",
                  boxShadow: "none",
                  outline: "none",
                }}
              />
            </div>

            {/* Content area */}
            {switcher.error ? (
              // Error state
              <div className="p-8 flex flex-col items-center justify-center gap-3 text-muted-foreground">
                <AlertCircle className="h-5 w-5 text-destructive" />
                <p className="text-center">{switcher.error}</p>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={switcher.handleRetry}
                  className="gap-2"
                >
                  <RefreshCw className="h-4 w-4" />
                  Retry
                </Button>
              </div>
            ) : switcher.isLoading ? (
              // Loading state
              <div className="p-8 flex flex-col items-center justify-center gap-2 text-muted-foreground transition-colors">
                <Loader2 className="h-5 w-5 animate-spin" />
                <p>Loading plans...</p>
              </div>
            ) : hasItems ? (
              // Results list
              <ScrollArea className="max-h-[400px]" type="auto">
                {switcher.canClearPlan && (
                  <PlanClearAction
                    isHighlighted={switcher.highlightedIndex === 0}
                    onMouseEnter={() => switcher.setHighlightedIndex(0)}
                    onClick={switcher.handleClear}
                    highlightedRef={switcher.highlightedItemRef}
                  />
                )}
                {switcher.filteredCandidates.map((plan, index) => {
                  const itemIndex = switcher.canClearPlan ? index + 1 : index;
                  const isActive = switcher.activePlanId === plan.sessionId;
                  const isHighlighted = switcher.highlightedIndex === itemIndex;

                  return (
                    <PlanCandidateItem
                      key={plan.sessionId}
                      plan={plan}
                      isActive={isActive}
                      isHighlighted={isHighlighted}
                      onMouseEnter={() => switcher.setHighlightedIndex(itemIndex)}
                      onClick={() => switcher.handleSelect(plan.sessionId)}
                      highlightedRef={switcher.highlightedItemRef}
                    />
                  );
                })}
              </ScrollArea>
            ) : (
              // Empty state
              <div className="p-8 text-center text-muted-foreground transition-colors">
                <FileText className="h-8 w-8 mx-auto mb-2 opacity-50 transition-opacity" />
                <p>No accepted plans found</p>
              </div>
            )}
          </div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
