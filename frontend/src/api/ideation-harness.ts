import { typedInvoke } from "@/lib/tauri";
import { z } from "zod";

export const IdeationLaneSchema = z.enum([
  "ideation_primary",
  "ideation_verifier",
  "ideation_subagent",
  "ideation_verifier_subagent",
]);

export type IdeationLane = z.infer<typeof IdeationLaneSchema>;

export const HarnessSchema = z.enum(["claude", "codex"]);

export type Harness = z.infer<typeof HarnessSchema>;

export const AgentLaneSettingsResponseSchema = z.object({
  projectId: z.string().nullable().optional(),
  lane: IdeationLaneSchema,
  harness: HarnessSchema,
  model: z.string().nullable().optional(),
  effort: z.string().nullable().optional(),
  approvalPolicy: z.string().nullable().optional(),
  sandboxMode: z.string().nullable().optional(),
  fallbackHarness: HarnessSchema.nullable().optional(),
  updatedAt: z.string(),
});

export type AgentLaneSettingsResponse = z.infer<
  typeof AgentLaneSettingsResponseSchema
>;

export const IdeationHarnessAvailabilityResponseSchema = z.object({
  projectId: z.string().nullable().optional(),
  lane: IdeationLaneSchema,
  configuredHarness: HarnessSchema.nullable().optional(),
  fallbackHarness: HarnessSchema.nullable().optional(),
  effectiveHarness: HarnessSchema,
  fallbackActivated: z.boolean(),
  binaryPath: z.string().nullable().optional(),
  binaryFound: z.boolean(),
  probeSucceeded: z.boolean(),
  available: z.boolean(),
  missingCoreExecFeatures: z.array(z.string()),
  error: z.string().nullable().optional(),
});

export type IdeationHarnessAvailabilityResponse = z.infer<
  typeof IdeationHarnessAvailabilityResponseSchema
>;

export interface IdeationHarnessLaneView {
  lane: IdeationLane;
  row: AgentLaneSettingsResponse | null;
  configuredHarness: Harness | null;
  effectiveHarness: Harness;
  fallbackHarness: Harness | null;
  fallbackActivated: boolean;
  binaryPath: string | null;
  binaryFound: boolean;
  probeSucceeded: boolean;
  available: boolean;
  missingCoreExecFeatures: string[];
  error: string | null;
}

export interface UpdateIdeationHarnessLaneInput {
  projectId: string | null;
  lane: IdeationLane;
  harness: Harness;
  model?: string | null;
  effort?: string | null;
  approvalPolicy?: string | null;
  sandboxMode?: string | null;
  fallbackHarness?: Harness | null;
}

export const IDEATION_LANES: IdeationLane[] = [
  "ideation_primary",
  "ideation_verifier",
  "ideation_subagent",
  "ideation_verifier_subagent",
];

export const defaultIdeationHarnessLanes: IdeationHarnessLaneView[] =
  IDEATION_LANES.map((lane) => ({
    lane,
    row: null,
    configuredHarness: null,
    effectiveHarness: "claude",
    fallbackHarness: null,
    fallbackActivated: false,
    binaryPath: null,
    binaryFound: false,
    probeSucceeded: false,
    available: false,
    missingCoreExecFeatures: [],
    error: null,
  }));

export function mergeIdeationHarnessState(
  rows: AgentLaneSettingsResponse[],
  availability: IdeationHarnessAvailabilityResponse[],
): IdeationHarnessLaneView[] {
  return IDEATION_LANES.map((lane) => {
    const row = rows.find((entry) => entry.lane === lane) ?? null;
    const status = availability.find((entry) => entry.lane === lane);

    return {
      lane,
      row,
      configuredHarness:
        row?.harness ?? status?.configuredHarness ?? null,
      effectiveHarness:
        status?.effectiveHarness ?? row?.harness ?? "claude",
      fallbackHarness:
        row?.fallbackHarness ?? status?.fallbackHarness ?? null,
      fallbackActivated: status?.fallbackActivated ?? false,
      binaryPath: status?.binaryPath ?? null,
      binaryFound: status?.binaryFound ?? false,
      probeSucceeded: status?.probeSucceeded ?? false,
      available: status?.available ?? false,
      missingCoreExecFeatures: status?.missingCoreExecFeatures ?? [],
      error: status?.error ?? null,
    };
  });
}

export const ideationHarnessApi = {
  async get(projectId: string | null): Promise<IdeationHarnessLaneView[]> {
    const [rows, availability] = await Promise.all([
      typedInvoke(
        "get_agent_lane_settings",
        { projectId },
        z.array(AgentLaneSettingsResponseSchema),
      ),
      typedInvoke(
        "get_ideation_harness_availability",
        { projectId },
        z.array(IdeationHarnessAvailabilityResponseSchema),
      ),
    ]);

    return mergeIdeationHarnessState(rows, availability);
  },

  update(
    input: UpdateIdeationHarnessLaneInput,
  ): Promise<AgentLaneSettingsResponse> {
    return typedInvoke(
      "update_agent_lane_settings",
      { input },
      AgentLaneSettingsResponseSchema,
    );
  },
} as const;
