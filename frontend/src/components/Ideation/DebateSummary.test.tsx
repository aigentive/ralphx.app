import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect } from "vitest";
import { DebateSummary } from "./DebateSummary";
import type { DebateSummaryData } from "./DebateSummary";

const mockData: DebateSummaryData = {
  advocates: [
    {
      name: "WebSockets",
      role: "Advocate A",
      strengths: ["Real-time bidirectional", "Low latency"],
      weaknesses: ["Complex setup", "Stateful connections"],
      evidence: ["Used by Figma for collab editing", "RFC 6455 standard"],
      criticChallenge: "Scaling WebSocket connections across multiple servers requires sticky sessions or a pub/sub layer.",
    },
    {
      name: "SSE",
      role: "Advocate B",
      strengths: ["Simple HTTP-based", "Auto-reconnect"],
      weaknesses: ["Unidirectional only", "No binary support"],
      evidence: ["GitHub uses SSE for notifications"],
      criticChallenge: "Cannot handle bidirectional communication without a separate POST channel.",
    },
    {
      name: "Sync Layer",
      role: "Advocate C",
      strengths: ["Abstractable transport", "Offline-first capable"],
      weaknesses: ["Higher complexity", "Larger bundle size"],
      evidence: ["CRDTs power collaborative apps"],
      criticChallenge: "Adds significant abstraction overhead for simple use cases.",
    },
  ],
  winner: {
    name: "WebSockets",
    justification: "Bidirectional communication needed for collaborative editing",
  },
};

describe("DebateSummary", () => {
  it("renders all advocate names", () => {
    render(<DebateSummary data={mockData} />);

    // WebSockets appears in both column + winner, others only in column
    expect(screen.getAllByText("WebSockets").length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText("SSE")).toBeInTheDocument();
    expect(screen.getByText("Sync Layer")).toBeInTheDocument();
  });

  it("renders advocate roles", () => {
    render(<DebateSummary data={mockData} />);

    expect(screen.getByText("Advocate A")).toBeInTheDocument();
    expect(screen.getByText("Advocate B")).toBeInTheDocument();
    expect(screen.getByText("Advocate C")).toBeInTheDocument();
  });

  it("renders the winner indicator with name and justification", () => {
    render(<DebateSummary data={mockData} />);

    const winnerSection = screen.getByTestId("debate-winner");
    expect(winnerSection).toBeInTheDocument();
    expect(within(winnerSection).getByText("WebSockets")).toBeInTheDocument();
    expect(within(winnerSection).getByText(/Bidirectional communication/)).toBeInTheDocument();
  });

  it("renders strengths for each advocate in wide layout", () => {
    render(<DebateSummary data={mockData} />);

    expect(screen.getByText("Real-time bidirectional")).toBeInTheDocument();
    expect(screen.getByText("Simple HTTP-based")).toBeInTheDocument();
    expect(screen.getByText("Abstractable transport")).toBeInTheDocument();
  });

  it("renders weaknesses for each advocate", () => {
    render(<DebateSummary data={mockData} />);

    expect(screen.getByText("Complex setup")).toBeInTheDocument();
    expect(screen.getByText("Unidirectional only")).toBeInTheDocument();
    expect(screen.getByText("Higher complexity")).toBeInTheDocument();
  });

  it("renders evidence for each advocate", () => {
    render(<DebateSummary data={mockData} />);

    expect(screen.getByText("Used by Figma for collab editing")).toBeInTheDocument();
    expect(screen.getByText("GitHub uses SSE for notifications")).toBeInTheDocument();
    expect(screen.getByText("CRDTs power collaborative apps")).toBeInTheDocument();
  });

  it("renders critic challenges", () => {
    render(<DebateSummary data={mockData} />);

    expect(screen.getByText(/Scaling WebSocket connections/)).toBeInTheDocument();
    expect(screen.getByText(/Cannot handle bidirectional/)).toBeInTheDocument();
    expect(screen.getByText(/Adds significant abstraction/)).toBeInTheDocument();
  });

  it("renders section headers", () => {
    render(<DebateSummary data={mockData} />);

    // Each advocate column has these section headers
    const strengthsHeaders = screen.getAllByText("Strengths");
    const weaknessesHeaders = screen.getAllByText("Weaknesses");
    const evidenceHeaders = screen.getAllByText("Evidence");
    const criticHeaders = screen.getAllByText("Critic Challenge");

    expect(strengthsHeaders.length).toBe(3);
    expect(weaknessesHeaders.length).toBe(3);
    expect(evidenceHeaders.length).toBe(3);
    expect(criticHeaders.length).toBe(3);
  });

  it("highlights the winner column with accent border", () => {
    render(<DebateSummary data={mockData} />);

    const winnerColumn = screen.getByTestId("advocate-column-WebSockets");
    expect(winnerColumn).toBeInTheDocument();
    // Winner column should have a warm orange border (design token)
    const style = winnerColumn.getAttribute("style") ?? "";
    expect(style).toContain("var(--accent-primary)");
  });

  it("handles single advocate gracefully", () => {
    const singleData: DebateSummaryData = {
      advocates: [mockData.advocates[0]],
      winner: mockData.winner,
    };

    render(<DebateSummary data={singleData} />);
    expect(screen.getAllByText("WebSockets").length).toBeGreaterThanOrEqual(1);
    expect(screen.getByTestId("debate-winner")).toBeInTheDocument();
  });

  it("handles advocate with optional color", () => {
    const colorData: DebateSummaryData = {
      ...mockData,
      advocates: mockData.advocates.map((a, i) => ({
        ...a,
        color: i === 0 ? "hsl(200 70% 50%)" : undefined,
      })),
    };

    render(<DebateSummary data={colorData} />);
    expect(screen.getAllByText("WebSockets").length).toBeGreaterThanOrEqual(1);
  });
});

describe("DebateSummary - Narrow layout (collapsible cards)", () => {
  it("renders collapsible cards with advocate names", () => {
    // Force narrow layout by setting testNarrow prop
    render(<DebateSummary data={mockData} forceNarrow />);

    expect(screen.getByTestId("advocate-card-WebSockets")).toBeInTheDocument();
    expect(screen.getByTestId("advocate-card-SSE")).toBeInTheDocument();
    expect(screen.getByTestId("advocate-card-Sync Layer")).toBeInTheDocument();
  });

  it("first card is expanded by default, others collapsed", () => {
    render(<DebateSummary data={mockData} forceNarrow />);

    // First card's content should be visible
    const firstCard = screen.getByTestId("advocate-card-WebSockets");
    expect(within(firstCard).getByText("Real-time bidirectional")).toBeInTheDocument();

    // Other cards' detailed content should not be visible (collapsed)
    const secondCard = screen.getByTestId("advocate-card-SSE");
    expect(within(secondCard).queryByText("Simple HTTP-based")).not.toBeInTheDocument();
  });

  it("clicking a collapsed card expands it", async () => {
    const user = userEvent.setup();
    render(<DebateSummary data={mockData} forceNarrow />);

    const secondCardTrigger = screen.getByTestId("advocate-trigger-SSE");
    await user.click(secondCardTrigger);

    const secondCard = screen.getByTestId("advocate-card-SSE");
    expect(within(secondCard).getByText("Simple HTTP-based")).toBeInTheDocument();
  });

  it("renders winner section in narrow layout", () => {
    render(<DebateSummary data={mockData} forceNarrow />);

    const winnerSection = screen.getByTestId("debate-winner");
    expect(winnerSection).toBeInTheDocument();
    expect(within(winnerSection).getByText("WebSockets")).toBeInTheDocument();
  });
});
