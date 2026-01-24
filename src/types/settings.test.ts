// Settings types tests

import { describe, it, expect } from "vitest";
import {
  ExecutionSettingsSchema,
  ModelSettingsSchema,
  ProjectReviewSettingsSchema,
  SupervisorSettingsSchema,
  ProjectSettingsSchema,
  SettingsProfileSchema,
  DEFAULT_EXECUTION_SETTINGS,
  DEFAULT_MODEL_SETTINGS,
  DEFAULT_PROJECT_REVIEW_SETTINGS,
  DEFAULT_SUPERVISOR_SETTINGS,
  DEFAULT_PROJECT_SETTINGS,
  parseProjectSettings,
  safeParseProjectSettings,
  parseSettingsProfile,
  safeParseSettingsProfile,
} from "./settings";

describe("ExecutionSettingsSchema", () => {
  it("parses valid execution settings", () => {
    const data = {
      max_concurrent_tasks: 4,
      auto_commit: false,
      commit_message_prefix: "fix: ",
      pause_on_failure: false,
      review_before_destructive: true,
    };
    const result = ExecutionSettingsSchema.parse(data);
    expect(result.max_concurrent_tasks).toBe(4);
    expect(result.auto_commit).toBe(false);
    expect(result.commit_message_prefix).toBe("fix: ");
  });

  it("applies defaults for missing fields", () => {
    const result = ExecutionSettingsSchema.parse({});
    expect(result).toEqual(DEFAULT_EXECUTION_SETTINGS);
  });

  it("validates max_concurrent_tasks range", () => {
    expect(() => ExecutionSettingsSchema.parse({ max_concurrent_tasks: 0 })).toThrow();
    expect(() => ExecutionSettingsSchema.parse({ max_concurrent_tasks: 11 })).toThrow();
    expect(ExecutionSettingsSchema.parse({ max_concurrent_tasks: 5 }).max_concurrent_tasks).toBe(5);
  });
});

describe("ModelSettingsSchema", () => {
  it("parses valid model settings", () => {
    const data = {
      model: "opus",
      allow_opus_upgrade: false,
    };
    const result = ModelSettingsSchema.parse(data);
    expect(result.model).toBe("opus");
    expect(result.allow_opus_upgrade).toBe(false);
  });

  it("applies defaults for missing fields", () => {
    const result = ModelSettingsSchema.parse({});
    expect(result).toEqual(DEFAULT_MODEL_SETTINGS);
  });

  it("validates model enum values", () => {
    expect(() => ModelSettingsSchema.parse({ model: "invalid" })).toThrow();
    expect(ModelSettingsSchema.parse({ model: "haiku" }).model).toBe("haiku");
  });
});

describe("ProjectReviewSettingsSchema", () => {
  it("parses valid review settings", () => {
    const data = {
      ai_review_enabled: false,
      ai_review_auto_fix: false,
      require_fix_approval: true,
      require_human_review: true,
      max_fix_attempts: 5,
    };
    const result = ProjectReviewSettingsSchema.parse(data);
    expect(result.ai_review_enabled).toBe(false);
    expect(result.require_human_review).toBe(true);
    expect(result.max_fix_attempts).toBe(5);
  });

  it("applies defaults for missing fields", () => {
    const result = ProjectReviewSettingsSchema.parse({});
    expect(result).toEqual(DEFAULT_PROJECT_REVIEW_SETTINGS);
  });

  it("validates max_fix_attempts range", () => {
    expect(() => ProjectReviewSettingsSchema.parse({ max_fix_attempts: 0 })).toThrow();
    expect(() => ProjectReviewSettingsSchema.parse({ max_fix_attempts: 11 })).toThrow();
    expect(ProjectReviewSettingsSchema.parse({ max_fix_attempts: 7 }).max_fix_attempts).toBe(7);
  });
});

describe("SupervisorSettingsSchema", () => {
  it("parses valid supervisor settings", () => {
    const data = {
      supervisor_enabled: false,
      loop_threshold: 5,
      stuck_timeout: 600,
    };
    const result = SupervisorSettingsSchema.parse(data);
    expect(result.supervisor_enabled).toBe(false);
    expect(result.loop_threshold).toBe(5);
    expect(result.stuck_timeout).toBe(600);
  });

  it("applies defaults for missing fields", () => {
    const result = SupervisorSettingsSchema.parse({});
    expect(result).toEqual(DEFAULT_SUPERVISOR_SETTINGS);
  });

  it("validates loop_threshold range", () => {
    expect(() => SupervisorSettingsSchema.parse({ loop_threshold: 1 })).toThrow();
    expect(() => SupervisorSettingsSchema.parse({ loop_threshold: 11 })).toThrow();
    expect(SupervisorSettingsSchema.parse({ loop_threshold: 6 }).loop_threshold).toBe(6);
  });

  it("validates stuck_timeout range", () => {
    expect(() => SupervisorSettingsSchema.parse({ stuck_timeout: 30 })).toThrow();
    expect(() => SupervisorSettingsSchema.parse({ stuck_timeout: 2000 })).toThrow();
    expect(SupervisorSettingsSchema.parse({ stuck_timeout: 900 }).stuck_timeout).toBe(900);
  });
});

describe("ProjectSettingsSchema", () => {
  it("parses full project settings", () => {
    const data = {
      execution: { max_concurrent_tasks: 3 },
      model: { model: "opus" },
      review: { ai_review_enabled: false },
      supervisor: { supervisor_enabled: false },
    };
    const result = ProjectSettingsSchema.parse(data);
    expect(result.execution.max_concurrent_tasks).toBe(3);
    expect(result.model.model).toBe("opus");
    expect(result.review.ai_review_enabled).toBe(false);
    expect(result.supervisor.supervisor_enabled).toBe(false);
  });

  it("applies all defaults for empty object", () => {
    const result = ProjectSettingsSchema.parse({});
    expect(result).toEqual(DEFAULT_PROJECT_SETTINGS);
  });

  it("applies partial defaults", () => {
    const result = ProjectSettingsSchema.parse({
      execution: { max_concurrent_tasks: 4 },
    });
    expect(result.execution.max_concurrent_tasks).toBe(4);
    expect(result.execution.auto_commit).toBe(true); // default
    expect(result.model).toEqual(DEFAULT_MODEL_SETTINGS);
    expect(result.review).toEqual(DEFAULT_PROJECT_REVIEW_SETTINGS);
    expect(result.supervisor).toEqual(DEFAULT_SUPERVISOR_SETTINGS);
  });
});

describe("SettingsProfileSchema", () => {
  it("parses valid settings profile", () => {
    const data = {
      id: "profile-1",
      name: "Development",
      description: "Settings for development",
      settings: DEFAULT_PROJECT_SETTINGS,
      isDefault: true,
      createdAt: "2026-01-24T10:00:00Z",
      updatedAt: "2026-01-24T11:00:00Z",
    };
    const result = SettingsProfileSchema.parse(data);
    expect(result.id).toBe("profile-1");
    expect(result.name).toBe("Development");
    expect(result.isDefault).toBe(true);
  });

  it("requires id and name", () => {
    expect(() =>
      SettingsProfileSchema.parse({
        settings: DEFAULT_PROJECT_SETTINGS,
        createdAt: "2026-01-24T10:00:00Z",
        updatedAt: "2026-01-24T10:00:00Z",
      })
    ).toThrow();
  });
});

describe("parseProjectSettings", () => {
  it("parses valid data", () => {
    const result = parseProjectSettings({});
    expect(result).toEqual(DEFAULT_PROJECT_SETTINGS);
  });

  it("throws on invalid data", () => {
    expect(() =>
      parseProjectSettings({
        execution: { max_concurrent_tasks: "invalid" },
      })
    ).toThrow();
  });
});

describe("safeParseProjectSettings", () => {
  it("returns settings for valid data", () => {
    const result = safeParseProjectSettings({});
    expect(result).toEqual(DEFAULT_PROJECT_SETTINGS);
  });

  it("returns null for invalid data", () => {
    const result = safeParseProjectSettings({
      execution: { max_concurrent_tasks: "invalid" },
    });
    expect(result).toBeNull();
  });
});

describe("parseSettingsProfile", () => {
  it("parses valid data", () => {
    const result = parseSettingsProfile({
      id: "p1",
      name: "Test",
      settings: {},
      createdAt: "2026-01-24T10:00:00Z",
      updatedAt: "2026-01-24T10:00:00Z",
    });
    expect(result.id).toBe("p1");
    expect(result.settings).toEqual(DEFAULT_PROJECT_SETTINGS);
  });

  it("throws on invalid data", () => {
    expect(() => parseSettingsProfile({})).toThrow();
  });
});

describe("safeParseSettingsProfile", () => {
  it("returns profile for valid data", () => {
    const result = safeParseSettingsProfile({
      id: "p1",
      name: "Test",
      settings: {},
      createdAt: "2026-01-24T10:00:00Z",
      updatedAt: "2026-01-24T10:00:00Z",
    });
    expect(result).not.toBeNull();
    expect(result?.id).toBe("p1");
  });

  it("returns null for invalid data", () => {
    const result = safeParseSettingsProfile({});
    expect(result).toBeNull();
  });
});
