import { describe, expect, it } from "vitest";

import {
  bridgeMessageFromExternalEvent,
  bridgeMessageFromVerificationEvent,
} from "./useProjectAgentBridgeEvents";

describe("project agent bridge event mapping", () => {
  it("maps plan creation events to stable parent-chat bridge messages", () => {
    const message = bridgeMessageFromExternalEvent(
      {
        id: 10,
        event_type: "ideation:plan_created",
        project_id: "project-1",
        created_at: "2026-04-22T21:27:43Z",
        payload: {
          session_id: "session-1",
          plan_title: "Fix Font Scale Switching Regression",
        },
      },
      "session-1"
    );

    expect(message?.eventKey).toBe("ideation:session-1:plan_created");
    expect(message?.content).toBe("Plan is ready: Fix Font Scale Switching Regression.");
  });

  it("uses the same verified key for external and live verification completion", () => {
    const external = bridgeMessageFromExternalEvent(
      {
        id: 11,
        event_type: "ideation:verified",
        project_id: "project-1",
        created_at: "2026-04-22T21:27:43Z",
        payload: { session_id: "session-1" },
      },
      "session-1"
    );

    const live = bridgeMessageFromVerificationEvent(
      {
        session_id: "session-1",
        status: "verified",
        in_progress: false,
        generation: 2,
        round: 4,
        max_rounds: 5,
        gap_score: 0,
      },
      "session-1"
    );

    expect(external?.eventKey).toBe("ideation:session-1:verified");
    expect(live?.eventKey).toBe("ideation:session-1:verified");
  });

  it("ignores events for another attached ideation session", () => {
    const message = bridgeMessageFromExternalEvent(
      {
        id: 12,
        event_type: "ideation:proposals_ready",
        project_id: "project-1",
        created_at: "2026-04-22T21:29:37Z",
        payload: {
          session_id: "other-session",
          proposal_count: 3,
        },
      },
      "session-1"
    );

    expect(message).toBeNull();
  });
});
