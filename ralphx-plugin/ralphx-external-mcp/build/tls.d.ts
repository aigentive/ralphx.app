/**
 * TLS configuration and enforcement for ralphx-external-mcp
 *
 * - TLS is required when binding to 0.0.0.0 (non-localhost)
 * - Server refuses to start without TLS cert+key for non-localhost binds
 */
import type { ServerOptions } from "node:https";
import type { TlsConfig } from "./types.js";
/**
 * Returns true if the given host requires TLS enforcement.
 * Localhost addresses are exempt (trust boundary documented).
 */
export declare function requiresTls(host: string): boolean;
/**
 * Validate TLS configuration for startup.
 * Throws if TLS is required but not configured, or if cert/key files are unreadable.
 */
export declare function validateTlsConfig(host: string, tls?: TlsConfig): void;
/**
 * Build Node.js HTTPS ServerOptions from TLS config.
 * Call validateTlsConfig first to ensure config is valid.
 */
export declare function buildTlsOptions(tls: TlsConfig): ServerOptions;
export declare class TlsError extends Error {
    constructor(message: string);
}
//# sourceMappingURL=tls.d.ts.map