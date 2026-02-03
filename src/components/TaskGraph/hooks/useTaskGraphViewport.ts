import type { Node } from "@xyflow/react";
import { useReactFlow } from "@xyflow/react";

export interface FitNodeOptions {
  duration?: number;
  padding?: number;
  maxZoom?: number;
}

export interface CenterNodeOptions {
  duration?: number;
  zoom?: number;
  fallbackWidth?: number;
  fallbackHeight?: number;
}

interface ViewportApi {
  getNodes: () => Node[];
  fitView: (options: { nodes?: Node[]; duration?: number; padding?: number; maxZoom?: number }) => void;
  setCenter: (x: number, y: number, options?: { duration?: number; zoom?: number }) => void;
  getViewport: () => { x: number; y: number; zoom: number };
  setViewport: (viewport: { x: number; y: number; zoom: number }, options?: { duration?: number }) => void;
}

export function resolveNodeDimensions(
  node: Node,
  fallback: { width: number; height: number }
): { width: number; height: number } {
  const measuredWidth = node.measured?.width;
  const measuredHeight = node.measured?.height;
  if (typeof measuredWidth === "number" && typeof measuredHeight === "number") {
    return { width: measuredWidth, height: measuredHeight };
  }

  const nodeWidth = typeof node.width === "number" ? node.width : undefined;
  const nodeHeight = typeof node.height === "number" ? node.height : undefined;
  if (typeof nodeWidth === "number" && typeof nodeHeight === "number") {
    return { width: nodeWidth, height: nodeHeight };
  }

  const data = node.data as { width?: number; height?: number } | undefined;
  const dataWidth = typeof data?.width === "number" ? data.width : undefined;
  const dataHeight = typeof data?.height === "number" ? data.height : undefined;
  if (typeof dataWidth === "number" && typeof dataHeight === "number") {
    return { width: dataWidth, height: dataHeight };
  }

  return fallback;
}

function createNodeCenter(
  node: Node,
  fallback: { width: number; height: number }
): { x: number; y: number } {
  const { width, height } = resolveNodeDimensions(node, fallback);
  return {
    x: node.position.x + width / 2,
    y: node.position.y + height / 2,
  };
}

export function createViewportActions({
  getNodes,
  fitView,
  setCenter,
  getViewport,
  setViewport,
}: ViewportApi) {
  const fitNodeInView = (
    nodeId: string,
    { duration = 220, padding = 0.18, maxZoom = 0.95 }: FitNodeOptions = {}
  ): boolean => {
    const node = getNodes().find((item) => item.id === nodeId);
    if (!node) return false;
    fitView({ nodes: [node], duration, padding, maxZoom });
    return true;
  };

  const centerOnNode = (
    nodeId: string,
    {
      duration = 300,
      zoom = 1.2,
      fallbackWidth = 180,
      fallbackHeight = 60,
    }: CenterNodeOptions = {}
  ): boolean => {
    const node = getNodes().find((item) => item.id === nodeId);
    if (!node) return false;
    const center = createNodeCenter(node, { width: fallbackWidth, height: fallbackHeight });
    setCenter(center.x, center.y, { duration, zoom });
    return true;
  };

  const fitNode = (
    node: Node,
    { duration = 220, padding = 0.18, maxZoom = 0.95 }: FitNodeOptions = {}
  ): void => {
    fitView({ nodes: [node], duration, padding, maxZoom });
  };

  const centerOnNodeObject = (
    node: Node,
    {
      duration = 300,
      zoom = 1.2,
      fallbackWidth = 180,
      fallbackHeight = 60,
    }: CenterNodeOptions = {}
  ): void => {
    const center = createNodeCenter(node, { width: fallbackWidth, height: fallbackHeight });
    setCenter(center.x, center.y, { duration, zoom });
  };

  const fitViewDefault = (options: { duration?: number; padding?: number } = {}): void => {
    fitView({ padding: options.padding ?? 0.2, duration: options.duration ?? 200 });
  };

  const zoomBy = (
    delta: number,
    {
      duration = 120,
      minZoom = 0.6,
      maxZoom = 1,
    }: { duration?: number; minZoom?: number; maxZoom?: number } = {}
  ): boolean => {
    const viewport = getViewport();
    const nextZoom = Math.min(maxZoom, Math.max(minZoom, viewport.zoom + delta));
    if (nextZoom === viewport.zoom) return false;
    setViewport({ ...viewport, zoom: nextZoom }, { duration });
    return true;
  };

  return { fitNodeInView, fitNode, centerOnNode, centerOnNodeObject, fitViewDefault, zoomBy };
}

export function useTaskGraphViewport() {
  const { getNodes, fitView, setCenter, getViewport, setViewport } = useReactFlow();
  return createViewportActions({ getNodes, fitView, setCenter, getViewport, setViewport });
}
