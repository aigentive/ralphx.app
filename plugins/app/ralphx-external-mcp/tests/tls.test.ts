import { describe, it, expect, vi } from "vitest";
import { requiresTls, validateTlsConfig, TlsError } from "../src/tls.js";

// Mock readFileSync for file-system checks
vi.mock("node:fs", () => ({
  readFileSync: vi.fn((path: string) => {
    if (path.includes("missing")) throw new Error("ENOENT");
    return "file-contents";
  }),
}));

describe("requiresTls", () => {
  it("returns false for 127.0.0.1", () => {
    expect(requiresTls("127.0.0.1")).toBe(false);
  });

  it("returns false for ::1", () => {
    expect(requiresTls("::1")).toBe(false);
  });

  it("returns false for localhost", () => {
    expect(requiresTls("localhost")).toBe(false);
  });

  it("returns true for 0.0.0.0", () => {
    expect(requiresTls("0.0.0.0")).toBe(true);
  });

  it("returns true for arbitrary IP", () => {
    expect(requiresTls("192.168.1.100")).toBe(true);
  });
});

describe("validateTlsConfig", () => {
  it("passes for localhost without TLS config", () => {
    expect(() => validateTlsConfig("127.0.0.1", undefined)).not.toThrow();
  });

  it("passes for localhost with TLS config", () => {
    expect(() =>
      validateTlsConfig("127.0.0.1", {
        certPath: "/path/to/cert.pem",
        keyPath: "/path/to/key.pem",
      })
    ).not.toThrow();
  });

  it("throws TlsError for non-localhost without TLS config", () => {
    expect(() => validateTlsConfig("0.0.0.0", undefined)).toThrowError(TlsError);
  });

  it("throws TlsError when certPath is empty", () => {
    expect(() =>
      validateTlsConfig("0.0.0.0", { certPath: "", keyPath: "/path/key.pem" })
    ).toThrowError(TlsError);
  });

  it("throws TlsError when keyPath is empty", () => {
    expect(() =>
      validateTlsConfig("0.0.0.0", { certPath: "/path/cert.pem", keyPath: "" })
    ).toThrowError(TlsError);
  });

  it("throws TlsError when cert file is not readable", () => {
    expect(() =>
      validateTlsConfig("0.0.0.0", {
        certPath: "/path/missing-cert.pem",
        keyPath: "/path/key.pem",
      })
    ).toThrowError(TlsError);
  });

  it("throws TlsError when key file is not readable", () => {
    expect(() =>
      validateTlsConfig("0.0.0.0", {
        certPath: "/path/cert.pem",
        keyPath: "/path/missing-key.pem",
      })
    ).toThrowError(TlsError);
  });

  it("passes when both files are readable for non-localhost", () => {
    expect(() =>
      validateTlsConfig("0.0.0.0", {
        certPath: "/path/cert.pem",
        keyPath: "/path/key.pem",
      })
    ).not.toThrow();
  });
});
