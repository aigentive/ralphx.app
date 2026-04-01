import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ActivityMessage } from "./ActivityMessage";

const mockNavigateToIdeationSession = vi.fn();

vi.mock("@/lib/navigation", () => ({
  navigateToIdeationSession: (sessionId: string) =>
    mockNavigateToIdeationSession(sessionId),
}));

describe("ActivityMessage", () => {
  it("opens follow-up ideation session from system activity metadata", async () => {
    const user = userEvent.setup();

    render(
      <ActivityMessage
        message={{
          id: "evt-1",
          type: "system",
          content:
            "Linked follow-up ideation session to handle unresolved unrelated scope drift separately.",
          timestamp: Date.now(),
          metadata: {
            followupSessionId: "session-followup-1",
          },
          taskId: "task-1",
          role: "system",
        }}
        isExpanded={false}
        onToggle={vi.fn()}
        copied={false}
        onCopy={vi.fn()}
      />
    );

    await user.click(screen.getByRole("button", { name: /Open Follow-up/i }));

    expect(mockNavigateToIdeationSession).toHaveBeenCalledWith(
      "session-followup-1"
    );
  });
});
