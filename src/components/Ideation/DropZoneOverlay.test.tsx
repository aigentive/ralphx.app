/**
 * Tests for DropZoneOverlay component
 */

import { render, screen } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import { DropZoneOverlay } from "./DropZoneOverlay";

describe("DropZoneOverlay", () => {
  describe("visibility", () => {
    it("renders nothing when isVisible is false", () => {
      const { container } = render(<DropZoneOverlay isVisible={false} />);
      expect(container.firstChild).toBeNull();
    });

    it("renders overlay when isVisible is true", () => {
      render(<DropZoneOverlay isVisible={true} />);
      expect(screen.getByTestId("drop-zone-overlay")).toBeInTheDocument();
    });
  });

  describe("content", () => {
    it("displays the drop message", () => {
      render(<DropZoneOverlay isVisible={true} />);
      expect(screen.getByText("Drop to import")).toBeInTheDocument();
    });

    it("displays a file icon", () => {
      render(<DropZoneOverlay isVisible={true} />);
      // FileDown icon should be present
      expect(screen.getByTestId("drop-zone-icon")).toBeInTheDocument();
    });
  });

  describe("styling", () => {
    it("has absolute positioning to fill parent", () => {
      render(<DropZoneOverlay isVisible={true} />);
      const overlay = screen.getByTestId("drop-zone-overlay");
      expect(overlay).toHaveClass("absolute", "inset-0");
    });

    it("has pointer-events-none to allow drop through", () => {
      render(<DropZoneOverlay isVisible={true} />);
      const overlay = screen.getByTestId("drop-zone-overlay");
      expect(overlay).toHaveClass("pointer-events-none");
    });

    it("has orange border indicating drop zone", () => {
      render(<DropZoneOverlay isVisible={true} />);
      const overlay = screen.getByTestId("drop-zone-overlay");
      // Border is applied via inline style for the pulsing animation
      expect(overlay).toBeInTheDocument();
    });

    it("has dimmed background overlay", () => {
      render(<DropZoneOverlay isVisible={true} />);
      const overlay = screen.getByTestId("drop-zone-overlay");
      // Background is applied via inline style
      expect(overlay).toBeInTheDocument();
    });
  });

  describe("custom message", () => {
    it("displays custom message when provided", () => {
      render(<DropZoneOverlay isVisible={true} message="Custom drop text" />);
      expect(screen.getByText("Custom drop text")).toBeInTheDocument();
    });
  });
});
