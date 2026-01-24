import { describe, it, expect, vi, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";
import { typedInvoke, api, HealthResponseSchema } from "./tauri";

// Cast invoke to a mock function for testing
const mockInvoke = invoke as ReturnType<typeof vi.fn>;

describe("typedInvoke", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("should invoke the command with the given arguments", async () => {
    const schema = z.object({ value: z.number() });
    mockInvoke.mockResolvedValue({ value: 42 });

    await typedInvoke("test_command", { arg1: "test" }, schema);

    expect(mockInvoke).toHaveBeenCalledWith("test_command", { arg1: "test" });
  });

  it("should return validated response when schema matches", async () => {
    const schema = z.object({ value: z.number() });
    mockInvoke.mockResolvedValue({ value: 42 });

    const result = await typedInvoke("test_command", {}, schema);

    expect(result).toEqual({ value: 42 });
  });

  it("should throw when response doesn't match schema", async () => {
    const schema = z.object({ value: z.number() });
    mockInvoke.mockResolvedValue({ value: "not a number" });

    await expect(typedInvoke("test_command", {}, schema)).rejects.toThrow();
  });

  it("should throw when response is missing required fields", async () => {
    const schema = z.object({ required: z.string() });
    mockInvoke.mockResolvedValue({});

    await expect(typedInvoke("test_command", {}, schema)).rejects.toThrow();
  });

  it("should handle null values according to schema", async () => {
    const schema = z.object({ value: z.string().nullable() });
    mockInvoke.mockResolvedValue({ value: null });

    const result = await typedInvoke("test_command", {}, schema);

    expect(result).toEqual({ value: null });
  });

  it("should handle arrays according to schema", async () => {
    const schema = z.array(z.number());
    mockInvoke.mockResolvedValue([1, 2, 3]);

    const result = await typedInvoke("test_command", {}, schema);

    expect(result).toEqual([1, 2, 3]);
  });

  it("should propagate invoke errors", async () => {
    const schema = z.object({ value: z.number() });
    mockInvoke.mockRejectedValue(new Error("Backend error"));

    await expect(typedInvoke("test_command", {}, schema)).rejects.toThrow(
      "Backend error"
    );
  });
});

describe("HealthResponseSchema", () => {
  it("should parse valid health response", () => {
    const response = { status: "ok" };
    expect(() => HealthResponseSchema.parse(response)).not.toThrow();
  });

  it("should reject response without status", () => {
    expect(() => HealthResponseSchema.parse({})).toThrow();
  });

  it("should reject response with non-string status", () => {
    expect(() => HealthResponseSchema.parse({ status: 123 })).toThrow();
  });
});

describe("api.health", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("should call health_check command", async () => {
    mockInvoke.mockResolvedValue({ status: "ok" });

    await api.health.check();

    expect(mockInvoke).toHaveBeenCalledWith("health_check", {});
  });

  it("should return health response", async () => {
    mockInvoke.mockResolvedValue({ status: "ok" });

    const result = await api.health.check();

    expect(result).toEqual({ status: "ok" });
  });

  it("should validate response with HealthResponseSchema", async () => {
    mockInvoke.mockResolvedValue({ status: 123 }); // Invalid

    await expect(api.health.check()).rejects.toThrow();
  });

  it("should propagate backend errors", async () => {
    mockInvoke.mockRejectedValue(new Error("Connection failed"));

    await expect(api.health.check()).rejects.toThrow("Connection failed");
  });
});
