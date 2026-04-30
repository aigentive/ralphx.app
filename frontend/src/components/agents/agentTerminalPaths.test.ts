import { describe, expect, it } from "vitest";

import { compactTerminalPath } from "./agentTerminalPaths";

describe("compactTerminalPath", () => {
  it("collapses macOS home directories", () => {
    expect(compactTerminalPath("/Users/alex/ralphx-worktrees/project-a")).toBe(
      "~/ralphx-worktrees/project-a"
    );
  });

  it("collapses Linux home directories", () => {
    expect(compactTerminalPath("/home/alex/ralphx-worktrees/project-a")).toBe(
      "~/ralphx-worktrees/project-a"
    );
  });

  it("leaves non-home paths unchanged", () => {
    expect(compactTerminalPath("/var/tmp/project-a")).toBe("/var/tmp/project-a");
  });
});
