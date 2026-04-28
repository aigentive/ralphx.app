import { describe, expect, it } from "vitest";

import {
  migrateAgentSessionStore,
  useAgentSessionStore,
} from "./agentSessionStore";

describe("agentSessionStore", () => {
  it("defaults the Agents sidebar to all projects", () => {
    expect(useAgentSessionStore.getInitialState().showAllProjects).toBe(true);
  });

  it("migrates older persisted sidebar filter state to all projects", () => {
    expect(
      migrateAgentSessionStore(
        {
          showAllProjects: false,
          projectSort: "latest",
        },
        0,
      ),
    ).toMatchObject({
      showAllProjects: true,
    });
  });

  it("preserves current persisted sidebar filter state", () => {
    expect(
      migrateAgentSessionStore(
        {
          showAllProjects: false,
          projectSort: "latest",
        },
        1,
      ),
    ).toMatchObject({
      showAllProjects: false,
    });
  });
});
