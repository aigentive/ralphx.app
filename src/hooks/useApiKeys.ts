/**
 * useApiKeys - TanStack Query hooks for API key CRUD operations.
 *
 * All calls go through Tauri invoke() — no HTTP fetch needed.
 * Tauri IPC is inherently trusted (only the webview can call invoke()).
 */

import { invoke } from "@tauri-apps/api/core";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  ApiKeySchema,
  ApiKeyCreatedResponseSchema,
  AuditLogEntrySchema,
  type ApiKey,
  type ApiKeyCreatedResponse,
  type AuditLogEntry,
} from "@/types/api-key";
import { z } from "zod";

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
  const data = await invoke<unknown[]>("list_api_keys");
  return z.array(ApiKeySchema).parse(data);
}

export async function fetchAuditLog(keyId: string): Promise<AuditLogEntry[]> {
  const data = await invoke<unknown[]>("get_api_key_audit_log", { id: keyId });
  return z.array(AuditLogEntrySchema).parse(data);
}

async function createKey(payload: {
  name: string;
  projectIds: string[];
  permissions?: number;
}): Promise<ApiKeyCreatedResponse> {
  const data = await invoke<unknown>("create_api_key", {
    name: payload.name,
    projectIds: payload.projectIds,
    permissions: payload.permissions,
  });
  return ApiKeyCreatedResponseSchema.parse(data);
}

async function revokeKey(id: string): Promise<void> {
  await invoke<void>("revoke_api_key", { id });
}

async function rotateKey(id: string): Promise<ApiKeyCreatedResponse> {
  const data = await invoke<unknown>("rotate_api_key", { id });
  return ApiKeyCreatedResponseSchema.parse(data);
}

async function updateKeyProjects(id: string, projectIds: string[]): Promise<void> {
  await invoke<void>("update_api_key_projects", { id, projectIds });
}

export async function updateKeyPermissions(id: string, permissions: number): Promise<void> {
  await invoke<void>("update_api_key_permissions", { id, permissions });
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
    ApiKeyCreatedResponse,
    Error,
    { name: string; projectIds: string[]; permissions?: number }
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
  return useMutation<ApiKeyCreatedResponse, Error, string>({
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
