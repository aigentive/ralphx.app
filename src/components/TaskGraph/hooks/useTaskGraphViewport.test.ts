import { describe, it, expect, vi } from "vitest";
import type { Node } from "@xyflow/react";
import { createViewportActions, resolveNodeDimensions } from "./useTaskGraphViewport";

function makeNode(overrides: Partial<Node> = {}): Node {
  return {
    id: overrides.id ?? "node-1",
    type: overrides.type ?? "task",
    position: overrides.position ?? { x: 0, y: 0 },
    data: overrides.data ?? {},
    ...overrides,
  } as Node;
}

describe("resolveNodeDimensions", () => {
  it("prefers measured dimensions over other sources", () => {
    const node = makeNode({
      measured: { width: 200, height: 120 },
      width: 180,
      height: 90,
      data: { width: 160, height: 80 },
    });

    expect(resolveNodeDimensions(node, { width: 140, height: 70 })).toEqual({
      width: 200,
      height: 120,
    });
  });

  it("falls back to node width/height, then data, then fallback", () => {
    const node = makeNode({ width: 180, height: 90 });
    expect(resolveNodeDimensions(node, { width: 140, height: 70 })).toEqual({
      width: 180,
      height: 90,
    });

    const dataNode = makeNode({ width: undefined, height: undefined, data: { width: 150, height: 75 } });
    expect(resolveNodeDimensions(dataNode, { width: 140, height: 70 })).toEqual({
      width: 150,
      height: 75,
    });

    const fallbackNode = makeNode({ width: undefined, height: undefined, data: {} });
    expect(resolveNodeDimensions(fallbackNode, { width: 140, height: 70 })).toEqual({
      width: 140,
      height: 70,
    });
  });
});

describe("createViewportActions", () => {
  it("fits a node into view when it exists", () => {
    const fitView = vi.fn();
    const setCenter = vi.fn();
    const node = makeNode({ id: "node-1" });
    const actions = createViewportActions({
      fitView,
      setCenter,
      getNodes: () => [node],
      getViewport: () => ({ x: 0, y: 0, zoom: 1 }),
      setViewport: vi.fn(),
    });

    expect(actions.fitNodeInView("node-1")).toBe(true);
    expect(fitView).toHaveBeenCalledWith({
      nodes: [node],
      duration: 220,
      padding: 0.18,
      maxZoom: 0.95,
    });
  });

  it("returns false when fitting a missing node", () => {
    const fitView = vi.fn();
    const actions = createViewportActions({
      fitView,
      setCenter: vi.fn(),
      getNodes: () => [],
      getViewport: () => ({ x: 0, y: 0, zoom: 1 }),
      setViewport: vi.fn(),
    });

    expect(actions.fitNodeInView("missing")).toBe(false);
    expect(fitView).not.toHaveBeenCalled();
  });

  it("centers on a node using resolved dimensions", () => {
    const setCenter = vi.fn();
    const node = makeNode({
      id: "node-1",
      position: { x: 10, y: 20 },
      measured: { width: 100, height: 80 },
    });

    const actions = createViewportActions({
      fitView: vi.fn(),
      setCenter,
      getNodes: () => [node],
      getViewport: () => ({ x: 0, y: 0, zoom: 1 }),
      setViewport: vi.fn(),
    });

    expect(
      actions.centerOnNode("node-1", {
        duration: 300,
        zoom: 1.2,
        fallbackWidth: 180,
        fallbackHeight: 60,
      })
    ).toBe(true);

    expect(setCenter).toHaveBeenCalledWith(60, 60, { duration: 300, zoom: 1.2 });
  });

  it("zooms by delta within bounds", () => {
    const setViewport = vi.fn();
    const actions = createViewportActions({
      fitView: vi.fn(),
      setCenter: vi.fn(),
      getNodes: () => [],
      getViewport: () => ({ x: 12, y: 34, zoom: 0.92 }),
      setViewport,
    });

    expect(actions.zoomBy(0.2, { minZoom: 0.6, maxZoom: 1, duration: 120 })).toBe(true);
    expect(setViewport).toHaveBeenCalledWith({ x: 12, y: 34, zoom: 1 }, { duration: 120 });
  });
});
