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

export function hasPermission(permissions: number, bit: number): boolean {
  return (permissions & bit) !== 0;
}

export function togglePermission(permissions: number, bit: number): number {
  return permissions ^ bit;
}

// ============================================================================
// Zod Schemas
// ============================================================================

export const ApiKeySchema = z.object({
  id: z.string().min(1),
  name: z.string().min(1),
  /** Short prefix shown for identification, e.g. "rxk_live_a3f2" */
  key_prefix: z.string().min(1),
  /** Bitmask: 1=read, 2=write, 4=admin */
  permissions: z.number().int().min(0).max(7),
  created_at: z.string(),
  revoked_at: z.string().nullable(),
  last_used_at: z.string().nullable(),
  /** Project IDs this key has access to */
  project_ids: z.array(z.string()).default([]),
});

export type ApiKey = z.infer<typeof ApiKeySchema>;

export const AuditLogEntrySchema = z.object({
  id: z.number().int(),
  api_key_id: z.string().min(1),
  tool_name: z.string().min(1),
  project_id: z.string().nullable(),
  success: z.number().int(), // 0 or 1
  latency_ms: z.number().int().nullable(),
  created_at: z.string(),
});

export type AuditLogEntry = z.infer<typeof AuditLogEntrySchema>;

export const ListApiKeysResponseSchema = z.object({
  keys: z.array(ApiKeySchema),
  count: z.number(),
});

export type ListApiKeysResponse = z.infer<typeof ListApiKeysResponseSchema>;

export const CreateApiKeyResponseSchema = z.object({
  id: z.string(),
  name: z.string(),
  key: z.string(),
  key_prefix: z.string(),
  permissions: z.number(),
  created_at: z.string(),
});

export type CreateApiKeyResponse = z.infer<typeof CreateApiKeyResponseSchema>;

export const RotateApiKeyResponseSchema = z.object({
  id: z.string(),
  new_key: z.string(),
  key_prefix: z.string(),
  old_key_grace_expires_at: z.string().nullable(),
});

export type RotateApiKeyResponse = z.infer<typeof RotateApiKeyResponseSchema>;

export const AuditLogResponseSchema = z.object({
  entries: z.array(AuditLogEntrySchema),
});

export type AuditLogResponse = z.infer<typeof AuditLogResponseSchema>;

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
