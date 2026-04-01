/**
 * TLS configuration and enforcement for ralphx-external-mcp
 *
 * - TLS is required when binding to 0.0.0.0 (non-localhost)
 * - Server refuses to start without TLS cert+key for non-localhost binds
 */
import { readFileSync } from "node:fs";
const LOCALHOST_ADDRESSES = ["127.0.0.1", "::1", "localhost"];
/**
 * Returns true if the given host requires TLS enforcement.
 * Localhost addresses are exempt (trust boundary documented).
 */
export function requiresTls(host) {
    return !LOCALHOST_ADDRESSES.includes(host);
}
/**
 * Validate TLS configuration for startup.
 * Throws if TLS is required but not configured, or if cert/key files are unreadable.
 */
export function validateTlsConfig(host, tls) {
    if (!requiresTls(host)) {
        // Localhost — TLS optional
        return;
    }
    if (!tls) {
        throw new TlsError(`TLS is required when binding to non-localhost address '${host}'. ` +
            "Configure external_mcp.tls.cert_path and external_mcp.tls.key_path. " +
            "Bearer tokens must not travel cleartext on the network.");
    }
    if (!tls.certPath || !tls.keyPath) {
        throw new TlsError("TLS config is incomplete: both cert_path and key_path must be specified.");
    }
    // Verify files are readable at startup
    try {
        readFileSync(tls.certPath);
    }
    catch {
        throw new TlsError(`TLS cert file not readable: ${tls.certPath}`);
    }
    try {
        readFileSync(tls.keyPath);
    }
    catch {
        throw new TlsError(`TLS key file not readable: ${tls.keyPath}`);
    }
}
/**
 * Build Node.js HTTPS ServerOptions from TLS config.
 * Call validateTlsConfig first to ensure config is valid.
 */
export function buildTlsOptions(tls) {
    return {
        cert: readFileSync(tls.certPath),
        key: readFileSync(tls.keyPath),
    };
}
export class TlsError extends Error {
    constructor(message) {
        super(message);
        this.name = "TlsError";
    }
}
//# sourceMappingURL=tls.js.map