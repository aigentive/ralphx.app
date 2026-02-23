/**
 * EventCards component tests
 *
 * Tests for TransitionEventCard, ActivityEventCard, ReviewEventCard,
 * EventCard dispatcher, SourceBadge, and ExpandableContent behaviour.
 */

import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import React from "react";
import { EventCard, SourceBadge } from "./EventCards";
import type { AuditEntry } from "./EventCards";

// ============================================================================
// Helpers
// ============================================================================

function makeEntry(overrides: Partial<AuditEntry> = {}): AuditEntry {
  return {
    id: "e1",
    timestamp: "2026-02-23T10:00:00+00:00",
    source: "activity",
    type: "text",
    actor: "Agent",
    description: "Some description",
    ...overrides,
  };
}

// ============================================================================
// TransitionEventCard
// ============================================================================

describe("TransitionEventCard", () => {
  it("renders from→to status badges", () => {
    render(
      <EventCard
        entry={makeEntry({
          source: "transition",
          type: "transition",
          fromStatus: "executing",
          toStatus: "pending_review",
          actor: "system",
          description: "",
        })}
      />
    );
    expect(screen.getByTestId("transition-card")).toBeInTheDocument();
    expect(screen.getByText("Executing")).toBeInTheDocument();
    expect(screen.getByText("Pending Review")).toBeInTheDocument();
  });

  it("shows actor/trigger", () => {
    render(
      <EventCard
        entry={makeEntry({
          source: "transition",
          type: "transition",
          fromStatus: "backlog",
          toStatus: "ready",
          actor: "scheduler",
          description: "",
        })}
      />
    );
    expect(screen.getByText(/scheduler/i)).toBeInTheDocument();
  });

  it("shows reason/description text", () => {
    render(
      <EventCard
        entry={makeEntry({
          source: "transition",
          type: "transition",
          fromStatus: "executing",
          toStatus: "failed",
          actor: "system",
          description: "Execution timed out after 10 minutes",
        })}
      />
    );
    expect(screen.getByText(/timed out/i)).toBeInTheDocument();
  });

  it("shows timestamp", () => {
    render(
      <EventCard
        entry={makeEntry({
          source: "transition",
          type: "transition",
          fromStatus: "backlog",
          toStatus: "ready",
          actor: "system",
          description: "",
          timestamp: "2026-02-23T14:30:00+00:00",
        })}
      />
    );
    const body = document.body.textContent ?? "";
    expect(body.includes("2026") || body.includes("14:30")).toBe(true);
  });
});

// ============================================================================
// ActivityEventCard
// ============================================================================

describe("ActivityEventCard", () => {
  it("renders with activity-card testid", () => {
    render(<EventCard entry={makeEntry({ source: "activity", type: "text" })} />);
    expect(screen.getByTestId("activity-card")).toBeInTheDocument();
  });

  it("renders tool_call with extracted tool name badge", () => {
    render(
      <EventCard
        entry={makeEntry({
          source: "activity",
          type: "tool_call",
          description: JSON.stringify({ tool: "bash", input: { command: "ls" } }),
          actor: "Agent",
        })}
      />
    );
    expect(screen.getByText("bash")).toBeInTheDocument();
  });

  it("renders thinking collapsed by default when description >100 chars", () => {
    const longThinking = "T".repeat(150);
    render(
      <EventCard
        entry={makeEntry({
          source: "activity",
          type: "thinking",
          description: longThinking,
        })}
      />
    );
    // Full text not visible — truncated
    expect(screen.queryByText(longThinking)).not.toBeInTheDocument();
    expect(screen.getByText(/Show more/i)).toBeInTheDocument();
  });

  it("renders error with data-variant=error attribute", () => {
    render(
      <EventCard
        entry={makeEntry({
          source: "activity",
          type: "error",
          description: "Something went wrong",
          actor: "Agent",
        })}
      />
    );
    const card = screen.getByTestId("activity-card");
    expect(card).toBeInTheDocument();
    expect(card.getAttribute("data-variant")).toBe("error");
  });

  it("shows timestamp", () => {
    render(
      <EventCard
        entry={makeEntry({
          source: "activity",
          type: "text",
          description: "Working...",
          timestamp: "2026-02-23T09:15:00+00:00",
        })}
      />
    );
    const body = document.body.textContent ?? "";
    expect(body.includes("2026") || body.includes("09:15")).toBe(true);
  });

  it("shows actor", () => {
    render(
      <EventCard
        entry={makeEntry({ source: "activity", type: "text", actor: "AI Agent" })}
      />
    );
    expect(screen.getByText(/AI Agent/i)).toBeInTheDocument();
  });
});

// ============================================================================
// ReviewEventCard
// ============================================================================

describe("ReviewEventCard", () => {
  it("renders Approved outcome badge", () => {
    render(
      <EventCard
        entry={makeEntry({
          source: "review",
          type: "Approved",
          actor: "AI Reviewer",
          description: "All checks passed",
        })}
      />
    );
    expect(screen.getByTestId("review-card")).toBeInTheDocument();
    expect(screen.getByText("Approved")).toBeInTheDocument();
  });

  it("renders Changes Requested outcome badge", () => {
    render(
      <EventCard
        entry={makeEntry({
          source: "review",
          type: "Changes Requested",
          actor: "AI Reviewer",
          description: "Found some issues",
        })}
      />
    );
    expect(screen.getByText("Changes Requested")).toBeInTheDocument();
  });

  it("renders Rejected outcome badge", () => {
    render(
      <EventCard
        entry={makeEntry({
          source: "review",
          type: "Rejected",
          actor: "AI Reviewer",
          description: "Critical issues found",
        })}
      />
    );
    expect(screen.getByText("Rejected")).toBeInTheDocument();
  });

  it("shows issues count from metadata", () => {
    render(
      <EventCard
        entry={makeEntry({
          source: "review",
          type: "Changes Requested",
          actor: "AI Reviewer",
          description: "Issues found",
          metadata: "3 issues found",
        })}
      />
    );
    expect(screen.getByText("3 issues found")).toBeInTheDocument();
  });

  it("shows reviewer actor", () => {
    render(
      <EventCard
        entry={makeEntry({
          source: "review",
          type: "Approved",
          actor: "AI Reviewer",
          description: "All good",
        })}
      />
    );
    expect(screen.getByText(/AI Reviewer/i)).toBeInTheDocument();
  });

  it("shows timestamp", () => {
    render(
      <EventCard
        entry={makeEntry({
          source: "review",
          type: "Approved",
          actor: "AI Reviewer",
          description: "",
          timestamp: "2026-02-23T12:45:00+00:00",
        })}
      />
    );
    const body = document.body.textContent ?? "";
    expect(body.includes("2026") || body.includes("12:45")).toBe(true);
  });
});

// ============================================================================
// ExpandableContent (via all card types)
// ============================================================================

describe("ExpandableContent", () => {
  it("truncates descriptions >200 chars and shows expand button", () => {
    const longText = "A".repeat(250);
    render(
      <EventCard entry={makeEntry({ source: "activity", type: "text", description: longText })} />
    );
    expect(screen.queryByText(longText)).not.toBeInTheDocument();
    expect(screen.getByText(/Show more/i)).toBeInTheDocument();
  });

  it("expands on click and shows full text", async () => {
    const longText = "B".repeat(250);
    render(
      <EventCard entry={makeEntry({ source: "activity", type: "text", description: longText })} />
    );
    await userEvent.click(screen.getByText(/Show more/i));
    expect(screen.getByText(longText)).toBeInTheDocument();
    expect(screen.getByText(/Show less/i)).toBeInTheDocument();
  });

  it("does not truncate descriptions ≤200 chars", () => {
    const shortText = "Short description under the limit";
    render(
      <EventCard entry={makeEntry({ source: "activity", type: "text", description: shortText })} />
    );
    expect(screen.getByText(shortText)).toBeInTheDocument();
    expect(screen.queryByText(/Show more/i)).not.toBeInTheDocument();
  });
});

// ============================================================================
// SourceBadge
// ============================================================================

describe("SourceBadge", () => {
  it("renders transition badge with correct label", () => {
    render(<SourceBadge source="transition" />);
    expect(screen.getByTestId("source-badge")).toBeInTheDocument();
    expect(screen.getByText("Transition")).toBeInTheDocument();
  });

  it("renders review badge with correct label", () => {
    render(<SourceBadge source="review" />);
    expect(screen.getByTestId("source-badge")).toBeInTheDocument();
    expect(screen.getByText("Review")).toBeInTheDocument();
  });

  it("renders activity badge with correct label", () => {
    render(<SourceBadge source="activity" />);
    expect(screen.getByTestId("source-badge")).toBeInTheDocument();
    expect(screen.getByText("Activity")).toBeInTheDocument();
  });
});
