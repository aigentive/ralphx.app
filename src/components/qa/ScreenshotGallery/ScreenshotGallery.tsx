/**
 * ScreenshotGallery - Professional screenshot gallery with comparison mode
 *
 * Premium design with Lucide icons:
 * - Thumbnail grid with hover effects
 * - Full-featured lightbox with navigation
 * - Expected vs Actual comparison view for failures
 * - Keyboard navigation (arrows, escape)
 * - Image zoom and pan
 */

import { useState, useCallback, useEffect, useRef } from "react";
import {
  ChevronLeft,
  ChevronRight,
  X,
  ZoomIn,
  ZoomOut,
  GitCompare,
  Image,
  AlertTriangle,
  Check,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
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
      className={cn(
        "group relative aspect-video rounded-xl overflow-hidden",
        "bg-[var(--bg-elevated)]",
        "transition-all duration-200",
        "hover:ring-2 hover:ring-[var(--accent-primary)] hover:ring-offset-2 hover:ring-offset-[var(--bg-base)]",
        "focus:outline-none focus:ring-2 focus:ring-[var(--accent-primary)] focus:ring-offset-2 focus:ring-offset-[var(--bg-base)]",
        "active:scale-[0.98]"
      )}
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
        <div className="w-full h-full flex items-center justify-center text-[var(--text-muted)]">
          <Image className="w-8 h-8" />
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
          className="absolute top-2 right-2 p-1.5 rounded-full bg-[var(--status-error)] text-white"
        >
          <X className="w-3.5 h-3.5" />
        </div>
      )}

      {/* Passed indicator */}
      {screenshot.stepResult?.status === "passed" && (
        <div
          data-testid={`screenshot-passed-indicator-${index}`}
          className="absolute top-2 right-2 p-1.5 rounded-full bg-[var(--status-success)] text-white"
        >
          <Check className="w-3.5 h-3.5" />
        </div>
      )}

      {/* Comparison available indicator */}
      {screenshot.expectedPath && (
        <div
          data-testid={`screenshot-comparison-indicator-${index}`}
          className="absolute top-2 left-2 p-1.5 rounded bg-[var(--accent-primary)] text-white"
          title="Comparison available"
        >
          <GitCompare className="w-3.5 h-3.5" />
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
              <Badge
                variant="outline"
                className="border-0 bg-red-500/20 text-[var(--status-error)] gap-1.5"
              >
                <AlertTriangle className="w-3 h-3" />
                Failed
              </Badge>
            )}
            {current.stepResult?.status === "passed" && (
              <Badge
                variant="outline"
                className="border-0 bg-emerald-500/20 text-[var(--status-success)] gap-1.5"
              >
                <Check className="w-3 h-3" />
                Passed
              </Badge>
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
            <Button
              data-testid="lightbox-toggle-comparison"
              onClick={(e) => {
                e.stopPropagation();
                setViewMode((m) => (m === "single" ? "comparison" : "single"));
              }}
              variant={viewMode === "comparison" ? "default" : "ghost"}
              size="sm"
              className={cn(
                "gap-2",
                viewMode === "comparison"
                  ? "bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)]"
                  : "text-white hover:bg-white/10"
              )}
            >
              <GitCompare className="w-4 h-4" />
              Compare
            </Button>
          )}

          {/* Zoom controls */}
          <div className="flex items-center gap-1 ml-4">
            <Button
              data-testid="lightbox-zoom-out"
              onClick={(e) => {
                e.stopPropagation();
                setZoom((z) => Math.max(z - 0.25, 0.5));
              }}
              disabled={zoom <= 0.5}
              variant="ghost"
              size="icon"
              className="text-white hover:bg-white/10 disabled:opacity-30"
              title="Zoom out (-)"
            >
              <ZoomOut className="w-5 h-5" />
            </Button>
            <span className="w-12 text-center text-sm text-white/80">
              {Math.round(zoom * 100)}%
            </span>
            <Button
              data-testid="lightbox-zoom-in"
              onClick={(e) => {
                e.stopPropagation();
                setZoom((z) => Math.min(z + 0.25, 4));
              }}
              disabled={zoom >= 4}
              variant="ghost"
              size="icon"
              className="text-white hover:bg-white/10 disabled:opacity-30"
              title="Zoom in (+)"
            >
              <ZoomIn className="w-5 h-5" />
            </Button>
          </div>

          {/* Close button */}
          <Button
            data-testid="lightbox-close"
            onClick={(e) => {
              e.stopPropagation();
              onClose();
            }}
            variant="ghost"
            size="icon"
            className="text-white hover:bg-white/10 ml-4"
            title="Close (Esc)"
          >
            <X className="w-6 h-6" />
          </Button>
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
                <Badge
                  variant="outline"
                  className="border-0 px-2 py-1 text-xs font-semibold uppercase tracking-wider bg-emerald-500/20 text-[var(--status-success)]"
                >
                  Expected
                </Badge>
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
                  className="max-h-[65vh] object-contain rounded-xl border border-emerald-500/30"
                  draggable={false}
                />
              ) : (
                <div className="flex-1 flex items-center justify-center rounded-xl border border-dashed border-white/20 bg-white/5 min-h-[200px]">
                  <div className="text-center text-white/40">
                    <Image className="w-12 h-12 mx-auto mb-2 opacity-50" />
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
                <Badge
                  variant="outline"
                  className="border-0 px-2 py-1 text-xs font-semibold uppercase tracking-wider bg-red-500/20 text-[var(--status-error)]"
                >
                  Actual
                </Badge>
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
                className="max-h-[65vh] object-contain rounded-xl border border-red-500/30"
                draggable={false}
              />
            </div>
          </div>
        )}

        {/* Navigation arrows */}
        {screenshots.length > 1 && viewMode === "single" && (
          <>
            <Button
              data-testid="lightbox-prev"
              onClick={(e) => {
                e.stopPropagation();
                if (hasPrev) onNavigate(currentIndex - 1);
              }}
              disabled={!hasPrev}
              variant="ghost"
              size="icon"
              className="absolute left-4 w-12 h-12 rounded-full bg-black/50 text-white hover:bg-black/70 disabled:opacity-30"
              title="Previous (←)"
            >
              <ChevronLeft className="w-6 h-6" />
            </Button>
            <Button
              data-testid="lightbox-next"
              onClick={(e) => {
                e.stopPropagation();
                if (hasNext) onNavigate(currentIndex + 1);
              }}
              disabled={!hasNext}
              variant="ghost"
              size="icon"
              className="absolute right-4 w-12 h-12 rounded-full bg-black/50 text-white hover:bg-black/70 disabled:opacity-30"
              title="Next (→)"
            >
              <ChevronRight className="w-6 h-6" />
            </Button>
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
              className={cn(
                "w-16 h-10 rounded-sm overflow-hidden transition-all",
                index === currentIndex
                  ? "ring-2 ring-[var(--accent-primary)] ring-offset-2 ring-offset-black"
                  : "opacity-50 hover:opacity-80"
              )}
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
      <div className="w-16 h-16 rounded-full bg-[var(--bg-elevated)] flex items-center justify-center mb-4">
        <Image className="w-8 h-8 text-[var(--text-muted)]" />
      </div>
      <p className="text-sm font-medium text-[var(--text-secondary)]">{message}</p>
      <p className="text-xs text-[var(--text-muted)] opacity-60 mt-1">
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
    <div data-testid="screenshot-gallery" className={cn(className)}>
      {/* Thumbnail Grid */}
      <div className={cn("grid gap-3", gridCols[columns])}>
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
