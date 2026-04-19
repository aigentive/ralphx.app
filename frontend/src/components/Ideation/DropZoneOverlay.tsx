/**
 * DropZoneOverlay - Visual feedback during drag-and-drop
 *
 * Shows a pulsing orange border and centered message when
 * files are being dragged over the drop zone.
 */

import { FileDown } from "lucide-react";

interface DropZoneOverlayProps {
  /** Whether the overlay is visible */
  isVisible: boolean;
  /** Custom message to display (default: "Drop to import") */
  message?: string;
}

export function DropZoneOverlay({
  isVisible,
  message = "Drop to import",
}: DropZoneOverlayProps) {
  if (!isVisible) {
    return null;
  }

  return (
    <div
      data-testid="drop-zone-overlay"
      className="absolute inset-0 z-50 pointer-events-none rounded-xl overflow-hidden"
      style={{
        // Dimmed background
        background: "var(--overlay-scrim-deep)",
        // Pulsing orange border — keyframe overrides this with --shadow-drop-zone-*
        boxShadow: "var(--shadow-drop-zone-border)",
        animation: "dropzone-pulse 1.5s ease-in-out infinite",
      }}
    >
      {/* Centered content */}
      <div className="absolute inset-0 flex flex-col items-center justify-center gap-3">
        {/* Icon container with glow */}
        <div
          data-testid="drop-zone-icon"
          className="w-14 h-14 rounded-xl flex items-center justify-center"
          style={{
            background:
              "linear-gradient(135deg, var(--accent-border) 0%, var(--accent-muted) 100%)",
            border: "1px solid var(--accent-strong)",
            boxShadow: "0 0 32px var(--accent-border)",
          }}
        >
          <FileDown
            className="w-6 h-6"
            style={{ color: "var(--accent-primary)" }}
            strokeWidth={1.5}
          />
        </div>

        {/* Message */}
        <span
          className="text-sm font-medium tracking-tight"
          style={{ color: "var(--text-primary)" }}
        >
          {message}
        </span>
      </div>

      {/* CSS keyframes for the pulsing animation */}
      <style>{`
        @keyframes dropzone-pulse {
          0%, 100% {
            box-shadow: var(--shadow-drop-zone-border);
          }
          50% {
            box-shadow: var(--shadow-drop-zone-active);
          }
        }
      `}</style>
    </div>
  );
}
