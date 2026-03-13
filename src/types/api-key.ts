/**
 * API Key types and Zod schemas for RalphX external API key management.
 */

import { z } from "zod";

// ============================================================================
// Permissions
// ============================================================================

/** Permission bitmask bits */
export const PERM_READ = 1;
export const PERM_WRITE = 2;
export const PERM_ADMIN = 4;
export const PERM_CREATE_PROJECT = 8;

export function hasPermission(permissions: number, bit: number): boolean {
  return (permissions & bit) !== 0;
}

export function togglePermission(permissions: number, bit: number): number {
  return permissions ^ bit;
}

// ============================================================================
// Zod Schemas
// ============================================================================

// ApiKeySchema matches ApiKeyInfoResponse from Tauri (camelCase via serde rename_all)
export const ApiKeySchema = z.object({
  id: z.string().min(1),
  name: z.string().min(1),
  /** Short prefix shown for identification, e.g. "rxk_live_a3f2" */
  keyPrefix: z.string().min(1),
  /** Bitmask: 1=read, 2=write, 4=admin, 8=create_project */
  permissions: z.number().int().min(0).max(15),
  createdAt: z.string(),
  revokedAt: z.string().nullable(),
  lastUsedAt: z.string().nullable(),
  /** Project IDs this key has access to */
  projectIds: z.array(z.string()).default([]),
});

export type ApiKey = z.infer<typeof ApiKeySchema>;

// AuditLogEntry matches AuditLogEntry domain struct (no camelCase rename — stays snake_case)
export const AuditLogEntrySchema = z.object({
  id: z.number().int(),
  api_key_id: z.string().min(1),
  tool_name: z.string().min(1),
  project_id: z.string().nullable(),
  success: z.boolean(),
  latency_ms: z.number().int().nullable(),
  created_at: z.string(),
});

export type AuditLogEntry = z.infer<typeof AuditLogEntrySchema>;

// ApiKeyCreatedResponseSchema matches ApiKeyCreatedResponse from Tauri (camelCase via serde rename_all)
// Used by create_api_key and rotate_api_key commands
export const ApiKeyCreatedResponseSchema = z.object({
  id: z.string(),
  name: z.string(),
  rawKey: z.string(),
  keyPrefix: z.string(),
  permissions: z.number(),
});

export type ApiKeyCreatedResponse = z.infer<typeof ApiKeyCreatedResponseSchema>;

// ============================================================================
// Parsing Utilities
// ============================================================================

export function parseApiKey(data: unknown): ApiKey {
  return ApiKeySchema.parse(data);
}

export function safeParseApiKey(data: unknown): ApiKey | null {
  const result = ApiKeySchema.safeParse(data);
  return result.success ? result.data : null;
}
