/**
 * Tests for @dnd-kit package integration
 *
 * Verifies that the drag-and-drop library is properly installed
 * and can be imported.
 */

import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { DndContext, useDroppable, useDraggable } from "@dnd-kit/core";
import {
  SortableContext,
  useSortable,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";

describe("@dnd-kit integration", () => {
  it("should import DndContext from @dnd-kit/core", () => {
    expect(DndContext).toBeDefined();
    expect(typeof DndContext).toBe("object"); // React component
  });

  it("should import useDroppable hook", () => {
    expect(useDroppable).toBeDefined();
    expect(typeof useDroppable).toBe("function");
  });

  it("should import useDraggable hook", () => {
    expect(useDraggable).toBeDefined();
    expect(typeof useDraggable).toBe("function");
  });

  it("should import SortableContext from @dnd-kit/sortable", () => {
    expect(SortableContext).toBeDefined();
  });

  it("should import useSortable hook", () => {
    expect(useSortable).toBeDefined();
    expect(typeof useSortable).toBe("function");
  });

  it("should import verticalListSortingStrategy", () => {
    expect(verticalListSortingStrategy).toBeDefined();
    expect(typeof verticalListSortingStrategy).toBe("function");
  });

  it("should import CSS from @dnd-kit/utilities", () => {
    expect(CSS).toBeDefined();
    expect(typeof CSS.Transform).toBe("object");
  });

  it("should render DndContext wrapper", () => {
    render(
      <DndContext>
        <div data-testid="dnd-child">Content</div>
      </DndContext>
    );

    expect(screen.getByTestId("dnd-child")).toBeInTheDocument();
  });
});
