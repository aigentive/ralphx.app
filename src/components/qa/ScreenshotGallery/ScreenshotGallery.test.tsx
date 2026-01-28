import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, fireEvent, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import {
  ScreenshotGallery,
  type Screenshot,
} from "./ScreenshotGallery";
import { pathsToScreenshots } from "./utils";
import type { QAStepResult } from "@/types/qa";

// ============================================================================
// Test Data
// ============================================================================

const mockScreenshots: Screenshot[] = [
  {
    id: "shot-1",
    path: "/screenshots/test-1.png",
    label: "QA1",
    timestamp: "2026-01-25T10:00:00Z",
  },
  {
    id: "shot-2",
    path: "/screenshots/test-2.png",
    label: "QA2",
    timestamp: "2026-01-25T10:01:00Z",
  },
  {
    id: "shot-3",
    path: "/screenshots/test-3.png",
    label: "QA3",
    timestamp: "2026-01-25T10:02:00Z",
  },
];

const mockFailedScreenshot: Screenshot = {
  id: "shot-failed",
  path: "/screenshots/failed.png",
  label: "QA-FAILED",
  stepResult: {
    step_id: "QA-FAILED",
    status: "failed",
    screenshot: "/screenshots/failed.png",
    expected: "Button should be visible",
    actual: "Button not found",
    error: "Element not found in DOM",
  },
};

const mockPassedScreenshot: Screenshot = {
  id: "shot-passed",
  path: "/screenshots/passed.png",
  label: "QA-PASSED",
  stepResult: {
    step_id: "QA-PASSED",
    status: "passed",
    screenshot: "/screenshots/passed.png",
  },
};

const mockComparisonScreenshot: Screenshot = {
  id: "shot-comparison",
  path: "/screenshots/actual.png",
  label: "QA-COMPARE",
  expectedPath: "/screenshots/expected.png",
  stepResult: {
    step_id: "QA-COMPARE",
    status: "failed",
    expected: "Expected text",
    actual: "Actual text",
  },
};

// ============================================================================
// Setup
// ============================================================================

describe("ScreenshotGallery", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  // ==========================================================================
  // Empty State Tests
  // ==========================================================================

  describe("Empty State", () => {
    it("shows empty state when no screenshots provided", () => {
      render(<ScreenshotGallery screenshots={[]} />);

      expect(screen.getByTestId("screenshot-gallery-empty")).toBeInTheDocument();
      expect(screen.getByText("No screenshots captured")).toBeInTheDocument();
    });

    it("shows custom empty message when provided", () => {
      render(
        <ScreenshotGallery
          screenshots={[]}
          emptyMessage="No visual verification results"
        />
      );

      expect(
        screen.getByText("No visual verification results")
      ).toBeInTheDocument();
    });

    it("shows helper text in empty state", () => {
      render(<ScreenshotGallery screenshots={[]} />);

      expect(
        screen.getByText(/Screenshots will appear here/)
      ).toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Thumbnail Grid Tests
  // ==========================================================================

  describe("Thumbnail Grid", () => {
    it("renders gallery with screenshots", () => {
      render(<ScreenshotGallery screenshots={mockScreenshots} />);

      expect(screen.getByTestId("screenshot-gallery")).toBeInTheDocument();
    });

    it("renders correct number of thumbnails", () => {
      render(<ScreenshotGallery screenshots={mockScreenshots} />);

      expect(screen.getByTestId("screenshot-thumbnail-0")).toBeInTheDocument();
      expect(screen.getByTestId("screenshot-thumbnail-1")).toBeInTheDocument();
      expect(screen.getByTestId("screenshot-thumbnail-2")).toBeInTheDocument();
    });

    it("displays thumbnail images with correct src", () => {
      render(<ScreenshotGallery screenshots={mockScreenshots} />);

      const thumbnail0 = screen.getByTestId("screenshot-thumbnail-0");
      const img = thumbnail0.querySelector("img");
      expect(img).toHaveAttribute("src", "/screenshots/test-1.png");
    });

    it("uses default 3 columns", () => {
      render(<ScreenshotGallery screenshots={mockScreenshots} />);

      const gallery = screen.getByTestId("screenshot-gallery");
      const grid = gallery.querySelector(".grid");
      expect(grid).toHaveClass("grid-cols-3");
    });

    it("supports 2 column layout", () => {
      render(<ScreenshotGallery screenshots={mockScreenshots} columns={2} />);

      const gallery = screen.getByTestId("screenshot-gallery");
      const grid = gallery.querySelector(".grid");
      expect(grid).toHaveClass("grid-cols-2");
    });

    it("supports 4 column layout", () => {
      render(<ScreenshotGallery screenshots={mockScreenshots} columns={4} />);

      const gallery = screen.getByTestId("screenshot-gallery");
      const grid = gallery.querySelector(".grid");
      expect(grid).toHaveClass("grid-cols-4");
    });

    it("applies custom className", () => {
      render(
        <ScreenshotGallery
          screenshots={mockScreenshots}
          className="custom-class"
        />
      );

      expect(screen.getByTestId("screenshot-gallery")).toHaveClass(
        "custom-class"
      );
    });

    it("shows failed indicator on failed screenshots", () => {
      render(
        <ScreenshotGallery screenshots={[mockFailedScreenshot, ...mockScreenshots]} />
      );

      expect(
        screen.getByTestId("screenshot-failed-indicator-0")
      ).toBeInTheDocument();
    });

    it("shows passed indicator on passed screenshots", () => {
      render(
        <ScreenshotGallery screenshots={[mockPassedScreenshot, ...mockScreenshots]} />
      );

      expect(
        screen.getByTestId("screenshot-passed-indicator-0")
      ).toBeInTheDocument();
    });

    it("shows comparison indicator when expectedPath is present", () => {
      render(
        <ScreenshotGallery
          screenshots={[mockComparisonScreenshot, ...mockScreenshots]}
        />
      );

      expect(
        screen.getByTestId("screenshot-comparison-indicator-0")
      ).toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Lightbox Tests
  // ==========================================================================

  describe("Lightbox", () => {
    it("opens lightbox when thumbnail is clicked", async () => {
      const user = userEvent.setup();
      render(<ScreenshotGallery screenshots={mockScreenshots} />);

      await user.click(screen.getByTestId("screenshot-thumbnail-0"));

      expect(screen.getByTestId("screenshot-lightbox")).toBeInTheDocument();
    });

    it("displays correct image in lightbox", async () => {
      const user = userEvent.setup();
      render(<ScreenshotGallery screenshots={mockScreenshots} />);

      await user.click(screen.getByTestId("screenshot-thumbnail-1"));

      const lightbox = screen.getByTestId("screenshot-lightbox");
      const mainImg = lightbox.querySelector('img[alt="QA2"]');
      expect(mainImg).toHaveAttribute("src", "/screenshots/test-2.png");
    });

    it("shows filename in lightbox header", async () => {
      const user = userEvent.setup();
      render(<ScreenshotGallery screenshots={mockScreenshots} />);

      await user.click(screen.getByTestId("screenshot-thumbnail-0"));

      expect(screen.getByTestId("lightbox-filename")).toHaveTextContent("QA1");
    });

    it("shows counter in lightbox", async () => {
      const user = userEvent.setup();
      render(<ScreenshotGallery screenshots={mockScreenshots} />);

      await user.click(screen.getByTestId("screenshot-thumbnail-0"));

      expect(screen.getByTestId("lightbox-counter")).toHaveTextContent(
        "1 / 3"
      );
    });

    it("closes lightbox when close button is clicked", async () => {
      const user = userEvent.setup();
      render(<ScreenshotGallery screenshots={mockScreenshots} />);

      await user.click(screen.getByTestId("screenshot-thumbnail-0"));
      expect(screen.getByTestId("screenshot-lightbox")).toBeInTheDocument();

      await user.click(screen.getByTestId("lightbox-close"));
      expect(
        screen.queryByTestId("screenshot-lightbox")
      ).not.toBeInTheDocument();
    });

    it("closes lightbox on Escape key", async () => {
      const user = userEvent.setup();
      render(<ScreenshotGallery screenshots={mockScreenshots} />);

      await user.click(screen.getByTestId("screenshot-thumbnail-0"));
      expect(screen.getByTestId("screenshot-lightbox")).toBeInTheDocument();

      await user.keyboard("{Escape}");
      expect(
        screen.queryByTestId("screenshot-lightbox")
      ).not.toBeInTheDocument();
    });

    it("calls onOpen callback when lightbox opens", async () => {
      const onOpen = vi.fn();
      const user = userEvent.setup();
      render(<ScreenshotGallery screenshots={mockScreenshots} onOpen={onOpen} />);

      await user.click(screen.getByTestId("screenshot-thumbnail-1"));

      expect(onOpen).toHaveBeenCalledWith(1);
    });

    it("calls onClose callback when lightbox closes", async () => {
      const onClose = vi.fn();
      const user = userEvent.setup();
      render(<ScreenshotGallery screenshots={mockScreenshots} onClose={onClose} />);

      await user.click(screen.getByTestId("screenshot-thumbnail-0"));
      await user.click(screen.getByTestId("lightbox-close"));

      expect(onClose).toHaveBeenCalled();
    });

    it("opens lightbox at initialIndex", () => {
      render(<ScreenshotGallery screenshots={mockScreenshots} initialIndex={2} />);

      expect(screen.getByTestId("screenshot-lightbox")).toBeInTheDocument();
      expect(screen.getByTestId("lightbox-counter")).toHaveTextContent("3 / 3");
    });
  });

  // ==========================================================================
  // Navigation Tests
  // ==========================================================================

  describe("Navigation", () => {
    it("navigates to next image with button", async () => {
      const user = userEvent.setup();
      render(<ScreenshotGallery screenshots={mockScreenshots} />);

      await user.click(screen.getByTestId("screenshot-thumbnail-0"));
      expect(screen.getByTestId("lightbox-counter")).toHaveTextContent("1 / 3");

      await user.click(screen.getByTestId("lightbox-next"));
      expect(screen.getByTestId("lightbox-counter")).toHaveTextContent("2 / 3");
    });

    it("navigates to previous image with button", async () => {
      const user = userEvent.setup();
      render(<ScreenshotGallery screenshots={mockScreenshots} />);

      await user.click(screen.getByTestId("screenshot-thumbnail-2"));
      expect(screen.getByTestId("lightbox-counter")).toHaveTextContent("3 / 3");

      await user.click(screen.getByTestId("lightbox-prev"));
      expect(screen.getByTestId("lightbox-counter")).toHaveTextContent("2 / 3");
    });

    it("disables prev button on first image", async () => {
      const user = userEvent.setup();
      render(<ScreenshotGallery screenshots={mockScreenshots} />);

      await user.click(screen.getByTestId("screenshot-thumbnail-0"));

      expect(screen.getByTestId("lightbox-prev")).toBeDisabled();
    });

    it("disables next button on last image", async () => {
      const user = userEvent.setup();
      render(<ScreenshotGallery screenshots={mockScreenshots} />);

      await user.click(screen.getByTestId("screenshot-thumbnail-2"));

      expect(screen.getByTestId("lightbox-next")).toBeDisabled();
    });

    it("navigates with ArrowRight key", async () => {
      const user = userEvent.setup();
      render(<ScreenshotGallery screenshots={mockScreenshots} />);

      await user.click(screen.getByTestId("screenshot-thumbnail-0"));
      await user.keyboard("{ArrowRight}");

      expect(screen.getByTestId("lightbox-counter")).toHaveTextContent("2 / 3");
    });

    it("navigates with ArrowLeft key", async () => {
      const user = userEvent.setup();
      render(<ScreenshotGallery screenshots={mockScreenshots} />);

      await user.click(screen.getByTestId("screenshot-thumbnail-2"));
      await user.keyboard("{ArrowLeft}");

      expect(screen.getByTestId("lightbox-counter")).toHaveTextContent("2 / 3");
    });

    it("navigates using thumbnail strip", async () => {
      const user = userEvent.setup();
      render(<ScreenshotGallery screenshots={mockScreenshots} />);

      await user.click(screen.getByTestId("screenshot-thumbnail-0"));
      await user.click(screen.getByTestId("lightbox-thumbnail-2"));

      expect(screen.getByTestId("lightbox-counter")).toHaveTextContent("3 / 3");
    });

    it("does not show navigation for single screenshot", async () => {
      const user = userEvent.setup();
      render(<ScreenshotGallery screenshots={[mockScreenshots[0]!]} />);

      await user.click(screen.getByTestId("screenshot-thumbnail-0"));

      expect(screen.queryByTestId("lightbox-prev")).not.toBeInTheDocument();
      expect(screen.queryByTestId("lightbox-next")).not.toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Zoom Tests
  // ==========================================================================

  describe("Zoom", () => {
    it("shows zoom controls in lightbox", async () => {
      const user = userEvent.setup();
      render(<ScreenshotGallery screenshots={mockScreenshots} />);

      await user.click(screen.getByTestId("screenshot-thumbnail-0"));

      expect(screen.getByTestId("lightbox-zoom-in")).toBeInTheDocument();
      expect(screen.getByTestId("lightbox-zoom-out")).toBeInTheDocument();
    });

    it("zooms in when zoom in button clicked", async () => {
      const user = userEvent.setup();
      render(<ScreenshotGallery screenshots={mockScreenshots} />);

      await user.click(screen.getByTestId("screenshot-thumbnail-0"));

      const zoomDisplay = screen.getByText("100%");
      expect(zoomDisplay).toBeInTheDocument();

      await user.click(screen.getByTestId("lightbox-zoom-in"));

      expect(screen.getByText("125%")).toBeInTheDocument();
    });

    it("zooms out when zoom out button clicked", async () => {
      const user = userEvent.setup();
      render(<ScreenshotGallery screenshots={mockScreenshots} />);

      await user.click(screen.getByTestId("screenshot-thumbnail-0"));
      await user.click(screen.getByTestId("lightbox-zoom-in")); // 125%
      await user.click(screen.getByTestId("lightbox-zoom-out")); // back to 100%

      expect(screen.getByText("100%")).toBeInTheDocument();
    });

    it("zooms in with + key", async () => {
      const user = userEvent.setup();
      render(<ScreenshotGallery screenshots={mockScreenshots} />);

      await user.click(screen.getByTestId("screenshot-thumbnail-0"));
      await user.keyboard("{+}");

      expect(screen.getByText("125%")).toBeInTheDocument();
    });

    it("zooms out with - key", async () => {
      const user = userEvent.setup();
      render(<ScreenshotGallery screenshots={mockScreenshots} />);

      await user.click(screen.getByTestId("screenshot-thumbnail-0"));
      await user.keyboard("{+}");
      await user.keyboard("{-}");

      expect(screen.getByText("100%")).toBeInTheDocument();
    });

    it("resets zoom with 0 key", async () => {
      const user = userEvent.setup();
      render(<ScreenshotGallery screenshots={mockScreenshots} />);

      await user.click(screen.getByTestId("screenshot-thumbnail-0"));
      await user.keyboard("{+}");
      await user.keyboard("{+}");
      await user.keyboard("{0}");

      expect(screen.getByText("100%")).toBeInTheDocument();
    });

    it("disables zoom out at minimum zoom", async () => {
      const user = userEvent.setup();
      render(<ScreenshotGallery screenshots={mockScreenshots} />);

      await user.click(screen.getByTestId("screenshot-thumbnail-0"));
      // Zoom out twice to reach 50%
      await user.keyboard("{-}");
      await user.keyboard("{-}");

      expect(screen.getByTestId("lightbox-zoom-out")).toBeDisabled();
    });

    it("disables zoom in at maximum zoom", async () => {
      const user = userEvent.setup();
      render(<ScreenshotGallery screenshots={mockScreenshots} />);

      await user.click(screen.getByTestId("screenshot-thumbnail-0"));
      // Zoom in to max (400%)
      for (let i = 0; i < 12; i++) {
        await user.keyboard("{+}");
      }

      expect(screen.getByTestId("lightbox-zoom-in")).toBeDisabled();
    });
  });

  // ==========================================================================
  // Comparison Mode Tests
  // ==========================================================================

  describe("Comparison Mode", () => {
    it("shows comparison toggle for screenshots with expectedPath", async () => {
      const user = userEvent.setup();
      render(
        <ScreenshotGallery screenshots={[mockComparisonScreenshot]} />
      );

      await user.click(screen.getByTestId("screenshot-thumbnail-0"));

      expect(
        screen.getByTestId("lightbox-toggle-comparison")
      ).toBeInTheDocument();
    });

    it("shows comparison toggle for failed screenshots with stepResult", async () => {
      const user = userEvent.setup();
      render(<ScreenshotGallery screenshots={[mockFailedScreenshot]} />);

      await user.click(screen.getByTestId("screenshot-thumbnail-0"));

      expect(
        screen.getByTestId("lightbox-toggle-comparison")
      ).toBeInTheDocument();
    });

    it("does not show comparison toggle for regular screenshots", async () => {
      const user = userEvent.setup();
      render(<ScreenshotGallery screenshots={mockScreenshots} />);

      await user.click(screen.getByTestId("screenshot-thumbnail-0"));

      expect(
        screen.queryByTestId("lightbox-toggle-comparison")
      ).not.toBeInTheDocument();
    });

    it("toggles to comparison view when button clicked", async () => {
      const user = userEvent.setup();
      render(
        <ScreenshotGallery screenshots={[mockComparisonScreenshot]} />
      );

      await user.click(screen.getByTestId("screenshot-thumbnail-0"));
      await user.click(screen.getByTestId("lightbox-toggle-comparison"));

      expect(screen.getByTestId("comparison-view")).toBeInTheDocument();
    });

    it("toggles comparison mode with c key", async () => {
      const user = userEvent.setup();
      render(
        <ScreenshotGallery screenshots={[mockComparisonScreenshot]} />
      );

      await user.click(screen.getByTestId("screenshot-thumbnail-0"));
      await user.keyboard("{c}");

      expect(screen.getByTestId("comparison-view")).toBeInTheDocument();
    });

    it("shows Expected and Actual panels in comparison view", async () => {
      const user = userEvent.setup();
      render(
        <ScreenshotGallery screenshots={[mockComparisonScreenshot]} />
      );

      await user.click(screen.getByTestId("screenshot-thumbnail-0"));
      await user.click(screen.getByTestId("lightbox-toggle-comparison"));

      expect(screen.getByText("Expected")).toBeInTheDocument();
      expect(screen.getByText("Actual")).toBeInTheDocument();
    });

    it("shows expected image when expectedPath is provided", async () => {
      const user = userEvent.setup();
      render(
        <ScreenshotGallery screenshots={[mockComparisonScreenshot]} />
      );

      await user.click(screen.getByTestId("screenshot-thumbnail-0"));
      await user.click(screen.getByTestId("lightbox-toggle-comparison"));

      expect(
        screen.getByTestId("comparison-expected-image")
      ).toHaveAttribute("src", "/screenshots/expected.png");
    });

    it("shows actual image in comparison view", async () => {
      const user = userEvent.setup();
      render(
        <ScreenshotGallery screenshots={[mockComparisonScreenshot]} />
      );

      await user.click(screen.getByTestId("screenshot-thumbnail-0"));
      await user.click(screen.getByTestId("lightbox-toggle-comparison"));

      expect(screen.getByTestId("comparison-actual-image")).toHaveAttribute(
        "src",
        "/screenshots/actual.png"
      );
    });

    it("shows placeholder when no expected image available", async () => {
      const user = userEvent.setup();
      render(<ScreenshotGallery screenshots={[mockFailedScreenshot]} />);

      await user.click(screen.getByTestId("screenshot-thumbnail-0"));
      await user.click(screen.getByTestId("lightbox-toggle-comparison"));

      expect(screen.getByText("No expected screenshot")).toBeInTheDocument();
    });

    it("shows expected/actual text values in comparison view", async () => {
      const user = userEvent.setup();
      render(
        <ScreenshotGallery screenshots={[mockComparisonScreenshot]} />
      );

      await user.click(screen.getByTestId("screenshot-thumbnail-0"));
      await user.click(screen.getByTestId("lightbox-toggle-comparison"));

      // Text appears in comparison view header
      const comparisonView = screen.getByTestId("comparison-view");
      expect(within(comparisonView).getByText("Expected text")).toBeInTheDocument();
      expect(within(comparisonView).getByText("Actual text")).toBeInTheDocument();
    });

    it("toggles back to single view", async () => {
      const user = userEvent.setup();
      render(
        <ScreenshotGallery screenshots={[mockComparisonScreenshot]} />
      );

      await user.click(screen.getByTestId("screenshot-thumbnail-0"));
      await user.click(screen.getByTestId("lightbox-toggle-comparison"));
      expect(screen.getByTestId("comparison-view")).toBeInTheDocument();

      await user.click(screen.getByTestId("lightbox-toggle-comparison"));
      expect(screen.queryByTestId("comparison-view")).not.toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Failure Details Tests
  // ==========================================================================

  describe("Failure Details", () => {
    it("shows failure details footer for failed screenshots", async () => {
      const user = userEvent.setup();
      render(<ScreenshotGallery screenshots={[mockFailedScreenshot]} />);

      await user.click(screen.getByTestId("screenshot-thumbnail-0"));

      expect(
        screen.getByTestId("lightbox-failure-details")
      ).toBeInTheDocument();
    });

    it("displays error message in failure details", async () => {
      const user = userEvent.setup();
      render(<ScreenshotGallery screenshots={[mockFailedScreenshot]} />);

      await user.click(screen.getByTestId("screenshot-thumbnail-0"));

      expect(screen.getByText(/Element not found in DOM/)).toBeInTheDocument();
    });

    it("displays expected/actual values in failure details", async () => {
      const user = userEvent.setup();
      render(<ScreenshotGallery screenshots={[mockFailedScreenshot]} />);

      await user.click(screen.getByTestId("screenshot-thumbnail-0"));

      expect(screen.getByText("Button should be visible")).toBeInTheDocument();
      expect(screen.getByText("Button not found")).toBeInTheDocument();
    });

    it("does not show failure details for passed screenshots", async () => {
      const user = userEvent.setup();
      render(<ScreenshotGallery screenshots={[mockPassedScreenshot]} />);

      await user.click(screen.getByTestId("screenshot-thumbnail-0"));

      expect(
        screen.queryByTestId("lightbox-failure-details")
      ).not.toBeInTheDocument();
    });

    it("shows Failed badge in header for failed screenshots", async () => {
      const user = userEvent.setup();
      render(<ScreenshotGallery screenshots={[mockFailedScreenshot]} />);

      await user.click(screen.getByTestId("screenshot-thumbnail-0"));

      expect(screen.getByText("Failed")).toBeInTheDocument();
    });

    it("shows Passed badge in header for passed screenshots", async () => {
      const user = userEvent.setup();
      render(<ScreenshotGallery screenshots={[mockPassedScreenshot]} />);

      await user.click(screen.getByTestId("screenshot-thumbnail-0"));

      expect(screen.getByText("Passed")).toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Thumbnail Strip Tests
  // ==========================================================================

  describe("Thumbnail Strip", () => {
    it("shows thumbnail strip for multiple screenshots", async () => {
      const user = userEvent.setup();
      render(<ScreenshotGallery screenshots={mockScreenshots} />);

      await user.click(screen.getByTestId("screenshot-thumbnail-0"));

      expect(screen.getByTestId("lightbox-thumbnail-0")).toBeInTheDocument();
      expect(screen.getByTestId("lightbox-thumbnail-1")).toBeInTheDocument();
      expect(screen.getByTestId("lightbox-thumbnail-2")).toBeInTheDocument();
    });

    it("does not show thumbnail strip for single screenshot", async () => {
      const user = userEvent.setup();
      render(<ScreenshotGallery screenshots={[mockScreenshots[0]!]} />);

      await user.click(screen.getByTestId("screenshot-thumbnail-0"));

      expect(
        screen.queryByTestId("lightbox-thumbnail-0")
      ).not.toBeInTheDocument();
    });

    it("highlights current thumbnail in strip", async () => {
      const user = userEvent.setup();
      render(<ScreenshotGallery screenshots={mockScreenshots} />);

      await user.click(screen.getByTestId("screenshot-thumbnail-1"));

      const thumbnail1 = screen.getByTestId("lightbox-thumbnail-1");
      expect(thumbnail1).toHaveClass("ring-2");
    });
  });

  // ==========================================================================
  // pathsToScreenshots Utility Tests
  // ==========================================================================

  describe("pathsToScreenshots", () => {
    it("converts paths to screenshot objects", () => {
      const paths = ["/screenshots/a.png", "/screenshots/b.png"];
      const result = pathsToScreenshots(paths);

      expect(result).toHaveLength(2);
      expect(result[0]).toMatchObject({
        id: "screenshot-0",
        path: "/screenshots/a.png",
        label: "a.png",
      });
      expect(result[1]).toMatchObject({
        id: "screenshot-1",
        path: "/screenshots/b.png",
        label: "b.png",
      });
    });

    it("matches step results to screenshots", () => {
      const paths = ["/screenshots/qa1.png"];
      const stepResults = new Map<string, QAStepResult>([
        [
          "QA1",
          {
            step_id: "QA1",
            status: "failed",
            screenshot: "/screenshots/qa1.png",
            error: "Test failed",
          },
        ],
      ]);

      const result = pathsToScreenshots(paths, stepResults);

      expect(result[0]).toMatchObject({
        label: "QA1",
        stepResult: {
          step_id: "QA1",
          status: "failed",
        },
      });
    });

    it("uses filename as label when no step result matches", () => {
      const paths = ["/screenshots/unmatched.png"];
      const stepResults = new Map<string, QAStepResult>();

      const result = pathsToScreenshots(paths, stepResults);

      expect(result[0]?.label).toBe("unmatched.png");
    });

    it("handles empty paths array", () => {
      const result = pathsToScreenshots([]);
      expect(result).toHaveLength(0);
    });
  });

  // ==========================================================================
  // Image Error Handling Tests
  // ==========================================================================

  describe("Image Error Handling", () => {
    it("shows placeholder when thumbnail image fails to load", async () => {
      render(<ScreenshotGallery screenshots={mockScreenshots} />);

      const thumbnail = screen.getByTestId("screenshot-thumbnail-0");
      const img = thumbnail.querySelector("img");

      // Before error, image should be present
      expect(img).toBeInTheDocument();

      // Simulate image error
      fireEvent.error(img!);

      // After error, image should be replaced with placeholder (img element removed)
      expect(thumbnail.querySelector("img")).not.toBeInTheDocument();
      // Placeholder SVG should be visible
      expect(thumbnail.querySelector("svg")).toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Accessibility Tests
  // ==========================================================================

  describe("Accessibility", () => {
    it("thumbnails are keyboard focusable", () => {
      render(<ScreenshotGallery screenshots={mockScreenshots} />);

      const thumbnail = screen.getByTestId("screenshot-thumbnail-0");
      thumbnail.focus();
      expect(thumbnail).toHaveFocus();
    });

    it("thumbnail has accessible alt text", () => {
      render(<ScreenshotGallery screenshots={mockScreenshots} />);

      const thumbnail = screen.getByTestId("screenshot-thumbnail-0");
      const img = thumbnail.querySelector("img");
      expect(img).toHaveAttribute("alt", "QA1");
    });

    it("lightbox close button has title", async () => {
      const user = userEvent.setup();
      render(<ScreenshotGallery screenshots={mockScreenshots} />);

      await user.click(screen.getByTestId("screenshot-thumbnail-0"));

      expect(screen.getByTestId("lightbox-close")).toHaveAttribute(
        "title",
        "Close (Esc)"
      );
    });

    it("navigation buttons have titles with keyboard shortcuts", async () => {
      const user = userEvent.setup();
      render(<ScreenshotGallery screenshots={mockScreenshots} />);

      await user.click(screen.getByTestId("screenshot-thumbnail-0"));

      expect(screen.getByTestId("lightbox-next")).toHaveAttribute(
        "title",
        "Next (→)"
      );
      expect(screen.getByTestId("lightbox-prev")).toHaveAttribute(
        "title",
        "Previous (←)"
      );
    });

    it("zoom buttons have titles with keyboard shortcuts", async () => {
      const user = userEvent.setup();
      render(<ScreenshotGallery screenshots={mockScreenshots} />);

      await user.click(screen.getByTestId("screenshot-thumbnail-0"));

      expect(screen.getByTestId("lightbox-zoom-in")).toHaveAttribute(
        "title",
        "Zoom in (+)"
      );
      expect(screen.getByTestId("lightbox-zoom-out")).toHaveAttribute(
        "title",
        "Zoom out (-)"
      );
    });
  });
});
