import { describe, it, expect } from "vitest";
import {
  AskUserQuestionOptionSchema,
  AskUserQuestionPayloadSchema,
  AskUserQuestionResponseSchema,
  AskUserQuestionPayloadListSchema,
  hasSelection,
  hasCustomResponse,
  isValidResponse,
  createSingleSelectResponse,
  createMultiSelectResponse,
  createCustomResponse,
} from "./ask-user-question";

describe("AskUserQuestionOptionSchema", () => {
  const validOption = {
    label: "JWT tokens",
    description: "Use JSON Web Tokens for authentication",
  };

  it("should parse a valid option", () => {
    expect(() => AskUserQuestionOptionSchema.parse(validOption)).not.toThrow();
  });

  it("should parse an option with minimal fields", () => {
    const minOption = {
      label: "Option A",
      description: "",
    };
    expect(() => AskUserQuestionOptionSchema.parse(minOption)).not.toThrow();
  });

  it("should reject option with empty label", () => {
    expect(() =>
      AskUserQuestionOptionSchema.parse({ ...validOption, label: "" })
    ).toThrow();
  });

  it("should reject option missing label", () => {
    expect(() =>
      AskUserQuestionOptionSchema.parse({ description: "Test" })
    ).toThrow();
  });

  it("should reject option missing description", () => {
    expect(() =>
      AskUserQuestionOptionSchema.parse({ label: "Test" })
    ).toThrow();
  });
});

describe("AskUserQuestionPayloadSchema", () => {
  const validPayload = {
    taskId: "task-123",
    question: "Which authentication method should we use?",
    header: "Auth method",
    options: [
      { label: "JWT tokens", description: "Use JSON Web Tokens" },
      { label: "Session cookies", description: "Use server-side sessions" },
    ],
    multiSelect: false,
  };

  it("should parse a valid payload", () => {
    expect(() => AskUserQuestionPayloadSchema.parse(validPayload)).not.toThrow();
  });

  it("should parse a multi-select payload", () => {
    const multiSelectPayload = {
      ...validPayload,
      multiSelect: true,
    };
    expect(() => AskUserQuestionPayloadSchema.parse(multiSelectPayload)).not.toThrow();
  });

  it("should parse payload with many options", () => {
    const manyOptions = {
      ...validPayload,
      options: [
        { label: "Option 1", description: "First option" },
        { label: "Option 2", description: "Second option" },
        { label: "Option 3", description: "Third option" },
        { label: "Option 4", description: "Fourth option" },
      ],
    };
    expect(() => AskUserQuestionPayloadSchema.parse(manyOptions)).not.toThrow();
  });

  it("should reject payload with empty taskId", () => {
    expect(() =>
      AskUserQuestionPayloadSchema.parse({ ...validPayload, taskId: "" })
    ).toThrow();
  });

  it("should reject payload with empty question", () => {
    expect(() =>
      AskUserQuestionPayloadSchema.parse({ ...validPayload, question: "" })
    ).toThrow();
  });

  it("should reject payload with empty header", () => {
    expect(() =>
      AskUserQuestionPayloadSchema.parse({ ...validPayload, header: "" })
    ).toThrow();
  });

  it("should reject payload with empty options array", () => {
    expect(() =>
      AskUserQuestionPayloadSchema.parse({ ...validPayload, options: [] })
    ).toThrow();
  });

  it("should reject payload with only one option", () => {
    expect(() =>
      AskUserQuestionPayloadSchema.parse({
        ...validPayload,
        options: [{ label: "Only one", description: "Single option" }],
      })
    ).toThrow();
  });

  it("should reject payload missing required fields", () => {
    expect(() => AskUserQuestionPayloadSchema.parse({})).toThrow();
    expect(() => AskUserQuestionPayloadSchema.parse({ taskId: "task-1" })).toThrow();
  });

  it("should reject payload with non-boolean multiSelect", () => {
    expect(() =>
      AskUserQuestionPayloadSchema.parse({ ...validPayload, multiSelect: "true" })
    ).toThrow();
    expect(() =>
      AskUserQuestionPayloadSchema.parse({ ...validPayload, multiSelect: 1 })
    ).toThrow();
  });
});

describe("AskUserQuestionResponseSchema", () => {
  const validResponse = {
    taskId: "task-123",
    selectedOptions: ["JWT tokens"],
  };

  it("should parse a valid response with single selection", () => {
    expect(() => AskUserQuestionResponseSchema.parse(validResponse)).not.toThrow();
  });

  it("should parse a response with multiple selections", () => {
    const multiResponse = {
      taskId: "task-123",
      selectedOptions: ["Option 1", "Option 2"],
    };
    expect(() => AskUserQuestionResponseSchema.parse(multiResponse)).not.toThrow();
  });

  it("should parse a response with custom response", () => {
    const customResponse = {
      taskId: "task-123",
      selectedOptions: [],
      customResponse: "I want to use a different approach",
    };
    expect(() => AskUserQuestionResponseSchema.parse(customResponse)).not.toThrow();
  });

  it("should parse a response with both selected options and custom response", () => {
    const combinedResponse = {
      taskId: "task-123",
      selectedOptions: ["Option 1"],
      customResponse: "But with some modifications",
    };
    expect(() => AskUserQuestionResponseSchema.parse(combinedResponse)).not.toThrow();
  });

  it("should parse a response with empty selectedOptions", () => {
    const emptySelection = {
      taskId: "task-123",
      selectedOptions: [],
    };
    expect(() => AskUserQuestionResponseSchema.parse(emptySelection)).not.toThrow();
  });

  it("should parse a response without customResponse", () => {
    const parsed = AskUserQuestionResponseSchema.parse(validResponse);
    expect(parsed.customResponse).toBeUndefined();
  });

  it("should reject response with empty taskId", () => {
    expect(() =>
      AskUserQuestionResponseSchema.parse({ ...validResponse, taskId: "" })
    ).toThrow();
  });

  it("should reject response missing taskId", () => {
    expect(() =>
      AskUserQuestionResponseSchema.parse({ selectedOptions: ["test"] })
    ).toThrow();
  });

  it("should reject response missing selectedOptions", () => {
    expect(() =>
      AskUserQuestionResponseSchema.parse({ taskId: "task-123" })
    ).toThrow();
  });

  it("should reject response with non-array selectedOptions", () => {
    expect(() =>
      AskUserQuestionResponseSchema.parse({ ...validResponse, selectedOptions: "Option 1" })
    ).toThrow();
  });
});

describe("AskUserQuestionPayloadListSchema", () => {
  it("should parse empty array", () => {
    expect(AskUserQuestionPayloadListSchema.parse([])).toEqual([]);
  });

  it("should parse array of valid payloads", () => {
    const payloads = [
      {
        taskId: "task-1",
        question: "Question 1?",
        header: "Q1",
        options: [
          { label: "A", description: "Option A" },
          { label: "B", description: "Option B" },
        ],
        multiSelect: false,
      },
      {
        taskId: "task-2",
        question: "Question 2?",
        header: "Q2",
        options: [
          { label: "C", description: "Option C" },
          { label: "D", description: "Option D" },
        ],
        multiSelect: true,
      },
    ];
    expect(() => AskUserQuestionPayloadListSchema.parse(payloads)).not.toThrow();
    expect(AskUserQuestionPayloadListSchema.parse(payloads)).toHaveLength(2);
  });

  it("should reject array with invalid payload", () => {
    const payloads = [
      {
        taskId: "task-1",
        // Missing required fields
      },
    ];
    expect(() => AskUserQuestionPayloadListSchema.parse(payloads)).toThrow();
  });
});

describe("AskUserQuestion helper functions", () => {
  describe("hasSelection", () => {
    it("should return true when options are selected", () => {
      expect(hasSelection({ taskId: "task-1", selectedOptions: ["Option A"] })).toBe(true);
      expect(hasSelection({ taskId: "task-1", selectedOptions: ["A", "B"] })).toBe(true);
    });

    it("should return false when no options are selected", () => {
      expect(hasSelection({ taskId: "task-1", selectedOptions: [] })).toBe(false);
    });
  });

  describe("hasCustomResponse", () => {
    it("should return true when custom response is provided", () => {
      expect(
        hasCustomResponse({
          taskId: "task-1",
          selectedOptions: [],
          customResponse: "My custom answer",
        })
      ).toBe(true);
    });

    it("should return false when custom response is undefined", () => {
      expect(hasCustomResponse({ taskId: "task-1", selectedOptions: [] })).toBe(false);
    });

    it("should return false when custom response is empty string", () => {
      expect(
        hasCustomResponse({
          taskId: "task-1",
          selectedOptions: [],
          customResponse: "",
        })
      ).toBe(false);
    });
  });

  describe("isValidResponse", () => {
    it("should return true when options are selected", () => {
      expect(isValidResponse({ taskId: "task-1", selectedOptions: ["Option A"] })).toBe(true);
    });

    it("should return true when custom response is provided", () => {
      expect(
        isValidResponse({
          taskId: "task-1",
          selectedOptions: [],
          customResponse: "My answer",
        })
      ).toBe(true);
    });

    it("should return true when both are provided", () => {
      expect(
        isValidResponse({
          taskId: "task-1",
          selectedOptions: ["A"],
          customResponse: "Additional info",
        })
      ).toBe(true);
    });

    it("should return false when neither is provided", () => {
      expect(isValidResponse({ taskId: "task-1", selectedOptions: [] })).toBe(false);
      expect(
        isValidResponse({
          taskId: "task-1",
          selectedOptions: [],
          customResponse: "",
        })
      ).toBe(false);
    });
  });

  describe("createSingleSelectResponse", () => {
    it("should create response with single option", () => {
      const response = createSingleSelectResponse("task-123", "Option A");
      expect(response.taskId).toBe("task-123");
      expect(response.selectedOptions).toEqual(["Option A"]);
      expect(response.customResponse).toBeUndefined();
    });
  });

  describe("createMultiSelectResponse", () => {
    it("should create response with multiple options", () => {
      const response = createMultiSelectResponse("task-456", ["A", "B", "C"]);
      expect(response.taskId).toBe("task-456");
      expect(response.selectedOptions).toEqual(["A", "B", "C"]);
      expect(response.customResponse).toBeUndefined();
    });

    it("should create response with empty options array", () => {
      const response = createMultiSelectResponse("task-789", []);
      expect(response.selectedOptions).toEqual([]);
    });
  });

  describe("createCustomResponse", () => {
    it("should create response with custom text", () => {
      const response = createCustomResponse("task-abc", "My custom answer");
      expect(response.taskId).toBe("task-abc");
      expect(response.selectedOptions).toEqual([]);
      expect(response.customResponse).toBe("My custom answer");
    });
  });
});
