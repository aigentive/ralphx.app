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
 */

import { useState, useEffect, useRef, useCallback } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { FileText, Check, Loader2, AlertCircle, RefreshCw } from "lucide-react";
import { usePlanStore } from "@/stores/planStore";
import { cn } from "@/lib/utils";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Button } from "@/components/ui/button";

// ============================================================================
// Types
// ============================================================================

interface PlanQuickSwitcherPaletteProps {
  projectId: string;
  isOpen: boolean;
  onClose: () => void;
  /** Optional CSS selector used to anchor horizontal centering to a specific container */
  anchorSelector?: string;
}

function formatIncompleteSummary(incomplete: number, total: number): string {
  if (total <= 0) return "No tasks yet";
  if (incomplete <= 0) {
    return total === 1 ? "1 task complete" : `${total} tasks complete`;
  }
  return `${incomplete} of ${total} incomplete`;
}

function getCompletionPercent(incomplete: number, total: number): number {
  if (total <= 0) return 0;
  const completed = Math.max(0, total - Math.max(0, incomplete));
  return Math.round((completed / total) * 100);
}

// ============================================================================
// Component
// ============================================================================

export function PlanQuickSwitcherPalette({
  projectId,
  isOpen,
  onClose,
  anchorSelector,
}: PlanQuickSwitcherPaletteProps) {
  const [searchQuery, setSearchQuery] = useState("");
  const [highlightedIndex, setHighlightedIndex] = useState(0);
  const [anchorCenterX, setAnchorCenterX] = useState<number | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const highlightedItemRef = useRef<HTMLButtonElement>(null);

  // Store state
  const activePlanId = usePlanStore((state) => state.activePlanByProject[projectId] ?? null);
  const planCandidates = usePlanStore((state) => state.planCandidates);
  const isLoading = usePlanStore((state) => state.isLoading);
  const error = usePlanStore((state) => state.error);
  const loadCandidates = usePlanStore((state) => state.loadCandidates);
  const setActivePlan = usePlanStore((state) => state.setActivePlan);

  // Filter candidates by search query (case-insensitive)
  const filteredCandidates = searchQuery
    ? planCandidates.filter((plan) =>
        (plan.title || "Untitled Plan").toLowerCase().includes(searchQuery.toLowerCase())
      )
    : planCandidates;

  // Auto-focus search input when opened
  useEffect(() => {
    if (isOpen && inputRef.current) {
      inputRef.current.focus();
    }
  }, [isOpen]);

  // Load candidates when opened
  useEffect(() => {
    if (isOpen) {
      loadCandidates(projectId);
    }
  }, [isOpen, projectId, loadCandidates]);

  // Reset state when closed
  useEffect(() => {
    if (!isOpen) {
      setSearchQuery("");
      setHighlightedIndex(0);
    }
  }, [isOpen]);

  // Reset highlighted index when filtered list changes
  useEffect(() => {
    setHighlightedIndex(0);
  }, [searchQuery]);

  // Scroll highlighted item into view
  useEffect(() => {
    if (
      highlightedItemRef.current &&
      typeof highlightedItemRef.current.scrollIntoView === "function"
    ) {
      highlightedItemRef.current.scrollIntoView({
        block: "nearest",
        behavior: "smooth",
      });
    }
  }, [highlightedIndex]);

  // Center to the requested anchor container (e.g., split-layout left pane).
  useEffect(() => {
    if (!isOpen) return;

    const updateAnchorCenter = () => {
      if (!anchorSelector) {
        setAnchorCenterX(null);
        return;
      }
      const anchor = document.querySelector(anchorSelector);
      if (anchor instanceof HTMLElement) {
        const rect = anchor.getBoundingClientRect();
        setAnchorCenterX(rect.left + rect.width / 2);
      } else {
        setAnchorCenterX(null);
      }
    };

    updateAnchorCenter();

    const anchor = anchorSelector ? document.querySelector(anchorSelector) : null;
    const resizeObserver =
      anchor instanceof HTMLElement && typeof ResizeObserver !== "undefined"
        ? new ResizeObserver(() => updateAnchorCenter())
        : null;

    if (anchor instanceof HTMLElement && resizeObserver) {
      resizeObserver.observe(anchor);
    }

    window.addEventListener("resize", updateAnchorCenter);
    window.addEventListener("scroll", updateAnchorCenter, true);

    return () => {
      resizeObserver?.disconnect();
      window.removeEventListener("resize", updateAnchorCenter);
      window.removeEventListener("scroll", updateAnchorCenter, true);
    };
  }, [isOpen, anchorSelector]);

  // Click outside to close
  useEffect(() => {
    if (!isOpen) return;

    const handleMouseDown = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        onClose();
      }
    };

    document.addEventListener("mousedown", handleMouseDown);
    return () => document.removeEventListener("mousedown", handleMouseDown);
  }, [isOpen, onClose]);

  // Handle plan selection
  const handleSelect = useCallback(
    async (sessionId: string) => {
      try {
        await setActivePlan(projectId, sessionId, "quick_switcher");
        onClose();
      } catch (error) {
        console.error("Failed to set active plan:", error);
      }
    },
    [projectId, setActivePlan, onClose]
  );

  // Handle retry
  const handleRetry = useCallback(() => {
    loadCandidates(projectId);
  }, [projectId, loadCandidates]);

  // Keyboard navigation handler
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      const candidateCount = filteredCandidates.length;

      // Prevent navigation if no candidates
      if (candidateCount === 0 && ["ArrowDown", "ArrowUp", "Home", "End"].includes(e.key)) {
        return;
      }

      switch (e.key) {
        case "ArrowDown":
          e.preventDefault();
          if (e.shiftKey) {
            setHighlightedIndex(candidateCount - 1);
          } else {
            setHighlightedIndex((i) => Math.min(i + 1, candidateCount - 1));
          }
          break;
        case "ArrowUp":
          e.preventDefault();
          if (e.shiftKey) {
            setHighlightedIndex(0);
          } else {
            setHighlightedIndex((i) => Math.max(i - 1, 0));
          }
          break;
        case "Home":
          e.preventDefault();
          setHighlightedIndex(0);
          break;
        case "End":
          e.preventDefault();
          setHighlightedIndex(candidateCount - 1);
          break;
        case "Enter":
          e.preventDefault();
          if (highlightedIndex >= 0 && filteredCandidates[highlightedIndex]) {
            handleSelect(filteredCandidates[highlightedIndex].sessionId);
          }
          break;
        case "Escape":
          e.preventDefault();
          onClose();
          break;
      }
    },
    [filteredCandidates, highlightedIndex, onClose, handleSelect]
  );


  return (
    <AnimatePresence>
      {isOpen && (
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
          ref={containerRef}
          data-quick-switcher-panel
          style={{
            left: anchorCenterX !== null ? `${anchorCenterX}px` : undefined,
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
                ref={inputRef}
                type="text"
                placeholder="Search plans... (Cmd+Shift+P)"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                onKeyDown={handleKeyDown}
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

            {/* Results list */}
            {error ? (
              <div className="p-8 flex flex-col items-center justify-center gap-3 text-muted-foreground">
                <AlertCircle className="h-5 w-5 text-destructive" />
                <p className="text-center">{error}</p>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleRetry}
                  className="gap-2"
                >
                  <RefreshCw className="h-4 w-4" />
                  Retry
                </Button>
              </div>
            ) : isLoading ? (
              <div className="p-8 flex flex-col items-center justify-center gap-2 text-muted-foreground transition-colors">
                <Loader2 className="h-5 w-5 animate-spin" />
                <p>Loading plans...</p>
              </div>
            ) : filteredCandidates.length > 0 ? (
              <ScrollArea className="max-h-[400px]">
                {filteredCandidates.map((plan, index) => {
                  const isActive = activePlanId === plan.sessionId;
                  const isHighlighted = highlightedIndex === index;
                  const completionPercent = getCompletionPercent(
                    plan.taskStats.incomplete,
                    plan.taskStats.total
                  );
                  const showProgressBar =
                    plan.taskStats.total > 0 && plan.taskStats.incomplete > 0;

                  return (
                    <button
                      key={plan.sessionId}
                      ref={isHighlighted ? highlightedItemRef : null}
                      onClick={() => handleSelect(plan.sessionId)}
                      onMouseEnter={() => setHighlightedIndex(index)}
                      className={cn(
                        "w-full text-left px-3 py-2 rounded-lg flex items-center justify-between",
                        "transition-all duration-150 origin-center",
                        "hover:scale-[1.01]",
                        "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
                        isHighlighted && "bg-accent",
                        isActive && "bg-accent/50"
                      )}
                      style={{
                        background:
                          isHighlighted
                            ? "hsla(14 100% 60% / 0.16)"
                            : isActive
                              ? "hsla(14 100% 60% / 0.1)"
                              : "transparent",
                        border: isHighlighted
                          ? "1px solid hsla(14 100% 60% / 0.35)"
                          : "1px solid transparent",
                      }}
                    >
                      <div className="flex-1 min-w-0">
                        <div
                          className="text-[13px] font-medium leading-tight"
                          style={{ color: isHighlighted ? "hsl(14 100% 66%)" : "hsl(220 10% 90%)" }}
                        >
                          {plan.title || "Untitled Plan"}
                        </div>
                        <div className="text-xs leading-tight mt-0.5" style={{ color: "hsl(220 10% 62%)" }}>
                          {formatIncompleteSummary(plan.taskStats.incomplete, plan.taskStats.total)}
                          {plan.taskStats.activeNow > 0 && " • Active work"}
                        </div>
                      </div>

                      <div className="flex items-center gap-2 ml-3 shrink-0">
                        {showProgressBar && (
                          <div className="flex items-center gap-1.5" aria-hidden="true">
                            <div
                              className="w-14 h-1 rounded-full overflow-hidden"
                              style={{ backgroundColor: "hsla(220 10% 100% / 0.1)" }}
                            >
                              <div
                                className="h-full rounded-full transition-all duration-300"
                                style={{
                                  width: `${completionPercent}%`,
                                  backgroundColor: "hsla(14 100% 60% / 0.7)",
                                }}
                              />
                            </div>
                            <span
                              className="text-[10px] tabular-nums"
                              style={{ color: "hsl(220 10% 48%)" }}
                            >
                              {completionPercent}%
                            </span>
                          </div>
                        )}
                        {isActive && <Check className="h-4 w-4" style={{ color: "hsl(14 100% 62%)" }} />}
                      </div>
                    </button>
                  );
                })}
              </ScrollArea>
            ) : (
              /* Empty state */
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
