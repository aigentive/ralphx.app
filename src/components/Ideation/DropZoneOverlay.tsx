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
        background: "rgba(10, 10, 10, 0.85)",
        // Pulsing orange border
        boxShadow: "inset 0 0 0 2px rgba(255, 107, 53, 0.8)",
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
              "linear-gradient(135deg, rgba(255, 107, 53, 0.2) 0%, rgba(255, 107, 53, 0.08) 100%)",
            border: "1px solid rgba(255, 107, 53, 0.4)",
            boxShadow: "0 0 32px rgba(255, 107, 53, 0.2)",
          }}
        >
          <FileDown
            className="w-6 h-6"
            style={{ color: "#ff6b35" }}
            strokeWidth={1.5}
          />
        </div>

        {/* Message */}
        <span
          className="text-sm font-medium tracking-tight"
          style={{ color: "rgba(255, 255, 255, 0.9)" }}
        >
          {message}
        </span>
      </div>

      {/* CSS keyframes for the pulsing animation */}
      <style>{`
        @keyframes dropzone-pulse {
          0%, 100% {
            box-shadow: inset 0 0 0 2px rgba(255, 107, 53, 0.8);
          }
          50% {
            box-shadow: inset 0 0 0 3px rgba(255, 107, 53, 1), 0 0 20px rgba(255, 107, 53, 0.3);
          }
        }
      `}</style>
    </div>
  );
}
