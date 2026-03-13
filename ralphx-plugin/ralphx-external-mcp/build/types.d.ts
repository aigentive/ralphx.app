/**
 * Shared types for ralphx-external-mcp
 */
/** Validated API key context — populated after successful auth */
export interface ApiKeyContext {
    keyId: string;
    projectIds: string[];
    permissions: number;
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
export declare const Permission: {
    readonly READ: 1;
    readonly WRITE: 2;
    readonly ADMIN: 4;
    readonly CREATE_PROJECT: 8;
};
export declare function hasPermission(permissions: number, flag: number): boolean;
//# sourceMappingURL=types.d.ts.map