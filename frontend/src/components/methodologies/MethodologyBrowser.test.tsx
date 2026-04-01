/**
 * MethodologyBrowser component tests
 *
 * Tests for:
 * - Displaying list of methodologies
 * - Active methodology badge
 * - Methodology cards with details
 * - Activate/Deactivate buttons
 * - Empty state
 * - Accessibility
 * - Styling with design tokens
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, within } from "@testing-library/react";
import { MethodologyBrowser } from "./MethodologyBrowser";
import type { MethodologyExtension } from "@/types/methodology";

// ============================================================================
// Test Data
// ============================================================================

const createMockMethodology = (overrides: Partial<MethodologyExtension> = {}): MethodologyExtension => ({
  id: "methodology-1",
  name: "BMAD",
  description: "Business Model Aligned Development",
  agentProfiles: [
    { id: "analyst", name: "Analyst", role: "Research and analysis" },
    { id: "architect", name: "Architect", role: "System design" },
  ],
  skills: [],
  workflow: { id: "bmad-workflow", name: "BMAD Workflow", columns: [], isDefault: false },
  phases: [
    { id: "p1", name: "Discovery", order: 1, requiredArtifacts: [], outputArtifacts: [] },
    { id: "p2", name: "Design", order: 2, requiredArtifacts: [], outputArtifacts: [] },
  ],
  templates: [],
  isActive: false,
  ...overrides,
});

const mockMethodologies: MethodologyExtension[] = [
  createMockMethodology({ id: "m1", name: "BMAD", description: "Business Model Aligned Development", isActive: true }),
  createMockMethodology({ id: "m2", name: "GSD", description: "Get Stuff Done", isActive: false }),
  createMockMethodology({ id: "m3", name: "Custom", description: "Custom methodology", isActive: false }),
];

describe("MethodologyBrowser", () => {
  const defaultProps = {
    methodologies: mockMethodologies,
    onActivate: vi.fn(),
    onDeactivate: vi.fn(),
    onSelect: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ==========================================================================
  // Rendering
  // ==========================================================================

  describe("rendering", () => {
    it("renders component with testid", () => {
      render(<MethodologyBrowser {...defaultProps} />);
      expect(screen.getByTestId("methodology-browser")).toBeInTheDocument();
    });

    it("displays all methodologies", () => {
      render(<MethodologyBrowser {...defaultProps} />);
      const cards = screen.getAllByTestId("methodology-card");
      expect(cards).toHaveLength(3);
    });

    it("displays methodology names", () => {
      render(<MethodologyBrowser {...defaultProps} />);
      expect(screen.getByText("BMAD")).toBeInTheDocument();
      expect(screen.getByText("GSD")).toBeInTheDocument();
      expect(screen.getByText("Custom")).toBeInTheDocument();
    });

    it("displays methodology descriptions", () => {
      render(<MethodologyBrowser {...defaultProps} />);
      expect(screen.getByText("Business Model Aligned Development")).toBeInTheDocument();
      expect(screen.getByText("Get Stuff Done")).toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Methodology Cards
  // ==========================================================================

  describe("methodology cards", () => {
    it("shows phase count on cards", () => {
      render(<MethodologyBrowser {...defaultProps} />);
      const cards = screen.getAllByTestId("methodology-card");
      expect(within(cards[0]).getByTestId("phase-count")).toHaveTextContent("2 phases");
    });

    it("shows agent count on cards", () => {
      render(<MethodologyBrowser {...defaultProps} />);
      const cards = screen.getAllByTestId("methodology-card");
      expect(within(cards[0]).getByTestId("agent-count")).toHaveTextContent("2 agents");
    });

    it("calls onSelect when card is clicked", () => {
      render(<MethodologyBrowser {...defaultProps} />);
      const cards = screen.getAllByTestId("methodology-card");
      fireEvent.click(cards[0]);
      expect(defaultProps.onSelect).toHaveBeenCalledWith("m1");
    });
  });

  // ==========================================================================
  // Active State
  // ==========================================================================

  describe("active state", () => {
    it("shows active badge on active methodology", () => {
      render(<MethodologyBrowser {...defaultProps} />);
      const cards = screen.getAllByTestId("methodology-card");
      expect(within(cards[0]).getByTestId("active-badge")).toBeInTheDocument();
    });

    it("does not show active badge on inactive methodologies", () => {
      render(<MethodologyBrowser {...defaultProps} />);
      const cards = screen.getAllByTestId("methodology-card");
      expect(within(cards[1]).queryByTestId("active-badge")).not.toBeInTheDocument();
      expect(within(cards[2]).queryByTestId("active-badge")).not.toBeInTheDocument();
    });

    it("highlights active methodology card", () => {
      render(<MethodologyBrowser {...defaultProps} />);
      const cards = screen.getAllByTestId("methodology-card");
      expect(cards[0]).toHaveAttribute("data-active", "true");
      expect(cards[1]).toHaveAttribute("data-active", "false");
    });
  });

  // ==========================================================================
  // Activate/Deactivate Actions
  // ==========================================================================

  describe("activate/deactivate actions", () => {
    it("shows deactivate button for active methodology", () => {
      render(<MethodologyBrowser {...defaultProps} />);
      const cards = screen.getAllByTestId("methodology-card");
      expect(within(cards[0]).getByTestId("deactivate-button")).toBeInTheDocument();
    });

    it("shows activate button for inactive methodologies", () => {
      render(<MethodologyBrowser {...defaultProps} />);
      const cards = screen.getAllByTestId("methodology-card");
      expect(within(cards[1]).getByTestId("activate-button")).toBeInTheDocument();
      expect(within(cards[2]).getByTestId("activate-button")).toBeInTheDocument();
    });

    it("calls onDeactivate when deactivate clicked", () => {
      render(<MethodologyBrowser {...defaultProps} />);
      const cards = screen.getAllByTestId("methodology-card");
      fireEvent.click(within(cards[0]).getByTestId("deactivate-button"));
      expect(defaultProps.onDeactivate).toHaveBeenCalledWith("m1");
    });

    it("calls onActivate when activate clicked", () => {
      render(<MethodologyBrowser {...defaultProps} />);
      const cards = screen.getAllByTestId("methodology-card");
      fireEvent.click(within(cards[1]).getByTestId("activate-button"));
      expect(defaultProps.onActivate).toHaveBeenCalledWith("m2");
    });

    it("stops event propagation on button clicks", () => {
      render(<MethodologyBrowser {...defaultProps} />);
      const cards = screen.getAllByTestId("methodology-card");
      fireEvent.click(within(cards[0]).getByTestId("deactivate-button"));
      expect(defaultProps.onSelect).not.toHaveBeenCalled();
    });
  });

  // ==========================================================================
  // Empty State
  // ==========================================================================

  describe("empty state", () => {
    it("shows empty message when no methodologies", () => {
      render(<MethodologyBrowser {...defaultProps} methodologies={[]} />);
      expect(screen.getByText(/no methodologies/i)).toBeInTheDocument();
    });

    it("hides methodology list when empty", () => {
      render(<MethodologyBrowser {...defaultProps} methodologies={[]} />);
      expect(screen.queryByTestId("methodology-card")).not.toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Accessibility
  // ==========================================================================

  describe("accessibility", () => {
    it("methodology cards have button role", () => {
      render(<MethodologyBrowser {...defaultProps} />);
      const cards = screen.getAllByTestId("methodology-card");
      cards.forEach((card) => {
        expect(card).toHaveAttribute("role", "button");
      });
    });

    it("cards have accessible names", () => {
      render(<MethodologyBrowser {...defaultProps} />);
      const cards = screen.getAllByTestId("methodology-card");
      expect(cards[0]).toHaveAttribute("aria-label", "BMAD");
    });

    it("activate buttons have accessible names", () => {
      render(<MethodologyBrowser {...defaultProps} />);
      const cards = screen.getAllByTestId("methodology-card");
      const activateBtn = within(cards[1]).getByTestId("activate-button");
      expect(activateBtn).toHaveAttribute("aria-label", "Activate GSD");
    });
  });

  // ==========================================================================
  // Styling
  // ==========================================================================

  describe("styling", () => {
    it("uses design tokens for background", () => {
      render(<MethodologyBrowser {...defaultProps} />);
      const browser = screen.getByTestId("methodology-browser");
      expect(browser).toHaveStyle({ backgroundColor: "var(--bg-surface)" });
    });

    it("uses accent color for active badge", () => {
      render(<MethodologyBrowser {...defaultProps} />);
      const cards = screen.getAllByTestId("methodology-card");
      const badge = within(cards[0]).getByTestId("active-badge");
      expect(badge).toHaveStyle({ color: "var(--status-success)" });
    });

    it("uses accent border for active card", () => {
      render(<MethodologyBrowser {...defaultProps} />);
      const cards = screen.getAllByTestId("methodology-card");
      const style = cards[0].getAttribute("style");
      expect(style).toContain("border-color: var(--accent-primary)");
    });
  });
});
