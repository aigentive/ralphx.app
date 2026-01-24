/**
 * ScreenshotGallery - Professional screenshot gallery with comparison mode
 *
 * Features:
 * - Thumbnail grid with hover effects
 * - Full-featured lightbox with navigation
 * - Expected vs Actual comparison view for failures
 * - Keyboard navigation (arrows, escape)
 * - Image zoom and pan
 */

import { useState, useCallback, useEffect, useRef } from "react";
import type { QAStepResult } from "@/types/qa";

// ============================================================================
// Types
// ============================================================================

export interface Screenshot {
  /** Unique identifier */
  id: string;
  /** Path or URL to the screenshot */
  path: string;
  /** Display label (e.g., step ID or filename) */
  label: string;
  /** Optional timestamp */
  timestamp?: string | undefined;
  /** Optional step result for failure context */
  stepResult?: QAStepResult | undefined;
  /** Expected screenshot path for comparison (if available) */
  expectedPath?: string | undefined;
}

export interface ScreenshotGalleryProps {
  /** Array of screenshots to display */
  screenshots: Screenshot[];
  /** Initial selected screenshot index */
  initialIndex?: number;
  /** Callback when lightbox is opened */
  onOpen?: (index: number) => void;
  /** Callback when lightbox is closed */
  onClose?: () => void;
  /** Optional class name for container */
  className?: string;
  /** Grid columns (default: 3) */
  columns?: 2 | 3 | 4;
  /** Empty state message */
  emptyMessage?: string;
}

type ViewMode = "single" | "comparison";

// ============================================================================
// Icons
// ============================================================================

function ChevronLeftIcon() {
  return (
    <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
      <path
        d="M15 6L9 12L15 18"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function ChevronRightIcon() {
  return (
    <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
      <path
        d="M9 6L15 12L9 18"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function CloseIcon() {
  return (
    <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
      <path
        d="M18 6L6 18M6 6L18 18"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function ZoomInIcon() {
  return (
    <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
      <circle cx="9" cy="9" r="6" stroke="currentColor" strokeWidth="1.5" />
      <path d="M13.5 13.5L17 17" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
      <path d="M9 6V12M6 9H12" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
    </svg>
  );
}

function ZoomOutIcon() {
  return (
    <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
      <circle cx="9" cy="9" r="6" stroke="currentColor" strokeWidth="1.5" />
      <path d="M13.5 13.5L17 17" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
      <path d="M6 9H12" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
    </svg>
  );
}

function CompareIcon() {
  return (
    <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
      <rect x="2" y="3" width="7" height="14" rx="1" stroke="currentColor" strokeWidth="1.5" />
      <rect x="11" y="3" width="7" height="14" rx="1" stroke="currentColor" strokeWidth="1.5" />
      <path d="M10 6V14" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeDasharray="2 2" />
    </svg>
  );
}

function ImageIcon({ className = "" }: { className?: string }) {
  return (
    <svg
      width="24"
      height="24"
      viewBox="0 0 24 24"
      fill="none"
      className={className}
    >
      <rect
        x="3"
        y="3"
        width="18"
        height="18"
        rx="2"
        stroke="currentColor"
        strokeWidth="1.5"
      />
      <circle cx="8" cy="8" r="2" fill="currentColor" />
      <path
        d="M21 15L16 10L8 18H19C20.1046 18 21 17.1046 21 16V15Z"
        fill="currentColor"
      />
    </svg>
  );
}

function AlertIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
      <path
        d="M8 1L15 14H1L8 1Z"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <path d="M8 6V9" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
      <circle cx="8" cy="11.5" r="0.75" fill="currentColor" />
    </svg>
  );
}

function CheckIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
      <path
        d="M13 4L6 11L3 8"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function XIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
      <path
        d="M12 4L4 12M4 4L12 12"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

// ============================================================================
// Helper Functions
// ============================================================================

function getFilename(path: string): string {
  return path.split("/").pop() || path;
}

function formatTimestamp(timestamp: string | undefined): string {
  if (!timestamp) return "";
  try {
    const date = new Date(timestamp);
    return date.toLocaleTimeString(undefined, {
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
  } catch {
    return timestamp;
  }
}

// ============================================================================
// Thumbnail Component
// ============================================================================

interface ThumbnailProps {
  screenshot: Screenshot;
  index: number;
  onClick: (index: number) => void;
}

function Thumbnail({ screenshot, index, onClick }: ThumbnailProps) {
  const [imageError, setImageError] = useState(false);
  const isFailed = screenshot.stepResult?.status === "failed";

  return (
    <button
      data-testid={`screenshot-thumbnail-${index}`}
      onClick={() => onClick(index)}
      className="group relative aspect-video rounded-lg overflow-hidden bg-[--bg-elevated] transition-all duration-200 hover:ring-2 hover:ring-[--accent-primary] hover:ring-offset-2 hover:ring-offset-[--bg-base] focus:outline-none focus:ring-2 focus:ring-[--accent-primary] focus:ring-offset-2 focus:ring-offset-[--bg-base]"
    >
      {/* Image or Placeholder */}
      {!imageError ? (
        <img
          src={screenshot.path}
          alt={screenshot.label}
          className="w-full h-full object-cover transition-transform duration-300 group-hover:scale-105"
          onError={() => setImageError(true)}
        />
      ) : (
        <div className="w-full h-full flex items-center justify-center text-[--text-muted]">
          <ImageIcon className="w-8 h-8" />
        </div>
      )}

      {/* Overlay on hover */}
      <div className="absolute inset-0 bg-gradient-to-t from-black/70 via-transparent to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-200" />

      {/* Label */}
      <div className="absolute bottom-0 left-0 right-0 p-2 flex items-center justify-between opacity-0 group-hover:opacity-100 transition-opacity duration-200">
        <span className="text-xs font-medium text-white truncate">
          {screenshot.label}
        </span>
        {screenshot.timestamp && (
          <span className="text-xs text-white/70">
            {formatTimestamp(screenshot.timestamp)}
          </span>
        )}
      </div>

      {/* Failed indicator */}
      {isFailed && (
        <div
          data-testid={`screenshot-failed-indicator-${index}`}
          className="absolute top-2 right-2 p-1.5 rounded-full bg-[--status-error] text-white"
        >
          <XIcon />
        </div>
      )}

      {/* Passed indicator */}
      {screenshot.stepResult?.status === "passed" && (
        <div
          data-testid={`screenshot-passed-indicator-${index}`}
          className="absolute top-2 right-2 p-1.5 rounded-full bg-[--status-success] text-white"
        >
          <CheckIcon />
        </div>
      )}

      {/* Comparison available indicator */}
      {screenshot.expectedPath && (
        <div
          data-testid={`screenshot-comparison-indicator-${index}`}
          className="absolute top-2 left-2 p-1 rounded bg-[--accent-primary] text-white"
          title="Comparison available"
        >
          <CompareIcon />
        </div>
      )}
    </button>
  );
}

// ============================================================================
// Lightbox Component
// ============================================================================

interface LightboxProps {
  screenshots: Screenshot[];
  currentIndex: number;
  onClose: () => void;
  onNavigate: (index: number) => void;
}

function Lightbox({
  screenshots,
  currentIndex,
  onClose,
  onNavigate,
}: LightboxProps) {
  const [viewMode, setViewMode] = useState<ViewMode>("single");
  const [zoom, setZoom] = useState(1);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [isDragging, setIsDragging] = useState(false);
  const dragStart = useRef({ x: 0, y: 0, panX: 0, panY: 0 });
  const imageRef = useRef<HTMLDivElement>(null);

  const current = screenshots[currentIndex];
  const hasPrev = currentIndex > 0;
  const hasNext = currentIndex < screenshots.length - 1;
  const hasComparison = !!current?.expectedPath || !!current?.stepResult;
  const isFailed = current?.stepResult?.status === "failed";

  // Reset zoom/pan when changing images
  useEffect(() => {
    setZoom(1);
    setPan({ x: 0, y: 0 });
  }, [currentIndex]);

  // Keyboard navigation
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
      if (e.key === "ArrowLeft" && hasPrev) onNavigate(currentIndex - 1);
      if (e.key === "ArrowRight" && hasNext) onNavigate(currentIndex + 1);
      if (e.key === "+" || e.key === "=") setZoom((z) => Math.min(z + 0.25, 4));
      if (e.key === "-") setZoom((z) => Math.max(z - 0.25, 0.5));
      if (e.key === "0") {
        setZoom(1);
        setPan({ x: 0, y: 0 });
      }
      if (e.key === "c" && hasComparison) {
        setViewMode((m) => (m === "single" ? "comparison" : "single"));
      }
    };
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [onClose, onNavigate, currentIndex, hasPrev, hasNext, hasComparison]);

  // Mouse drag for panning
  const handleMouseDown = (e: React.MouseEvent) => {
    if (zoom > 1) {
      setIsDragging(true);
      dragStart.current = {
        x: e.clientX,
        y: e.clientY,
        panX: pan.x,
        panY: pan.y,
      };
    }
  };

  const handleMouseMove = (e: React.MouseEvent) => {
    if (isDragging) {
      const dx = e.clientX - dragStart.current.x;
      const dy = e.clientY - dragStart.current.y;
      setPan({
        x: dragStart.current.panX + dx,
        y: dragStart.current.panY + dy,
      });
    }
  };

  const handleMouseUp = () => {
    setIsDragging(false);
  };

  if (!current) return null;

  return (
    <div
      data-testid="screenshot-lightbox"
      className="fixed inset-0 z-50 flex flex-col bg-black/95"
      onClick={onClose}
    >
      {/* Header */}
      <header
        className="flex items-center justify-between px-6 py-4 bg-gradient-to-b from-black/50 to-transparent"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center gap-4">
          {/* Filename and status */}
          <div className="flex items-center gap-3">
            <h2
              data-testid="lightbox-filename"
              className="text-white font-medium"
            >
              {current.label}
            </h2>
            {isFailed && (
              <span className="flex items-center gap-1.5 px-2 py-0.5 rounded text-xs font-medium bg-[--status-error]/20 text-[--status-error]">
                <AlertIcon />
                Failed
              </span>
            )}
            {current.stepResult?.status === "passed" && (
              <span className="flex items-center gap-1.5 px-2 py-0.5 rounded text-xs font-medium bg-[--status-success]/20 text-[--status-success]">
                <CheckIcon />
                Passed
              </span>
            )}
          </div>

          {/* Counter */}
          <span
            data-testid="lightbox-counter"
            className="text-white/60 text-sm"
          >
            {currentIndex + 1} / {screenshots.length}
          </span>
        </div>

        {/* Controls */}
        <div className="flex items-center gap-2">
          {/* View mode toggle */}
          {hasComparison && (
            <button
              data-testid="lightbox-toggle-comparison"
              onClick={(e) => {
                e.stopPropagation();
                setViewMode((m) => (m === "single" ? "comparison" : "single"));
              }}
              className={`flex items-center gap-2 px-3 py-1.5 rounded text-sm font-medium transition-colors ${
                viewMode === "comparison"
                  ? "bg-[--accent-primary] text-white"
                  : "bg-white/10 text-white hover:bg-white/20"
              }`}
            >
              <CompareIcon />
              Compare
            </button>
          )}

          {/* Zoom controls */}
          <div className="flex items-center gap-1 ml-4">
            <button
              data-testid="lightbox-zoom-out"
              onClick={(e) => {
                e.stopPropagation();
                setZoom((z) => Math.max(z - 0.25, 0.5));
              }}
              disabled={zoom <= 0.5}
              className="p-2 text-white hover:bg-white/10 rounded disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
              title="Zoom out (-)"
            >
              <ZoomOutIcon />
            </button>
            <span className="w-12 text-center text-sm text-white/80">
              {Math.round(zoom * 100)}%
            </span>
            <button
              data-testid="lightbox-zoom-in"
              onClick={(e) => {
                e.stopPropagation();
                setZoom((z) => Math.min(z + 0.25, 4));
              }}
              disabled={zoom >= 4}
              className="p-2 text-white hover:bg-white/10 rounded disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
              title="Zoom in (+)"
            >
              <ZoomInIcon />
            </button>
          </div>

          {/* Close button */}
          <button
            data-testid="lightbox-close"
            onClick={(e) => {
              e.stopPropagation();
              onClose();
            }}
            className="p-2 text-white hover:bg-white/10 rounded transition-colors ml-4"
            title="Close (Esc)"
          >
            <CloseIcon />
          </button>
        </div>
      </header>

      {/* Main content area */}
      <div
        className="flex-1 flex items-center justify-center relative overflow-hidden"
        onClick={(e) => e.stopPropagation()}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onMouseLeave={handleMouseUp}
        style={{ cursor: zoom > 1 ? (isDragging ? "grabbing" : "grab") : "default" }}
      >
        {viewMode === "single" ? (
          /* Single image view */
          <div
            ref={imageRef}
            className="transition-transform duration-200"
            style={{
              transform: `scale(${zoom}) translate(${pan.x / zoom}px, ${pan.y / zoom}px)`,
            }}
          >
            <img
              src={current.path}
              alt={current.label}
              className="max-w-[90vw] max-h-[75vh] object-contain rounded-lg shadow-2xl"
              draggable={false}
            />
          </div>
        ) : (
          /* Comparison view */
          <div
            data-testid="comparison-view"
            className="flex gap-6 max-w-[95vw] max-h-[75vh]"
          >
            {/* Expected panel */}
            <div className="flex-1 flex flex-col">
              <div className="flex items-center gap-2 mb-3">
                <span className="px-2 py-1 rounded text-xs font-semibold uppercase tracking-wider bg-[--status-success]/20 text-[--status-success]">
                  Expected
                </span>
                {current.stepResult?.expected && (
                  <span className="text-sm text-white/60 truncate max-w-[300px]">
                    {current.stepResult.expected}
                  </span>
                )}
              </div>
              {current.expectedPath ? (
                <img
                  data-testid="comparison-expected-image"
                  src={current.expectedPath}
                  alt="Expected"
                  className="max-h-[65vh] object-contain rounded-lg border border-[--status-success]/30"
                  draggable={false}
                />
              ) : (
                <div className="flex-1 flex items-center justify-center rounded-lg border border-dashed border-white/20 bg-white/5 min-h-[200px]">
                  <div className="text-center text-white/40">
                    <ImageIcon className="w-12 h-12 mx-auto mb-2 opacity-50" />
                    <p className="text-sm">No expected screenshot</p>
                    {current.stepResult?.expected && (
                      <p className="text-xs mt-2 max-w-[200px] mx-auto">
                        Expected: "{current.stepResult.expected}"
                      </p>
                    )}
                  </div>
                </div>
              )}
            </div>

            {/* Divider */}
            <div className="w-px bg-white/20 self-stretch" />

            {/* Actual panel */}
            <div className="flex-1 flex flex-col">
              <div className="flex items-center gap-2 mb-3">
                <span className="px-2 py-1 rounded text-xs font-semibold uppercase tracking-wider bg-[--status-error]/20 text-[--status-error]">
                  Actual
                </span>
                {current.stepResult?.actual && (
                  <span className="text-sm text-white/60 truncate max-w-[300px]">
                    {current.stepResult.actual}
                  </span>
                )}
              </div>
              <img
                data-testid="comparison-actual-image"
                src={current.path}
                alt="Actual"
                className="max-h-[65vh] object-contain rounded-lg border border-[--status-error]/30"
                draggable={false}
              />
            </div>
          </div>
        )}

        {/* Navigation arrows */}
        {screenshots.length > 1 && viewMode === "single" && (
          <>
            <button
              data-testid="lightbox-prev"
              onClick={(e) => {
                e.stopPropagation();
                if (hasPrev) onNavigate(currentIndex - 1);
              }}
              disabled={!hasPrev}
              className="absolute left-4 p-3 rounded-full bg-black/50 text-white hover:bg-black/70 disabled:opacity-30 disabled:cursor-not-allowed transition-all"
              title="Previous (←)"
            >
              <ChevronLeftIcon />
            </button>
            <button
              data-testid="lightbox-next"
              onClick={(e) => {
                e.stopPropagation();
                if (hasNext) onNavigate(currentIndex + 1);
              }}
              disabled={!hasNext}
              className="absolute right-4 p-3 rounded-full bg-black/50 text-white hover:bg-black/70 disabled:opacity-30 disabled:cursor-not-allowed transition-all"
              title="Next (→)"
            >
              <ChevronRightIcon />
            </button>
          </>
        )}
      </div>

      {/* Footer with failure details */}
      {isFailed && current.stepResult && (
        <footer
          data-testid="lightbox-failure-details"
          className="px-6 py-4 bg-gradient-to-t from-black/50 to-transparent"
          onClick={(e) => e.stopPropagation()}
        >
          <div className="max-w-3xl mx-auto p-4 rounded-lg bg-[--status-error]/10 border border-[--status-error]/30">
            {current.stepResult.error && (
              <p className="text-sm text-[--status-error] mb-2">
                <span className="font-semibold">Error:</span> {current.stepResult.error}
              </p>
            )}
            {(current.stepResult.expected || current.stepResult.actual) && (
              <div className="grid grid-cols-2 gap-4 text-sm">
                {current.stepResult.expected && (
                  <div>
                    <span className="text-[--status-success] font-medium">Expected:</span>
                    <p className="text-white/80 mt-0.5">{current.stepResult.expected}</p>
                  </div>
                )}
                {current.stepResult.actual && (
                  <div>
                    <span className="text-[--status-error] font-medium">Actual:</span>
                    <p className="text-white/80 mt-0.5">{current.stepResult.actual}</p>
                  </div>
                )}
              </div>
            )}
          </div>
        </footer>
      )}

      {/* Thumbnail strip */}
      {screenshots.length > 1 && (
        <div
          className="flex justify-center gap-2 px-6 py-4 bg-black/50"
          onClick={(e) => e.stopPropagation()}
        >
          {screenshots.map((screenshot, index) => (
            <button
              key={screenshot.id}
              data-testid={`lightbox-thumbnail-${index}`}
              onClick={() => onNavigate(index)}
              className={`w-16 h-10 rounded overflow-hidden transition-all ${
                index === currentIndex
                  ? "ring-2 ring-[--accent-primary] ring-offset-2 ring-offset-black"
                  : "opacity-50 hover:opacity-80"
              }`}
            >
              <img
                src={screenshot.path}
                alt={screenshot.label}
                className="w-full h-full object-cover"
              />
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

// ============================================================================
// Empty State Component
// ============================================================================

interface EmptyStateProps {
  message?: string | undefined;
}

function EmptyState({ message = "No screenshots captured" }: EmptyStateProps) {
  return (
    <div
      data-testid="screenshot-gallery-empty"
      className="flex flex-col items-center justify-center py-12 text-center"
    >
      <div className="w-16 h-16 rounded-full bg-[--bg-elevated] flex items-center justify-center mb-4">
        <ImageIcon className="w-8 h-8 text-[--text-muted]" />
      </div>
      <p className="text-[--text-muted] text-sm">{message}</p>
      <p className="text-[--text-muted]/60 text-xs mt-1">
        Screenshots will appear here when captured during QA testing
      </p>
    </div>
  );
}

// ============================================================================
// Main Component
// ============================================================================

export function ScreenshotGallery({
  screenshots,
  initialIndex,
  onOpen,
  onClose,
  className = "",
  columns = 3,
  emptyMessage,
}: ScreenshotGalleryProps) {
  const [lightboxIndex, setLightboxIndex] = useState<number | null>(
    initialIndex ?? null
  );

  const handleThumbnailClick = useCallback(
    (index: number) => {
      setLightboxIndex(index);
      onOpen?.(index);
    },
    [onOpen]
  );

  const handleLightboxClose = useCallback(() => {
    setLightboxIndex(null);
    onClose?.();
  }, [onClose]);

  const handleLightboxNavigate = useCallback((index: number) => {
    setLightboxIndex(index);
  }, []);

  // Empty state
  if (screenshots.length === 0) {
    return <EmptyState message={emptyMessage} />;
  }

  const gridCols = {
    2: "grid-cols-2",
    3: "grid-cols-3",
    4: "grid-cols-4",
  };

  return (
    <div data-testid="screenshot-gallery" className={className}>
      {/* Thumbnail Grid */}
      <div className={`grid ${gridCols[columns]} gap-3`}>
        {screenshots.map((screenshot, index) => (
          <Thumbnail
            key={screenshot.id}
            screenshot={screenshot}
            index={index}
            onClick={handleThumbnailClick}
          />
        ))}
      </div>

      {/* Lightbox */}
      {lightboxIndex !== null && (
        <Lightbox
          screenshots={screenshots}
          currentIndex={lightboxIndex}
          onClose={handleLightboxClose}
          onNavigate={handleLightboxNavigate}
        />
      )}
    </div>
  );
}

// ============================================================================
// Utility: Convert simple paths to Screenshot objects
// ============================================================================

/**
 * Convert an array of screenshot paths to Screenshot objects
 * Useful for integration with TaskDetailQAPanel
 */
export function pathsToScreenshots(
  paths: string[],
  stepResults?: Map<string, QAStepResult>
): Screenshot[] {
  return paths.map((path, index) => {
    const filename = getFilename(path);
    // Try to match with step result based on filename
    const matchingResult = stepResults
      ? Array.from(stepResults.values()).find((r) => r.screenshot === path)
      : undefined;

    return {
      id: `screenshot-${index}`,
      path,
      label: matchingResult?.step_id || filename,
      stepResult: matchingResult,
    };
  });
}

export default ScreenshotGallery;
