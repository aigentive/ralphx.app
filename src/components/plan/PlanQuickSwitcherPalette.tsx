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
}

// ============================================================================
// Component
// ============================================================================

export function PlanQuickSwitcherPalette({
  projectId,
  isOpen,
  onClose,
}: PlanQuickSwitcherPaletteProps) {
  const [searchQuery, setSearchQuery] = useState("");
  const [highlightedIndex, setHighlightedIndex] = useState(0);
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
    if (highlightedItemRef.current) {
      highlightedItemRef.current.scrollIntoView({
        block: "nearest",
        behavior: "smooth",
      });
    }
  }, [highlightedIndex]);

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
          setHighlightedIndex((i) => (i + 1) % candidateCount);
          break;
        case "ArrowUp":
          e.preventDefault();
          setHighlightedIndex((i) => (i - 1 + candidateCount) % candidateCount);
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
          className="fixed top-20 left-1/2 -translate-x-1/2 z-50 w-[600px]"
          ref={containerRef}
        >
          <div
            className="rounded-lg border border-white/10 bg-gray-900/90 backdrop-blur-xl shadow-2xl"
            onClick={(e) => e.stopPropagation()}
          >
            {/* Search input */}
            <div className="border-b border-white/10 p-4">
              <input
                ref={inputRef}
                type="text"
                placeholder="Search plans... (Cmd+Shift+P)"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                onKeyDown={handleKeyDown}
                className={cn(
                  "w-full bg-transparent border-0 text-white placeholder:text-gray-400",
                  "outline-none ring-0 focus:ring-0 focus:outline-none focus-visible:outline-none",
                  "transition-colors"
                )}
                style={{ boxShadow: "none", outline: "none" }}
              />
            </div>

            {/* Results list */}
            {error ? (
              <div className="p-8 flex flex-col items-center justify-center gap-3 text-gray-400">
                <AlertCircle className="h-5 w-5 text-red-400" />
                <p className="text-center">{error}</p>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleRetry}
                  className="gap-2 bg-white/5 border-white/10 text-white hover:bg-white/10"
                >
                  <RefreshCw className="h-4 w-4" />
                  Retry
                </Button>
              </div>
            ) : isLoading ? (
              <div className="p-8 flex flex-col items-center justify-center gap-2 text-gray-400 transition-colors">
                <Loader2 className="h-5 w-5 animate-spin" />
                <p>Loading plans...</p>
              </div>
            ) : filteredCandidates.length > 0 ? (
              <ScrollArea className="max-h-[400px]">
                {filteredCandidates.map((plan, index) => {
                  const isActive = activePlanId === plan.sessionId;
                  const isHighlighted = highlightedIndex === index;

                  return (
                    <button
                      key={plan.sessionId}
                      ref={isHighlighted ? highlightedItemRef : null}
                      onClick={() => handleSelect(plan.sessionId)}
                      onMouseEnter={() => setHighlightedIndex(index)}
                      className={cn(
                        "w-full text-left px-4 py-3 flex items-center justify-between",
                        "hover:bg-white/5 hover:scale-[1.01] transition-all origin-center",
                        isHighlighted && "bg-white/10 ring-2 ring-[#ff6b35] ring-opacity-50",
                        isActive && "border-l-2 border-[#ff6b35]"
                      )}
                    >
                      <div className="flex-1">
                        <div className="font-medium text-white">
                          {plan.title || "Untitled Plan"}
                        </div>
                        <div className="text-sm text-gray-400">
                          {plan.taskStats.incomplete}/{plan.taskStats.total} incomplete
                          {plan.taskStats.activeNow > 0 && " • Active work"}
                        </div>
                      </div>

                      {isActive && <Check className="h-4 w-4 text-[#ff6b35]" />}
                    </button>
                  );
                })}
              </ScrollArea>
            ) : (
              /* Empty state */
              <div className="p-8 text-center text-gray-400 transition-colors">
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
