import { describe, expect, it } from "vitest";
import { normalizePermissionToolInput } from "../permission-handler.js";

describe("normalizePermissionToolInput", () => {
  it("adds snake_case and path aliases for Write requests", () => {
    expect(
      normalizePermissionToolInput("Write", {
        filePath: "/tmp/out.md",
        content: "hello",
      })
    ).toEqual({
      filePath: "/tmp/out.md",
      file_path: "/tmp/out.md",
      path: "/tmp/out.md",
      content: "hello",
    });
  });

  it("maps Read path into file_path aliases", () => {
    expect(
      normalizePermissionToolInput("Read", {
        path: "/tmp/input.md",
      })
    ).toEqual({
      path: "/tmp/input.md",
      file_path: "/tmp/input.md",
      filePath: "/tmp/input.md",
    });
  });

  it("adds snake_case aliases for Edit camelCase fields", () => {
    expect(
      normalizePermissionToolInput("Edit", {
        filePath: "/tmp/file.ts",
        oldString: "before",
        newString: "after",
      })
    ).toEqual({
      filePath: "/tmp/file.ts",
      file_path: "/tmp/file.ts",
      path: "/tmp/file.ts",
      oldString: "before",
      old_string: "before",
      newString: "after",
      new_string: "after",
    });
  });
});
