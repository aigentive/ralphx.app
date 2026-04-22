import { useCallback, useEffect, useRef } from "react";
import { useQueryClient } from "@tanstack/react-query";

import { chatApi } from "@/api/chat";
import { invalidateConversationDataQueries } from "@/hooks/useChat";
import { useEventBus } from "@/providers/EventProvider";
import {
  PlanVerificationStatusChangedSchema,
  ProposalEventSchema,
  type PlanVerificationStatusChangedPayload,
} from "@/types/events";
import { PendingVerificationEventSchema } from "@/types/verification-config";
import type { AgentConversation } from "./agentConversations";

type BridgeMessage = {
  eventKey: string;
  eventType: string;
  content: string;
  metadata: Record<string, unknown>;
};

type ExternalEvent = {
  id: number;
  event_type: string;
  project_id: string;
  payload: Record<string, unknown>;
  created_at: string;
};

type ExternalEventsResponse = {
  events: ExternalEvent[];
  next_cursor: number | null;
  has_more: boolean;
};

type ProjectAgentBridgeParams = {
  conversation: AgentConversation | null;
  attachedIdeationSessionId: string | null;
  projectId: string | null;
};

export function useProjectAgentBridgeEvents({
  conversation,
  attachedIdeationSessionId,
  projectId,
}: ProjectAgentBridgeParams) {
  const bus = useEventBus();
  const queryClient = useQueryClient();
  const cursorRef = useRef(0);
  const inFlightKeysRef = useRef<Set<string>>(new Set());

  const appendBridgeMessage = useCallback(
    async (message: BridgeMessage) => {
      if (!conversation || conversation.contextType !== "project" || !attachedIdeationSessionId) {
        return;
      }
      if (inFlightKeysRef.current.has(message.eventKey)) {
        return;
      }
      inFlightKeysRef.current.add(message.eventKey);
      try {
        const created = await chatApi.appendAgentBridgeMessage({
          conversationId: conversation.id,
          sourceSessionId: attachedIdeationSessionId,
          eventType: message.eventType,
          eventKey: message.eventKey,
          content: message.content,
          metadata: message.metadata,
        });
        if (created) {
          invalidateConversationDataQueries(queryClient, conversation.id);
        }
      } catch (error) {
        console.warn("[AgentsBridge] Failed to append bridge message", error);
      } finally {
        inFlightKeysRef.current.delete(message.eventKey);
      }
    },
    [attachedIdeationSessionId, conversation, queryClient]
  );

  useEffect(() => {
    cursorRef.current = 0;
    inFlightKeysRef.current.clear();
  }, [attachedIdeationSessionId, conversation?.id, projectId]);

  useEffect(() => {
    if (!conversation || conversation.contextType !== "project" || !attachedIdeationSessionId) {
      return;
    }

    const unsubscribes = [
      bus.subscribe<unknown>("ideation:child_session_created", (payload) => {
        const message = bridgeMessageFromChildSessionEvent(payload, attachedIdeationSessionId);
        if (message) {
          void appendBridgeMessage(message);
        }
      }),
      bus.subscribe<unknown>("plan_verification:status_changed", (payload) => {
        const parsed = PlanVerificationStatusChangedSchema.safeParse(payload);
        if (!parsed.success) {
          return;
        }
        const message = bridgeMessageFromVerificationEvent(parsed.data, attachedIdeationSessionId);
        if (message) {
          void appendBridgeMessage(message);
        }
      }),
      bus.subscribe<unknown>("proposal:created", (payload) => {
        const parsed = ProposalEventSchema.safeParse({
          type: "created",
          ...(isRecord(payload) ? payload : {}),
        });
        if (!parsed.success || parsed.data.type !== "created") {
          return;
        }
        const proposal = parsed.data.proposal;
        if (proposal.session_id !== attachedIdeationSessionId) {
          return;
        }
        void appendBridgeMessage({
          eventKey: `proposal:${attachedIdeationSessionId}:${proposal.id}:created`,
          eventType: "proposal:created",
          content: `Proposal created: ${proposal.title}.`,
          metadata: { proposalId: proposal.id, title: proposal.title },
        });
      }),
      bus.subscribe<unknown>("ideation:finalize_pending_confirmation", (payload) => {
        if (!isRecord(payload) || payload.sessionId !== attachedIdeationSessionId) {
          return;
        }
        void appendBridgeMessage({
          eventKey: `ideation:${attachedIdeationSessionId}:finalize_pending_confirmation`,
          eventType: "ideation:finalize_pending_confirmation",
          content: "The ideation run is waiting for plan finalization approval.",
          metadata: { payload },
        });
      }),
      bus.subscribe<unknown>("verification:pending_confirmation", (payload) => {
        const parsed = PendingVerificationEventSchema.safeParse(payload);
        if (!parsed.success || parsed.data.session_id !== attachedIdeationSessionId) {
          return;
        }
        void appendBridgeMessage({
          eventKey: `ideation:${attachedIdeationSessionId}:verification_pending_confirmation`,
          eventType: "verification:pending_confirmation",
          content: "Verification is waiting for approval to start.",
          metadata: { payload: parsed.data },
        });
      }),
    ];

    return () => {
      unsubscribes.forEach((unsubscribe) => unsubscribe());
    };
  }, [appendBridgeMessage, attachedIdeationSessionId, bus, conversation]);

  useEffect(() => {
    if (!projectId || !conversation || conversation.contextType !== "project" || !attachedIdeationSessionId) {
      return;
    }

    let cancelled = false;
    const pollEvents = async () => {
      let cursor = cursorRef.current;
      for (let page = 0; page < 10 && !cancelled; page += 1) {
        const response = await fetchProjectEvents(projectId, cursor);
        let maxCursor = cursor;
        for (const event of response.events) {
          maxCursor = Math.max(maxCursor, event.id);
          const message = bridgeMessageFromExternalEvent(event, attachedIdeationSessionId);
          if (message) {
            void appendBridgeMessage(message);
          }
        }
        cursorRef.current = maxCursor;
        if (!response.has_more || response.next_cursor == null) {
          break;
        }
        cursor = response.next_cursor;
      }
    };

    void pollEvents().catch((error) => {
      console.warn("[AgentsBridge] Failed to poll external events", error);
    });
    const intervalId = window.setInterval(() => {
      void pollEvents().catch((error) => {
        console.warn("[AgentsBridge] Failed to poll external events", error);
      });
    }, 5_000);

    return () => {
      cancelled = true;
      window.clearInterval(intervalId);
    };
  }, [appendBridgeMessage, attachedIdeationSessionId, conversation, projectId]);
}

export function bridgeMessageFromVerificationEvent(
  event: PlanVerificationStatusChangedPayload,
  sessionId: string
): BridgeMessage | null {
  if (event.session_id !== sessionId) {
    return null;
  }
  const generation = event.generation ?? 0;
  if (event.in_progress) {
    return {
      eventKey: `ideation:${sessionId}:verification_started:${generation}`,
      eventType: "plan_verification:status_changed",
      content: "Verification started for the attached ideation run.",
      metadata: { status: event.status, generation, round: event.round ?? null },
    };
  }
  if (event.status === "verified") {
    return {
      eventKey: `ideation:${sessionId}:verified`,
      eventType: "plan_verification:status_changed",
      content: verificationCompleteContent("Plan verified for the attached ideation run.", event),
      metadata: { status: event.status, generation, gapScore: event.gap_score ?? null },
    };
  }
  if (event.status === "needs_revision") {
    return {
      eventKey: `ideation:${sessionId}:verification_needs_revision:${generation}`,
      eventType: "plan_verification:status_changed",
      content: verificationCompleteContent(
        "Verification finished with gaps remaining. Open Verification to review them.",
        event
      ),
      metadata: { status: event.status, generation, gapScore: event.gap_score ?? null },
    };
  }
  if (event.status === "skipped") {
    return {
      eventKey: `ideation:${sessionId}:verification_skipped:${generation}`,
      eventType: "plan_verification:status_changed",
      content: "Verification was skipped for the attached ideation run.",
      metadata: { status: event.status, generation },
    };
  }
  return null;
}

export function bridgeMessageFromExternalEvent(
  event: ExternalEvent,
  sessionId: string
): BridgeMessage | null {
  const payload = event.payload;
  if (payloadSessionId(payload) !== sessionId) {
    return null;
  }

  switch (event.event_type) {
    case "ideation:plan_created": {
      const title = stringField(payload, "plan_title") ?? stringField(payload, "session_title");
      return {
        eventKey: `ideation:${sessionId}:plan_created`,
        eventType: event.event_type,
        content: title ? `Plan is ready: ${title}.` : "Plan is ready in the attached ideation run.",
        metadata: { externalEventId: event.id, payload },
      };
    }
    case "ideation:verified":
      return {
        eventKey: `ideation:${sessionId}:verified`,
        eventType: event.event_type,
        content: "Plan verified for the attached ideation run.",
        metadata: { externalEventId: event.id, payload },
      };
    case "ideation:proposals_ready": {
      const count = numberField(payload, "proposal_count");
      return {
        eventKey: `ideation:${sessionId}:proposals_ready`,
        eventType: event.event_type,
        content:
          count === 1
            ? "Proposals are ready: 1 proposal generated."
            : `Proposals are ready: ${count ?? "multiple"} proposals generated.`,
        metadata: { externalEventId: event.id, payload },
      };
    }
    case "ideation:session_accepted":
      return {
        eventKey: `ideation:${sessionId}:session_accepted`,
        eventType: event.event_type,
        content: "Plan accepted. RalphX is creating and running implementation tasks.",
        metadata: { externalEventId: event.id, payload },
      };
    case "task:execution_started":
      return {
        eventKey: `pipeline:${sessionId}:task_execution_started:${taskIdentity(payload)}`,
        eventType: event.event_type,
        content: `Task started: ${taskLabel(payload)}.`,
        metadata: { externalEventId: event.id, payload },
      };
    case "task:execution_completed":
      return {
        eventKey: `pipeline:${sessionId}:task_execution_completed:${taskIdentity(payload)}`,
        eventType: event.event_type,
        content: `Task execution completed: ${taskLabel(payload)}.`,
        metadata: { externalEventId: event.id, payload },
      };
    case "merge:ready":
      return {
        eventKey: `pipeline:${sessionId}:merge_ready:${taskIdentity(payload)}`,
        eventType: event.event_type,
        content: `Merge ready: ${taskLabel(payload)}.`,
        metadata: { externalEventId: event.id, payload },
      };
    case "merge:completed": {
      const commit = stringField(payload, "commit_sha");
      const suffix = commit ? ` Commit ${commit.slice(0, 7)}.` : "";
      return {
        eventKey: `pipeline:${sessionId}:merge_completed:${taskIdentity(payload)}:${commit ?? event.id}`,
        eventType: event.event_type,
        content: `Merged: ${taskLabel(payload)}.${suffix}`,
        metadata: { externalEventId: event.id, payload },
      };
    }
    case "task:status_changed": {
      const newStatus = stringField(payload, "new_status");
      if (!newStatus || !["blocked", "failed", "merge_incomplete", "cancelled"].includes(newStatus)) {
        return null;
      }
      return {
        eventKey: `pipeline:${sessionId}:task_status:${taskIdentity(payload)}:${newStatus}`,
        eventType: event.event_type,
        content: `Task needs attention: ${taskLabel(payload)} is ${newStatus.replace(/_/g, " ")}.`,
        metadata: { externalEventId: event.id, payload },
      };
    }
    default:
      return null;
  }
}

function bridgeMessageFromChildSessionEvent(payload: unknown, sessionId: string): BridgeMessage | null {
  if (!isRecord(payload) || payload.parentSessionId !== sessionId || payload.purpose !== "verification") {
    return null;
  }
  const childSessionId = typeof payload.sessionId === "string" ? payload.sessionId : "unknown";
  return {
    eventKey: `ideation:${sessionId}:verification_child:${childSessionId}`,
    eventType: "ideation:child_session_created",
    content: "Verification agent spawned for the attached ideation run.",
    metadata: { payload },
  };
}

async function fetchProjectEvents(projectId: string, cursor: number): Promise<ExternalEventsResponse> {
  const params = new URLSearchParams({
    project_id: projectId,
    cursor: String(cursor),
    limit: "200",
  });
  const response = await fetch(`http://localhost:3847/api/external/events/poll?${params.toString()}`);
  if (!response.ok) {
    throw new Error(`External event poll failed: ${response.status}`);
  }
  return (await response.json()) as ExternalEventsResponse;
}

function verificationCompleteContent(prefix: string, event: PlanVerificationStatusChangedPayload): string {
  const details: string[] = [];
  if (event.round != null && event.max_rounds != null) {
    details.push(`round ${event.round}/${event.max_rounds}`);
  }
  if (event.gap_score != null) {
    details.push(`gap score ${event.gap_score}`);
  }
  return details.length > 0 ? `${prefix} (${details.join(", ")}).` : prefix;
}

function payloadSessionId(payload: Record<string, unknown>): string | null {
  return stringField(payload, "session_id") ?? stringField(payload, "sessionId");
}

function taskIdentity(payload: Record<string, unknown>): string {
  return stringField(payload, "task_id") ?? stringField(payload, "taskId") ?? "unknown";
}

function taskLabel(payload: Record<string, unknown>): string {
  return truncateLabel(
    stringField(payload, "task_title") ??
      stringField(payload, "taskTitle") ??
      cleanHumanContext(stringField(payload, "human_context")) ??
      taskIdentity(payload)
  );
}

function cleanHumanContext(value: string | null): string | null {
  if (!value) {
    return null;
  }
  return value.replace(/^\[[^\]]+\]\s*/, "").trim();
}

function truncateLabel(value: string): string {
  return value.length > 140 ? `${value.slice(0, 137)}...` : value;
}

function stringField(record: Record<string, unknown>, key: string): string | null {
  const value = record[key];
  return typeof value === "string" && value.trim().length > 0 ? value : null;
}

function numberField(record: Record<string, unknown>, key: string): number | null {
  const value = record[key];
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
