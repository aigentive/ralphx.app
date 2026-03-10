/**
 * useApiKeys - TanStack Query hooks for API key CRUD operations.
 *
 * All calls go directly to http://localhost:3847/api/auth/keys
 * (not through Tauri invoke — the auth endpoints are HTTP-native).
 */

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  ListApiKeysResponseSchema,
  CreateApiKeyResponseSchema,
  RotateApiKeyResponseSchema,
  AuditLogResponseSchema,
  type ApiKey,
  type AuditLogEntry,
} from "@/types/api-key";

const BASE = "http://localhost:3847";

// ============================================================================
// Query Key Factory
// ============================================================================

export const apiKeyKeys = {
  all: ["apiKeys"] as const,
  lists: () => [...apiKeyKeys.all, "list"] as const,
  list: () => [...apiKeyKeys.lists()] as const,
  details: () => [...apiKeyKeys.all, "detail"] as const,
  detail: (id: string) => [...apiKeyKeys.details(), id] as const,
  audit: (id: string) => [...apiKeyKeys.all, "audit", id] as const,
};

// ============================================================================
// Fetchers
// ============================================================================

async function fetchKeys(): Promise<ApiKey[]> {
  const res = await fetch(`${BASE}/api/auth/keys`);
  if (!res.ok) {
    const text = await res.text();
    throw new Error(text || `HTTP ${res.status}`);
  }
  const data: unknown = await res.json();
  const parsed = ListApiKeysResponseSchema.parse(data);
  return parsed.keys;
}

export async function fetchAuditLog(keyId: string): Promise<AuditLogEntry[]> {
  const res = await fetch(`${BASE}/api/auth/keys/${keyId}/audit`);
  if (!res.ok) {
    const text = await res.text();
    throw new Error(text || `HTTP ${res.status}`);
  }
  const data: unknown = await res.json();
  const parsed = AuditLogResponseSchema.parse(data);
  return parsed.entries;
}

async function createKey(payload: {
  name: string;
  project_ids: string[];
  permissions?: number;
}): Promise<{ raw_key: string }> {
  const res = await fetch(`${BASE}/api/auth/keys`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload),
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(text || `HTTP ${res.status}`);
  }
  const data: unknown = await res.json();
  const result = CreateApiKeyResponseSchema.safeParse(data);
  if (!result.success) {
    throw new Error(`Invalid create key response: ${result.error.message}`);
  }
  return { raw_key: result.data.key };
}

async function revokeKey(id: string): Promise<void> {
  const res = await fetch(`${BASE}/api/auth/keys/${id}`, { method: "DELETE" });
  if (!res.ok) {
    const text = await res.text();
    let message = text;
    try {
      const parsed: unknown = JSON.parse(text);
      if (parsed !== null && typeof parsed === "object" && "message" in parsed) {
        const msg = (parsed as { message: unknown }).message;
        if (typeof msg === "string") {
          message = msg;
        }
      }
    } catch {
      // not JSON — use raw text as-is
    }
    throw new Error(message || `HTTP ${res.status}`);
  }
}

async function rotateKey(id: string): Promise<{ raw_key: string }> {
  const res = await fetch(`${BASE}/api/auth/keys/${id}/rotate`, {
    method: "POST",
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(text || `HTTP ${res.status}`);
  }
  const data: unknown = await res.json();
  const result = RotateApiKeyResponseSchema.safeParse(data);
  if (!result.success) {
    throw new Error(`Invalid rotate key response: ${result.error.message}`);
  }
  return { raw_key: result.data.new_key };
}

async function updateKeyProjects(id: string, projectIds: string[]): Promise<void> {
  const res = await fetch(`${BASE}/api/auth/keys/${id}/projects`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ project_ids: projectIds }),
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(text || `HTTP ${res.status}`);
  }
}

export async function updateKeyPermissions(id: string, permissions: number): Promise<void> {
  const res = await fetch(`${BASE}/api/auth/keys/${id}/permissions`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ permissions }),
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(text || `HTTP ${res.status}`);
  }
}

// ============================================================================
// Hooks
// ============================================================================

/** List all API keys */
export function useApiKeys() {
  return useQuery<ApiKey[], Error>({
    queryKey: apiKeyKeys.list(),
    queryFn: fetchKeys,
  });
}

/** Audit log for a single key */
export function useApiKeyAuditLog(keyId: string) {
  return useQuery<AuditLogEntry[], Error>({
    queryKey: apiKeyKeys.audit(keyId),
    queryFn: () => fetchAuditLog(keyId),
    enabled: Boolean(keyId),
  });
}

/** Create a new API key */
export function useCreateApiKey() {
  const qc = useQueryClient();
  return useMutation<
    { raw_key: string },
    Error,
    { name: string; project_ids: string[]; permissions?: number }
  >({
    mutationFn: createKey,
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: apiKeyKeys.list() });
    },
  });
}

/** Revoke (delete) an API key */
export function useRevokeApiKey() {
  const qc = useQueryClient();
  return useMutation<void, Error, string>({
    mutationFn: revokeKey,
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: apiKeyKeys.list() });
    },
  });
}

/** Rotate an API key — returns new raw key once */
export function useRotateApiKey() {
  const qc = useQueryClient();
  return useMutation<{ raw_key: string }, Error, string>({
    mutationFn: rotateKey,
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: apiKeyKeys.list() });
    },
  });
}

/** Update project associations for a key */
export function useUpdateKeyProjects() {
  const qc = useQueryClient();
  return useMutation<void, Error, { id: string; projectIds: string[] }>({
    mutationFn: ({ id, projectIds }) => updateKeyProjects(id, projectIds),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: apiKeyKeys.list() });
    },
  });
}

/** Update permissions bitmask for a key */
export function useUpdateKeyPermissions() {
  const qc = useQueryClient();
  return useMutation<void, Error, { id: string; permissions: number }>({
    mutationFn: ({ id, permissions }) => updateKeyPermissions(id, permissions),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: apiKeyKeys.list() });
    },
  });
}
