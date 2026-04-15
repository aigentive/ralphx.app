import { afterEach, describe, expect, it, vi } from "vitest";

import { createVerificationRuntime } from "../verification-runtime.js";

afterEach(() => {
  vi.useRealTimers();
});

describe("verification runtime parent resolution", () => {
  it("ignores model-supplied verifier session ids and resolves the canonical parent session", async () => {
    const callTauri = vi.fn();
    const callTauriGet = vi.fn(async (endpoint: string) => {
      if (endpoint === "parent_session_context/child-session") {
        return {
          parent_session: {
            id: "parent-session",
          },
        };
      }
      throw new Error(`unexpected endpoint ${endpoint}`);
    });

    const runtime = createVerificationRuntime({
      callTauri,
      callTauriGet,
      agentType: "ralphx-plan-verifier",
      contextType: "ideation",
      contextId: "child-session",
    });

    await expect(
      runtime.resolveVerifierParentSessionId(
        "wrong-session",
        "run_verification_enrichment"
      )
    ).resolves.toBe("parent-session");
  });

  it("remaps delegated verification publishers to the parent ideation session", async () => {
    const callTauri = vi.fn();
    const callTauriGet = vi.fn(async (endpoint: string) => {
      if (endpoint === "coordination/delegated-session/delegated-session/status") {
        return {
          session: {
            id: "delegated-session",
            parent_context_type: "ideation",
            parent_context_id: "parent-session",
          },
        };
      }
      throw new Error(`unexpected endpoint ${endpoint}`);
    });

    const runtime = createVerificationRuntime({
      callTauri,
      callTauriGet,
      agentType: "ralphx-plan-critic-completeness",
      contextType: "delegation",
      contextId: "delegated-session",
    });

    await expect(
      runtime.resolveVerificationFindingSessionId(
        undefined,
        "publish_verification_finding"
      )
    ).resolves.toBe("parent-session");
  });
});

describe("verification runtime settlement and terminal cleanup", () => {
  it("waits longer than 15s by default for enrichment delegates before calling the pass timed out", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-04-13T16:35:53.000Z"));

    const callTauri = vi.fn(async (endpoint: string) => {
      if (endpoint === "coordination/delegate/start") {
        return {
          job_id: "job-1",
          delegated_session_id: "delegated-1",
          agent_name: "ralphx:ralphx-ideation-specialist-intent",
        };
      }
      if (endpoint === "coordination/delegate/wait") {
        const completed = Date.now() >= Date.parse("2026-04-13T16:36:13.000Z");
        return {
          job_id: "job-1",
          status: completed ? "completed" : "running",
          delegated_status: {
            latest_run: {
              status: completed ? "completed" : "running",
            },
            agent_state: {
              estimated_status: completed ? "completed" : "running",
            },
          },
        };
      }
      throw new Error(`unexpected endpoint ${endpoint}`);
    });
    const callTauriGet = vi.fn(async (endpoint: string) => {
      if (endpoint === "parent_session_context/child-session") {
        return {
          parent_session: {
            id: "parent-session",
          },
        };
      }
      if (endpoint === "get_session_plan/parent-session") {
        return {
          id: "plan-1",
          content:
            "## Goal\nFlip merge validation default.\n\n## Affected Files\n- `src-tauri/foo.rs` — update existing code.\n",
        };
      }
      if (endpoint.startsWith("team/verification-findings/")) {
        return {
          findings: [],
          count: 0,
        };
      }
      throw new Error(`unexpected endpoint ${endpoint}`);
    });

    const runtime = createVerificationRuntime({
      callTauri,
      callTauriGet,
      agentType: "ralphx-plan-verifier",
      contextType: "ideation",
      contextId: "child-session",
    });

    const enrichmentPromise = runtime.runVerificationEnrichment({
      selected_specialists: ["intent"],
    });

    await vi.advanceTimersByTimeAsync(21_000);
    const result = await enrichmentPromise;

    expect(result).toMatchObject({
      timed_out: false,
      requested_specialists: ["intent"],
      findings_by_critic: [
        {
          critic: "intent",
          found: false,
        },
      ],
    });
  });

  it("keeps timed-out required critics pending when they are still running and clamps the wait budget to the tool-safe cap", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-04-13T16:35:53.000Z"));

    const callTauri = vi.fn(async (endpoint: string) => {
      if (endpoint === "coordination/delegate/wait") {
        return {
          job_id: "job-1",
          status: "running",
          delegated_status: {
            agent_state: {
              estimated_status: "running",
            },
            latest_run: {
              status: "running",
            },
          },
        };
      }
      throw new Error(`unexpected endpoint ${endpoint}`);
    });
    const callTauriGet = vi.fn(async (endpoint: string) => {
      if (endpoint.startsWith("team/verification-findings/")) {
        return {
          findings: [],
          count: 0,
        };
      }
      throw new Error(`unexpected endpoint ${endpoint}`);
    });

    const runtime = createVerificationRuntime({
      callTauri,
      callTauriGet,
      agentType: "ralphx-plan-verifier",
      contextType: "ideation",
      contextId: "child-session",
    });

    const settlementPromise = runtime.awaitVerificationRoundSettlement({
      session_id: "parent-session",
      delegates: [
        {
          job_id: "job-1",
          critic: "completeness",
          label: "completeness",
          required: true,
        },
      ],
      created_after: "2026-04-13T16:35:54.802Z",
      rescue_budget_exhausted: true,
      include_full_content: false,
      include_messages: false,
      message_limit: 1,
      max_wait_ms: 600000,
      poll_interval_ms: 1000,
    });

    await vi.advanceTimersByTimeAsync(91_000);
    const result = await settlementPromise;

    expect(result).toMatchObject({
      classification: "pending",
      timed_out: true,
      settled: false,
      max_wait_ms: 90000,
      recommended_next_action: "perform_single_rescue_or_wait",
      missing_required_critics: ["completeness"],
    });
  });

  it("waits longer than 15s by default for optional verification specialists during a round", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-04-13T16:35:53.000Z"));

    const callTauri = vi.fn(async (endpoint: string, payload?: Record<string, unknown>) => {
      if (endpoint === "coordination/delegate/start") {
        const agentName = String(payload?.agent_name ?? "");
        if (agentName.includes("specialist-ux")) {
          return {
            job_id: "optional-ux",
            delegated_session_id: "delegated-ux",
            agent_name: agentName,
          };
        }
        return {
          job_id: `${agentName.split(":").pop() ?? "critic"}-job`,
          delegated_session_id: `${agentName.split(":").pop() ?? "critic"}-session`,
          agent_name: agentName,
        };
      }
      if (endpoint === "coordination/delegate/wait") {
        const jobId = String(payload?.job_id ?? "");
        const optionalCompleted = Date.now() >= Date.parse("2026-04-13T16:36:13.000Z");
        if (jobId === "optional-ux") {
          return {
            job_id: jobId,
            status: optionalCompleted ? "completed" : "running",
            label: "ux",
            delegated_status: {
              latest_run: {
                status: optionalCompleted ? "completed" : "running",
              },
              agent_state: {
                estimated_status: optionalCompleted ? "completed" : "running",
              },
            },
          };
        }
        return {
          job_id: jobId,
          status: "completed",
          delegated_status: {
            latest_run: {
              status: "completed",
            },
            agent_state: {
              estimated_status: "completed",
            },
          },
        };
      }
      throw new Error(`unexpected endpoint ${endpoint}`);
    });
    const callTauriGet = vi.fn(async (endpoint: string) => {
      if (endpoint === "parent_session_context/child-session") {
        return {
          parent_session: {
            id: "parent-session",
          },
        };
      }
      if (endpoint === "get_session_plan/parent-session") {
        return {
          id: "plan-1",
          content:
            "## Goal\nShip a UX-visible verification workflow.\n\n## Affected Files\n- `frontend/src/components/Chat/tool-widgets/VerificationWidget.tsx` — update existing UI.\n",
        };
      }
      if (endpoint.startsWith("team/verification-findings/parent-session")) {
        if (endpoint.includes("created_after=")) {
          return {
            findings: [
              {
                artifact_id: "finding-1",
                title: "Completeness finding",
                created_at: "2026-04-13T16:35:54.000Z",
                critic: "completeness",
                round: 1,
                status: "complete",
                summary: "No completeness gaps.",
                gaps: [],
              },
              {
                artifact_id: "finding-2",
                title: "Feasibility finding",
                created_at: "2026-04-13T16:35:54.500Z",
                critic: "feasibility",
                round: 1,
                status: "complete",
                summary: "No feasibility gaps.",
                gaps: [],
              },
            ],
            count: 2,
          };
        }
        return {
          findings: [],
          count: 0,
        };
      }
      throw new Error(`unexpected endpoint ${endpoint}`);
    });

    const runtime = createVerificationRuntime({
      callTauri,
      callTauriGet,
      agentType: "ralphx-plan-verifier",
      contextType: "ideation",
      contextId: "child-session",
    });

    const roundPromise = runtime.runVerificationRound({
      round: 1,
      selected_specialists: ["ux"],
    });

    await vi.runAllTimersAsync();
    const result = await roundPromise as {
      classification: string;
      optional_timed_out: boolean;
      optional_delegate_snapshots: Array<{ job_id: string; status: string }>;
    };

    expect(result.classification).toBe("complete");
    expect(result.optional_timed_out).toBe(false);
    expect(result.optional_delegate_snapshots).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          job_id: "optional-ux",
          status: "completed",
        }),
      ])
    );
  });

  it("routes verifier terminal cleanup with missing round context to infra-failure instead of persisting a zero-gap verdict", async () => {
    const callTauri = vi.fn(async (endpoint: string, payload: Record<string, unknown>) => ({
      endpoint,
      payload,
    }));
    const callTauriGet = vi.fn(async (endpoint: string) => {
      if (endpoint === "parent_session_context/child-session") {
        return {
          parent_session: {
            id: "parent-session",
          },
        };
      }
      throw new Error(`unexpected endpoint ${endpoint}`);
    });

    const runtime = createVerificationRuntime({
      callTauri,
      callTauriGet,
      agentType: "ralphx-plan-verifier",
      contextType: "ideation",
      contextId: "child-session",
    });

    const result = await runtime.completePlanVerificationForTool({
      session_id: "wrong-session",
      status: "needs_revision",
      convergence_reason: "agent_error",
      generation: 6,
      created_after: "2026-04-13T16:35:54.802Z",
    } as unknown as {
      session_id?: string;
      status: string;
      convergence_reason?: string;
      generation: number;
    });

    expect(callTauri).toHaveBeenCalledTimes(1);
    expect(callTauri).toHaveBeenCalledWith(
      "ideation/sessions/parent-session/verification/infra-failure",
      {
        generation: 6,
        convergence_reason: "agent_error",
        round: undefined,
      }
    );
    expect(result).toMatchObject({
      endpoint: "ideation/sessions/parent-session/verification/infra-failure",
    });
  });

  it("rejects actionable needs_revision cleanup without a terminal convergence reason", async () => {
    const callTauri = vi.fn();
    const callTauriGet = vi.fn(async (endpoint: string) => {
      if (endpoint === "parent_session_context/child-session") {
        return {
          parent_session: {
            id: "parent-session",
          },
        };
      }
      throw new Error(`unexpected endpoint ${endpoint}`);
    });

    const runtime = createVerificationRuntime({
      callTauri,
      callTauriGet,
      agentType: "ralphx-plan-verifier",
      contextType: "ideation",
      contextId: "child-session",
    });

    runtime.rememberVerificationRoundState("parent-session", {
      round: 1,
      classification: "complete",
      createdAfter: "2026-04-13T17:24:37.913Z",
      mergedGaps: [
        { severity: "medium", category: "testing", description: "Missing repo regression" },
      ],
      requiredDelegates: [
        {
          job_id: "job-1",
          critic: "completeness",
          label: "completeness",
          required: true,
        },
      ],
    });

    await expect(
      runtime.completePlanVerificationForTool({
        session_id: "wrong-session",
        status: "needs_revision",
        generation: 7,
        round: 1,
      })
    ).rejects.toThrow(
      "complete_plan_verification cannot finalize an actionable needs_revision result without a terminal convergence_reason"
    );

    expect(callTauri).not.toHaveBeenCalled();
  });

  it("ignores model-supplied settlement fields and uses cached round state for terminal cleanup", async () => {
    const callTauri = vi.fn(async (endpoint: string, payload: Record<string, unknown>) => {
      if (endpoint === "coordination/delegate/wait") {
        return {
          job_id: "job-1",
          status: "completed",
          delegated_status: {
            latest_run: {
              status: "completed",
            },
            agent_state: {
              estimated_status: "completed",
            },
          },
        };
      }

      return {
        endpoint,
        payload,
      };
    });
    const callTauriGet = vi.fn(async (endpoint: string) => {
      if (endpoint === "parent_session_context/child-session") {
        return {
          parent_session: {
            id: "parent-session",
          },
        };
      }

      if (endpoint.startsWith("team/verification-findings/")) {
        return {
          findings: [
            {
              artifact_id: "finding-1",
              title: "Completeness: Round 1",
              critic: "completeness",
              round: 1,
              created_at: "2026-04-13T17:25:41.875717+00:00",
              status: "complete",
              summary: "No blockers.",
              gaps: [],
            },
          ],
          count: 1,
        };
      }

      throw new Error(`unexpected endpoint ${endpoint}`);
    });

    const runtime = createVerificationRuntime({
      callTauri,
      callTauriGet,
      agentType: "ralphx-plan-verifier",
      contextType: "ideation",
      contextId: "child-session",
    });

    runtime.rememberVerificationRoundState("parent-session", {
      round: 1,
      classification: "complete",
      createdAfter: "2026-04-13T17:24:37.913Z",
      mergedGaps: [{ severity: "high", category: "ignored", description: "should not be used" }],
      requiredDelegates: [
        {
          job_id: "job-1",
          critic: "completeness",
          label: "completeness",
          required: true,
        },
      ],
    });

    const result = await runtime.completePlanVerificationForTool({
      session_id: "wrong-session",
      status: "verified",
      convergence_reason: "zero_blocking",
      generation: 7,
      round: 1,
      required_delegates: [
        {
          job_id: "wrong-job",
          critic: "wrong",
          label: "wrong",
          required: true,
        },
      ] as any,
      created_after: "wrong-created-after",
      rescue_budget_exhausted: false,
    } as unknown as {
      session_id?: string;
      status: string;
      convergence_reason?: string;
      generation: number;
      round?: number;
    });

    expect(callTauri).toHaveBeenCalledWith(
      "ideation/sessions/parent-session/verification",
      expect.objectContaining({
        status: "verified",
        in_progress: false,
        generation: 7,
        round: 1,
        gaps: [],
      })
    );
    expect(result).toMatchObject({
      endpoint: "ideation/sessions/parent-session/verification",
    });
  });
});
