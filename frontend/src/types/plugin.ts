import { z } from 'zod';

/**
 * Plugin author schema
 */
export const PluginAuthorSchema = z.object({
  name: z.string().min(1),
  email: z.string().email().optional(),
  url: z.string().url().optional(),
});

export type PluginAuthor = z.infer<typeof PluginAuthorSchema>;

/**
 * RalphX Plugin manifest schema (plugin.json)
 */
export const PluginManifestSchema = z.object({
  name: z.string().min(1),
  description: z.string().min(1),
  version: z.string().regex(/^\d+\.\d+\.\d+$/, 'Version must be semver (e.g., 1.0.0)'),
  author: PluginAuthorSchema,
  agents: z.string().optional(),
  skills: z.string().optional(),
  hooks: z.string().optional(),
  mcpServers: z.string().optional(),
});

export type PluginManifest = z.infer<typeof PluginManifestSchema>;

/**
 * Parse and validate a plugin manifest
 */
export function parsePluginManifest(json: unknown): PluginManifest {
  return PluginManifestSchema.parse(json);
}

/**
 * Safely parse a plugin manifest, returning null on failure
 */
export function safeParsePluginManifest(json: unknown): PluginManifest | null {
  const result = PluginManifestSchema.safeParse(json);
  return result.success ? result.data : null;
}
