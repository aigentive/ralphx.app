import { z } from "zod";

export const featureFlagsSchema = z.object({
  activityPage: z.boolean(),
  extensibilityPage: z.boolean(),
});

export type FeatureFlags = z.infer<typeof featureFlagsSchema>;
