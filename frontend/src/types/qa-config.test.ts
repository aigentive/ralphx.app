import { describe, it, expect } from "vitest";
import {
  QAPrepStatusSchema,
  QATestStatusSchema,
  QASettingsSchema,
  TaskQAConfigSchema,
  QA_PREP_STATUS_VALUES,
  QA_TEST_STATUS_VALUES,
  DEFAULT_QA_SETTINGS,
  DEFAULT_TASK_QA_CONFIG,
  isPrepComplete,
  isPrepFailed,
  isTestTerminal,
  isTestPassed,
  isTestFailed,
  shouldRunQAForCategory,
  requiresQA,
  createTaskQAConfig,
  createInheritedTaskQAConfig,
  parseQASettings,
  safeParseQASettings,
  parseTaskQAConfig,
  safeParseTaskQAConfig,
} from "./qa-config";
import type { QASettings, TaskQAConfig } from "./qa-config";

describe("QAPrepStatus", () => {
  it("should have all expected values", () => {
    expect(QA_PREP_STATUS_VALUES).toEqual([
      "pending",
      "running",
      "completed",
      "failed",
    ]);
  });

  it("should parse valid values", () => {
    expect(QAPrepStatusSchema.parse("pending")).toBe("pending");
    expect(QAPrepStatusSchema.parse("running")).toBe("running");
    expect(QAPrepStatusSchema.parse("completed")).toBe("completed");
    expect(QAPrepStatusSchema.parse("failed")).toBe("failed");
  });

  it("should reject invalid values", () => {
    expect(() => QAPrepStatusSchema.parse("invalid")).toThrow();
    expect(() => QAPrepStatusSchema.parse("")).toThrow();
    expect(() => QAPrepStatusSchema.parse(123)).toThrow();
  });

  it("isPrepComplete returns true only for completed", () => {
    expect(isPrepComplete("pending")).toBe(false);
    expect(isPrepComplete("running")).toBe(false);
    expect(isPrepComplete("completed")).toBe(true);
    expect(isPrepComplete("failed")).toBe(false);
  });

  it("isPrepFailed returns true only for failed", () => {
    expect(isPrepFailed("pending")).toBe(false);
    expect(isPrepFailed("running")).toBe(false);
    expect(isPrepFailed("completed")).toBe(false);
    expect(isPrepFailed("failed")).toBe(true);
  });
});

describe("QATestStatus", () => {
  it("should have all expected values", () => {
    expect(QA_TEST_STATUS_VALUES).toEqual([
      "pending",
      "waiting_for_prep",
      "running",
      "passed",
      "failed",
    ]);
  });

  it("should parse valid values", () => {
    expect(QATestStatusSchema.parse("pending")).toBe("pending");
    expect(QATestStatusSchema.parse("waiting_for_prep")).toBe("waiting_for_prep");
    expect(QATestStatusSchema.parse("running")).toBe("running");
    expect(QATestStatusSchema.parse("passed")).toBe("passed");
    expect(QATestStatusSchema.parse("failed")).toBe("failed");
  });

  it("should reject invalid values", () => {
    expect(() => QATestStatusSchema.parse("invalid")).toThrow();
    expect(() => QATestStatusSchema.parse("waiting")).toThrow();
  });

  it("isTestTerminal returns true for passed and failed", () => {
    expect(isTestTerminal("pending")).toBe(false);
    expect(isTestTerminal("waiting_for_prep")).toBe(false);
    expect(isTestTerminal("running")).toBe(false);
    expect(isTestTerminal("passed")).toBe(true);
    expect(isTestTerminal("failed")).toBe(true);
  });

  it("isTestPassed returns true only for passed", () => {
    expect(isTestPassed("pending")).toBe(false);
    expect(isTestPassed("passed")).toBe(true);
    expect(isTestPassed("failed")).toBe(false);
  });

  it("isTestFailed returns true only for failed", () => {
    expect(isTestFailed("pending")).toBe(false);
    expect(isTestFailed("passed")).toBe(false);
    expect(isTestFailed("failed")).toBe(true);
  });
});

describe("QASettings", () => {
  it("should have correct defaults", () => {
    expect(DEFAULT_QA_SETTINGS).toEqual({
      qa_enabled: true,
      auto_qa_for_ui_tasks: true,
      auto_qa_for_api_tasks: false,
      qa_prep_enabled: true,
      browser_testing_enabled: true,
      browser_testing_url: "http://localhost:1420",
    });
  });

  it("should parse valid settings", () => {
    const settings = QASettingsSchema.parse({
      qa_enabled: true,
      auto_qa_for_ui_tasks: true,
      auto_qa_for_api_tasks: false,
      qa_prep_enabled: true,
      browser_testing_enabled: true,
      browser_testing_url: "http://localhost:3000",
    });
    expect(settings.browser_testing_url).toBe("http://localhost:3000");
  });

  it("should reject invalid URL", () => {
    expect(() =>
      QASettingsSchema.parse({
        qa_enabled: true,
        auto_qa_for_ui_tasks: true,
        auto_qa_for_api_tasks: false,
        qa_prep_enabled: true,
        browser_testing_enabled: true,
        browser_testing_url: "not-a-url",
      })
    ).toThrow();
  });

  it("should require all fields", () => {
    expect(() => QASettingsSchema.parse({})).toThrow();
    expect(() =>
      QASettingsSchema.parse({ qa_enabled: true })
    ).toThrow();
  });

  describe("shouldRunQAForCategory", () => {
    it("returns false when qa_enabled is false", () => {
      const settings: QASettings = { ...DEFAULT_QA_SETTINGS, qa_enabled: false };
      expect(shouldRunQAForCategory(settings, "ui")).toBe(false);
      expect(shouldRunQAForCategory(settings, "api")).toBe(false);
    });

    it("returns true for UI categories when auto_qa_for_ui_tasks is true", () => {
      const settings = DEFAULT_QA_SETTINGS;
      expect(shouldRunQAForCategory(settings, "ui")).toBe(true);
      expect(shouldRunQAForCategory(settings, "component")).toBe(true);
      expect(shouldRunQAForCategory(settings, "feature")).toBe(true);
    });

    it("returns false for API categories when auto_qa_for_api_tasks is false", () => {
      const settings = DEFAULT_QA_SETTINGS;
      expect(shouldRunQAForCategory(settings, "api")).toBe(false);
      expect(shouldRunQAForCategory(settings, "backend")).toBe(false);
      expect(shouldRunQAForCategory(settings, "endpoint")).toBe(false);
    });

    it("returns true for API categories when auto_qa_for_api_tasks is true", () => {
      const settings: QASettings = {
        ...DEFAULT_QA_SETTINGS,
        auto_qa_for_api_tasks: true,
      };
      expect(shouldRunQAForCategory(settings, "api")).toBe(true);
      expect(shouldRunQAForCategory(settings, "backend")).toBe(true);
    });

    it("returns false for unknown categories", () => {
      const settings = DEFAULT_QA_SETTINGS;
      expect(shouldRunQAForCategory(settings, "unknown")).toBe(false);
      expect(shouldRunQAForCategory(settings, "docs")).toBe(false);
      expect(shouldRunQAForCategory(settings, "testing")).toBe(false);
    });
  });

  it("parseQASettings parses valid data", () => {
    const result = parseQASettings(DEFAULT_QA_SETTINGS);
    expect(result).toEqual(DEFAULT_QA_SETTINGS);
  });

  it("safeParseQASettings returns data for valid input", () => {
    const result = safeParseQASettings(DEFAULT_QA_SETTINGS);
    expect(result).toEqual(DEFAULT_QA_SETTINGS);
  });

  it("safeParseQASettings returns null for invalid data", () => {
    const result = safeParseQASettings({ invalid: true });
    expect(result).toBeNull();
  });
});

describe("TaskQAConfig", () => {
  it("should have correct defaults", () => {
    expect(DEFAULT_TASK_QA_CONFIG).toEqual({
      needs_qa: null,
      qa_prep_status: "pending",
      qa_test_status: "pending",
    });
  });

  it("should parse valid config with needs_qa null", () => {
    const config = TaskQAConfigSchema.parse({
      needs_qa: null,
      qa_prep_status: "pending",
      qa_test_status: "pending",
    });
    expect(config.needs_qa).toBeNull();
  });

  it("should parse valid config with needs_qa true", () => {
    const config = TaskQAConfigSchema.parse({
      needs_qa: true,
      qa_prep_status: "completed",
      qa_test_status: "passed",
    });
    expect(config.needs_qa).toBe(true);
    expect(config.qa_prep_status).toBe("completed");
    expect(config.qa_test_status).toBe("passed");
  });

  it("should parse valid config with needs_qa false", () => {
    const config = TaskQAConfigSchema.parse({
      needs_qa: false,
      qa_prep_status: "pending",
      qa_test_status: "pending",
    });
    expect(config.needs_qa).toBe(false);
  });

  it("should reject invalid qa_prep_status", () => {
    expect(() =>
      TaskQAConfigSchema.parse({
        needs_qa: null,
        qa_prep_status: "invalid",
        qa_test_status: "pending",
      })
    ).toThrow();
  });

  it("should reject invalid qa_test_status", () => {
    expect(() =>
      TaskQAConfigSchema.parse({
        needs_qa: null,
        qa_prep_status: "pending",
        qa_test_status: "invalid",
      })
    ).toThrow();
  });

  describe("requiresQA", () => {
    it("returns explicit override when needs_qa is true", () => {
      const config = createTaskQAConfig(true);
      const settings: QASettings = { ...DEFAULT_QA_SETTINGS, qa_enabled: false };
      expect(requiresQA(config, settings, "unknown")).toBe(true);
    });

    it("returns explicit override when needs_qa is false", () => {
      const config = createTaskQAConfig(false);
      const settings = DEFAULT_QA_SETTINGS;
      expect(requiresQA(config, settings, "ui")).toBe(false);
    });

    it("inherits from global settings when needs_qa is null", () => {
      const config = createInheritedTaskQAConfig();
      const settings = DEFAULT_QA_SETTINGS;
      expect(requiresQA(config, settings, "ui")).toBe(true);
      expect(requiresQA(config, settings, "api")).toBe(false);
    });
  });

  describe("createTaskQAConfig", () => {
    it("creates config with needs_qa true", () => {
      const config = createTaskQAConfig(true);
      expect(config.needs_qa).toBe(true);
      expect(config.qa_prep_status).toBe("pending");
      expect(config.qa_test_status).toBe("pending");
    });

    it("creates config with needs_qa false", () => {
      const config = createTaskQAConfig(false);
      expect(config.needs_qa).toBe(false);
    });
  });

  describe("createInheritedTaskQAConfig", () => {
    it("creates config with needs_qa null", () => {
      const config = createInheritedTaskQAConfig();
      expect(config.needs_qa).toBeNull();
      expect(config.qa_prep_status).toBe("pending");
      expect(config.qa_test_status).toBe("pending");
    });
  });

  it("parseTaskQAConfig parses valid data", () => {
    const result = parseTaskQAConfig(DEFAULT_TASK_QA_CONFIG);
    expect(result).toEqual(DEFAULT_TASK_QA_CONFIG);
  });

  it("safeParseTaskQAConfig returns data for valid input", () => {
    const result = safeParseTaskQAConfig(DEFAULT_TASK_QA_CONFIG);
    expect(result).toEqual(DEFAULT_TASK_QA_CONFIG);
  });

  it("safeParseTaskQAConfig returns null for invalid data", () => {
    const result = safeParseTaskQAConfig({ invalid: true });
    expect(result).toBeNull();
  });
});

describe("JSON roundtrip", () => {
  it("QASettings survives JSON serialization", () => {
    const original: QASettings = {
      qa_enabled: true,
      auto_qa_for_ui_tasks: false,
      auto_qa_for_api_tasks: true,
      qa_prep_enabled: false,
      browser_testing_enabled: true,
      browser_testing_url: "http://example.com:8080",
    };
    const json = JSON.stringify(original);
    const parsed = QASettingsSchema.parse(JSON.parse(json));
    expect(parsed).toEqual(original);
  });

  it("TaskQAConfig survives JSON serialization", () => {
    const original: TaskQAConfig = {
      needs_qa: true,
      qa_prep_status: "completed",
      qa_test_status: "passed",
    };
    const json = JSON.stringify(original);
    const parsed = TaskQAConfigSchema.parse(JSON.parse(json));
    expect(parsed).toEqual(original);
  });

  it("TaskQAConfig with null needs_qa survives JSON serialization", () => {
    const original: TaskQAConfig = {
      needs_qa: null,
      qa_prep_status: "running",
      qa_test_status: "waiting_for_prep",
    };
    const json = JSON.stringify(original);
    const parsed = TaskQAConfigSchema.parse(JSON.parse(json));
    expect(parsed).toEqual(original);
  });
});
