/**
 * PlanQuickSwitcherPalette - Placeholder for plan quick switcher
 * TODO: Full implementation in separate task (Phase 4)
 */

import { useEffect, useRef } from "react";
import { X, FileText } from "lucide-react";
import { motion, AnimatePresence } from "framer-motion";

interface PlanQuickSwitcherPaletteProps {
  isOpen: boolean;
  onClose: () => void;
}

/**
 * Minimal placeholder component for plan quick switcher.
 * Shows that Cmd+Shift+P keyboard shortcut is working.
 * Full implementation will include:
 * - Plan selection from backend
 * - Fuzzy search/filtering
 * - Keyboard navigation (up/down/enter)
 * - Integration with planStore
 */
export function PlanQuickSwitcherPalette({
  isOpen,
  onClose,
}: PlanQuickSwitcherPaletteProps) {
  const panelRef = useRef<HTMLDivElement>(null);

  // Auto-focus on open
  useEffect(() => {
    if (isOpen && panelRef.current) {
      panelRef.current.focus();
    }
  }, [isOpen]);

  // Handle escape key
  useEffect(() => {
    if (!isOpen) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isOpen, onClose]);

  return (
    <AnimatePresence>
      {isOpen && (
        <motion.div
          ref={panelRef}
          initial={{ opacity: 0, y: -20 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: -20 }}
          transition={{ duration: 0.15 }}
          className="fixed top-20 left-1/2 -translate-x-1/2 z-50 w-[600px]"
          data-quick-switcher-panel
          tabIndex={-1}
        >
          <div
            className="rounded-lg border shadow-2xl overflow-hidden"
            style={{
              background: "hsla(220 10% 10% / 0.95)",
              backdropFilter: "blur(24px)",
              WebkitBackdropFilter: "blur(24px)",
              borderColor: "hsla(220 10% 100% / 0.1)",
              boxShadow: "0 8px 32px hsla(220 20% 0% / 0.5)",
            }}
            onClick={(e) => e.stopPropagation()}
          >
            {/* Header */}
            <div
              className="flex items-center justify-between p-4 border-b"
              style={{ borderColor: "hsla(220 10% 100% / 0.1)" }}
            >
              <div className="flex items-center gap-3">
                <FileText
                  className="w-5 h-5"
                  style={{ color: "#ff6b35" }}
                />
                <h2 className="text-lg font-semibold" style={{ color: "var(--text-primary)" }}>
                  Plan Quick Switcher
                </h2>
              </div>
              <button
                onClick={onClose}
                className="p-1 rounded hover:bg-white/5 transition-colors"
                style={{ color: "rgba(255,255,255,0.5)" }}
                aria-label="Close"
              >
                <X className="w-5 h-5" />
              </button>
            </div>

            {/* Placeholder content */}
            <div className="p-8 text-center">
              <div className="mb-4">
                <FileText
                  className="w-12 h-12 mx-auto mb-3 opacity-50"
                  style={{ color: "#ff6b35" }}
                />
              </div>
              <h3 className="text-lg font-medium mb-2" style={{ color: "var(--text-primary)" }}>
                Coming Soon
              </h3>
              <p className="text-sm mb-4" style={{ color: "rgba(255,255,255,0.6)" }}>
                Plan quick switcher is under development.
              </p>
              <p className="text-xs" style={{ color: "rgba(255,255,255,0.4)" }}>
                Press <kbd className="px-2 py-1 rounded" style={{ backgroundColor: "rgba(255,255,255,0.1)" }}>Esc</kbd> to close
              </p>
            </div>
          </div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
