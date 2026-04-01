import { z } from "zod";

export const featureFlagsSchema = z.object({
  activityPage: z.boolean(),
  extensibilityPage: z.boolean(),
  battleMode: z.boolean().default(true),
});

export type FeatureFlags = z.infer<typeof featureFlagsSchema>;
