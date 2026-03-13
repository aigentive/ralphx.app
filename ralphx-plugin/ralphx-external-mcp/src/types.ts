/**
 * Shared types for ralphx-external-mcp
 */

/** Validated API key context — populated after successful auth */
export interface ApiKeyContext {
  keyId: string;
  projectIds: string[];
  permissions: number; // bitmask: 1=read, 2=write, 4=admin
}

/** External MCP server configuration */
export interface ExternalMcpConfig {
  /** Port to bind on (default: 3848) */
  port: number;
  /** Host to bind on (default: 127.0.0.1) */
  host: string;
  /** Backend URL (default: http://127.0.0.1:3847) */
  backendUrl: string;
  /** TLS config (required when host != 127.0.0.1) */
  tls?: TlsConfig;
  /** Rate limiter config */
  rateLimit?: RateLimitConfig;
}

export interface TlsConfig {
  certPath: string;
  keyPath: string;
}

export interface RateLimitConfig {
  /** Max requests per second per API key (default: 10) */
  requestsPerSecond: number;
  /** Max concurrent connections (default: 50) */
  maxConnections: number;
  /** Auth failures before IP lockout (default: 5) */
  authFailuresBeforeLockout: number;
  /** Lockout duration in seconds (default: 30) */
  lockoutDurationSecs: number;
  /** Max external ideation sessions (default: 1) */
  maxExternalIdeationSessions: number;
}

/** Validate key response from :3847/api/auth/validate-key */
export interface ValidateKeyResponse {
  key_id: string;
  project_ids: string[];
  permissions: number;
}

/** Permission bitmask constants */
export const Permission = {
  READ: 1,
  WRITE: 2,
  ADMIN: 4,
  CREATE_PROJECT: 8,
} as const;

export function hasPermission(permissions: number, flag: number): boolean {
  return (permissions & flag) !== 0;
}
