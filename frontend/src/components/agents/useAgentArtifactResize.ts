import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type MouseEvent as ReactMouseEvent,
} from "react";

import {
  AGENTS_ARTIFACT_MIN_WIDTH,
  AGENTS_CHAT_MIN_WIDTH,
} from "./AgentsArtifactPaneRegion";

const AGENTS_ARTIFACT_WIDTH_STORAGE_KEY = "ralphx-agents-artifact-width";
const AGENTS_ARTIFACT_DEFAULT_WIDTH = "66.666667%";

export function useAgentArtifactResize() {
  const [artifactPanelWidth, setArtifactPanelWidth] = useState<number | null>(() => {
    const saved = window.localStorage.getItem(AGENTS_ARTIFACT_WIDTH_STORAGE_KEY);
    if (!saved) {
      return null;
    }
    const parsed = Number.parseInt(saved, 10);
    return Number.isFinite(parsed) && parsed >= AGENTS_ARTIFACT_MIN_WIDTH ? parsed : null;
  });
  const [isArtifactResizing, setIsArtifactResizing] = useState(false);
  const splitContainerRef = useRef<HTMLDivElement>(null);
  const artifactResizeFrameRef = useRef<number | null>(null);
  const pendingArtifactWidthRef = useRef<number | null>(null);
  const artifactResizeBoundsRef = useRef<{ right: number; maxWidth: number } | null>(null);
  const artifactWidthCss = artifactPanelWidth
    ? `${artifactPanelWidth}px`
    : AGENTS_ARTIFACT_DEFAULT_WIDTH;

  const handleArtifactResizeStart = useCallback((event: ReactMouseEvent) => {
    event.preventDefault();
    const container = splitContainerRef.current;
    if (container) {
      const rect = container.getBoundingClientRect();
      artifactResizeBoundsRef.current = {
        right: rect.right,
        maxWidth: Math.max(
          AGENTS_ARTIFACT_MIN_WIDTH,
          rect.width - AGENTS_CHAT_MIN_WIDTH,
        ),
      };
    } else {
      artifactResizeBoundsRef.current = null;
    }
    pendingArtifactWidthRef.current = null;
    setIsArtifactResizing(true);
  }, []);

  const handleArtifactResizeReset = useCallback((event: ReactMouseEvent) => {
    event.preventDefault();
    if (artifactResizeFrameRef.current !== null) {
      window.cancelAnimationFrame(artifactResizeFrameRef.current);
      artifactResizeFrameRef.current = null;
    }
    pendingArtifactWidthRef.current = null;
    artifactResizeBoundsRef.current = null;
    setArtifactPanelWidth(null);
  }, []);

  const flushPendingArtifactWidth = useCallback(() => {
    if (artifactResizeFrameRef.current !== null) {
      window.cancelAnimationFrame(artifactResizeFrameRef.current);
      artifactResizeFrameRef.current = null;
    }
    const pendingWidth = pendingArtifactWidthRef.current;
    pendingArtifactWidthRef.current = null;
    if (pendingWidth !== null) {
      setArtifactPanelWidth(pendingWidth);
    }
  }, []);

  const scheduleArtifactWidth = useCallback((nextWidth: number) => {
    pendingArtifactWidthRef.current = nextWidth;
    if (artifactResizeFrameRef.current !== null) {
      return;
    }
    artifactResizeFrameRef.current = window.requestAnimationFrame(() => {
      artifactResizeFrameRef.current = null;
      const pendingWidth = pendingArtifactWidthRef.current;
      pendingArtifactWidthRef.current = null;
      if (pendingWidth !== null) {
        setArtifactPanelWidth(pendingWidth);
      }
    });
  }, []);

  useEffect(
    () => () => {
      if (artifactResizeFrameRef.current !== null) {
        window.cancelAnimationFrame(artifactResizeFrameRef.current);
      }
    },
    [],
  );

  useEffect(() => {
    if (!isArtifactResizing) {
      return;
    }

    const handleMouseMove = (event: MouseEvent) => {
      const container = splitContainerRef.current;
      if (!container) {
        return;
      }
      const bounds =
        artifactResizeBoundsRef.current ??
        (() => {
          const rect = container.getBoundingClientRect();
          const nextBounds = {
            right: rect.right,
            maxWidth: Math.max(
              AGENTS_ARTIFACT_MIN_WIDTH,
              rect.width - AGENTS_CHAT_MIN_WIDTH,
            ),
          };
          artifactResizeBoundsRef.current = nextBounds;
          return nextBounds;
        })();
      const nextWidth = bounds.right - event.clientX;
      scheduleArtifactWidth(
        Math.max(AGENTS_ARTIFACT_MIN_WIDTH, Math.min(bounds.maxWidth, nextWidth)),
      );
    };

    const handleMouseUp = () => {
      flushPendingArtifactWidth();
      artifactResizeBoundsRef.current = null;
      setIsArtifactResizing(false);
    };

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);

    return () => {
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };
  }, [flushPendingArtifactWidth, isArtifactResizing, scheduleArtifactWidth]);

  useEffect(() => {
    if (artifactPanelWidth !== null) {
      window.localStorage.setItem(AGENTS_ARTIFACT_WIDTH_STORAGE_KEY, String(artifactPanelWidth));
      return;
    }
    window.localStorage.removeItem(AGENTS_ARTIFACT_WIDTH_STORAGE_KEY);
  }, [artifactPanelWidth]);
  return {
    artifactWidthCss,
    handleArtifactResizeReset,
    handleArtifactResizeStart,
    isArtifactResizing,
    splitContainerRef,
  };
}
