import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import type { IncomingMessage, ServerResponse } from "node:http";
import { handleHealth, handleReady } from "../src/health.js";

function makeReq(): IncomingMessage {
  return {} as IncomingMessage;
}

function makeRes(): {
  res: ServerResponse;
  getStatus: () => number | undefined;
  getBody: () => string | undefined;
} {
  let status: number | undefined;
  let body: string | undefined;
  const res = {
    writeHead: (code: number) => {
      status = code;
    },
    end: (b: string) => {
      body = b;
    },
    headersSent: false,
  } as unknown as ServerResponse;
  return {
    res,
    getStatus: () => status,
    getBody: () => body,
  };
}

describe("handleHealth", () => {
  it("always returns 200", () => {
    const { res, getStatus } = makeRes();
    handleHealth(makeReq(), res);
    expect(getStatus()).toBe(200);
  });

  it("returns status: ok in body", () => {
    const { res, getBody } = makeRes();
    handleHealth(makeReq(), res);
    const parsed = JSON.parse(getBody()!);
    expect(parsed.status).toBe("ok");
  });
});

describe("handleReady", () => {
  beforeEach(() => {
    vi.stubGlobal("fetch", vi.fn());
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("returns 200 when backend is reachable", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({ ok: true })
    );

    const { res, getStatus, getBody } = makeRes();
    await handleReady(makeReq(), res);

    expect(getStatus()).toBe(200);
    const parsed = JSON.parse(getBody()!);
    expect(parsed.status).toBe("ready");
    expect(parsed.backend).toBe("reachable");
  });

  it("returns 503 when backend is unreachable", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockRejectedValue(new Error("ECONNREFUSED"))
    );

    const { res, getStatus, getBody } = makeRes();
    await handleReady(makeReq(), res);

    expect(getStatus()).toBe(503);
    const parsed = JSON.parse(getBody()!);
    expect(parsed.status).toBe("not_ready");
    expect(parsed.backend).toBe("unreachable");
  });

  it("returns 503 when backend returns non-ok status", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({ ok: false, status: 500 })
    );

    const { res, getStatus } = makeRes();
    await handleReady(makeReq(), res);

    expect(getStatus()).toBe(503);
  });
});
