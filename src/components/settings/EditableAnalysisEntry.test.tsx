/**
 * Tests for EditableAnalysisEntry component
 *
 * Tests collapsed/expanded states, field editing, array operations,
 * visual indicators, and user-added vs detected entries.
 */

import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, beforeEach, vi } from "vitest";
import { EditableAnalysisEntry } from "./EditableAnalysisEntry";
import type { AnalysisEntry } from "./useAnalysisEditor";

const mockEntry: AnalysisEntry = {
  path: ".",
  label: "Frontend (React/TS)",
  install: "npm install",
  validate: ["npm run typecheck", "npm run lint"],
  worktree_setup: ["ln -s node_modules"],
};

const mockCallbacks = {
  onUpdateField: vi.fn(),
  onResetField: vi.fn(),
  onResetEntry: vi.fn(),
  onAddArrayItem: vi.fn(),
  onRemoveArrayItem: vi.fn(),
  onUpdateArrayItem: vi.fn(),
  isFieldCustomized: vi.fn((_field) => false),
  isUserAdded: false,
};

describe("EditableAnalysisEntry", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("collapsed view", () => {
    it("renders collapsed view by default", () => {
      render(
        <EditableAnalysisEntry
          entry={mockEntry}
          entryIdx={0}
          {...mockCallbacks}
        />
      );

      expect(screen.getByText(".")).toBeInTheDocument();
      expect(screen.getByText("Frontend (React/TS)")).toBeInTheDocument();
    });

    it("shows customization dot when any field is customized", () => {
      const customizedCallbacks = {
        ...mockCallbacks,
        isFieldCustomized: vi.fn((field) => field === "label"),
      };

      const { container } = render(
        <EditableAnalysisEntry
          entry={mockEntry}
          entryIdx={0}
          {...customizedCallbacks}
        />
      );

      // Look for the orange dot (should be present)
      const dots = container.querySelectorAll('[title="This entry has customizations"]');
      expect(dots.length).toBeGreaterThan(0);
    });

    it("does not show customization dot when no fields customized", () => {
      const { container } = render(
        <EditableAnalysisEntry
          entry={mockEntry}
          entryIdx={0}
          {...mockCallbacks}
        />
      );

      const dots = container.querySelectorAll('[title="This entry has customizations"]');
      expect(dots.length).toBe(0);
    });

    it("toggles expansion when clicked", () => {
      const { rerender } = render(
        <EditableAnalysisEntry
          entry={mockEntry}
          entryIdx={0}
          {...mockCallbacks}
        />
      );

      // Initially collapsed - expanded content not visible
      expect(screen.queryByPlaceholderText("e.g., . or src-tauri/")).not.toBeInTheDocument();

      // Click to expand
      const header = screen.getByRole("button", { name: /\./i });
      fireEvent.click(header);

      rerender(
        <EditableAnalysisEntry
          entry={mockEntry}
          entryIdx={0}
          {...mockCallbacks}
        />
      );

      // Now expanded content should be visible
      expect(screen.getByPlaceholderText("e.g., . or src-tauri/")).toBeInTheDocument();
    });
  });

  describe("expanded view - text fields", () => {
    beforeEach(() => {
      const header = screen.queryByRole("button", { name: /\./i });
      if (header && !screen.queryByPlaceholderText("e.g., . or src-tauri/")) {
        fireEvent.click(header);
      }
    });

    it("renders path field with current value", () => {
      render(
        <EditableAnalysisEntry
          entry={mockEntry}
          entryIdx={0}
          {...mockCallbacks}
        />
      );

      // Click to expand first
      const header = screen.getByRole("button", { name: /\./i });
      fireEvent.click(header);

      const pathInput = screen.getByPlaceholderText("e.g., . or src-tauri/");
      expect(pathInput).toHaveValue(".");
    });

    it("calls onUpdateField when path changes", () => {
      render(
        <EditableAnalysisEntry
          entry={mockEntry}
          entryIdx={0}
          {...mockCallbacks}
        />
      );

      // Expand
      fireEvent.click(screen.getByRole("button", { name: /\./i }));

      const pathInput = screen.getByPlaceholderText("e.g., . or src-tauri/");
      fireEvent.change(pathInput, { target: { value: "src-tauri" } });

      expect(mockCallbacks.onUpdateField).toHaveBeenCalledWith(
        "path",
        "src-tauri"
      );
    });

    it("shows reset link only when field is customized", () => {
      const customizedCallbacks = {
        ...mockCallbacks,
        isFieldCustomized: vi.fn((field) => field === "label"),
      };

      render(
        <EditableAnalysisEntry
          entry={mockEntry}
          entryIdx={0}
          {...customizedCallbacks}
        />
      );

      // Expand
      fireEvent.click(screen.getByRole("button", { name: /\./i }));

      // Find all reset buttons and check which ones are visible
      const resetButtons = screen.getAllByText("Reset");
      // Should have multiple reset buttons for the different fields
      expect(resetButtons.length).toBeGreaterThan(0);
    });

    it("calls onResetField when reset link clicked", () => {
      const customizedCallbacks = {
        ...mockCallbacks,
        isFieldCustomized: vi.fn((field) => field === "label"),
      };

      render(
        <EditableAnalysisEntry
          entry={mockEntry}
          entryIdx={0}
          {...customizedCallbacks}
        />
      );

      // Expand
      fireEvent.click(screen.getByRole("button", { name: /\./i }));

      // Find the label reset button (second reset in the component)
      const resetButtons = screen.getAllByText("Reset");
      expect(resetButtons.length).toBeGreaterThan(0);
    });

    it("shows clear button for install field when value is present", () => {
      render(
        <EditableAnalysisEntry
          entry={mockEntry}
          entryIdx={0}
          {...mockCallbacks}
        />
      );

      // Expand
      fireEvent.click(screen.getByRole("button", { name: /\./i }));

      // Clear button should be present for install field
      const clearButtons = screen.getAllByTitle("Clear");
      expect(clearButtons.length).toBeGreaterThan(0);
    });

    it("handles null install field", () => {
      const entryWithoutInstall: AnalysisEntry = {
        ...mockEntry,
        install: null,
      };

      render(
        <EditableAnalysisEntry
          entry={entryWithoutInstall}
          entryIdx={0}
          {...mockCallbacks}
        />
      );

      // Expand
      fireEvent.click(screen.getByRole("button", { name: /\./i }));

      // Clear button should NOT be present when install is null
      const clearButtons = screen.queryAllByTitle("Clear");
      expect(clearButtons.length).toBe(0);
    });
  });

  describe("expanded view - array fields", () => {
    it("renders validate commands list", () => {
      render(
        <EditableAnalysisEntry
          entry={mockEntry}
          entryIdx={0}
          {...mockCallbacks}
        />
      );

      // Expand
      fireEvent.click(screen.getByRole("button", { name: /\./i }));

      // Check for validate command inputs (first 2 of all command inputs)
      const commandInputs = screen.getAllByPlaceholderText("Enter command...");
      expect(commandInputs.length).toBeGreaterThanOrEqual(2); // at least validate + worktree_setup
      expect(commandInputs[0]).toHaveValue("npm run typecheck");
      expect(commandInputs[1]).toHaveValue("npm run lint");
    });

    it("adds validate command when + Add Command clicked", () => {
      render(
        <EditableAnalysisEntry
          entry={mockEntry}
          entryIdx={0}
          {...mockCallbacks}
        />
      );

      // Expand
      fireEvent.click(screen.getByRole("button", { name: /\./i }));

      // Find and click "Add Command" button
      const addButtons = screen.getAllByText(/Add/);
      const addCommandBtn = addButtons.find((btn) => btn.textContent?.includes("Command"));
      expect(addCommandBtn).toBeTruthy();

      if (addCommandBtn) {
        fireEvent.click(addCommandBtn);
        expect(mockCallbacks.onAddArrayItem).toHaveBeenCalledWith("validate");
      }
    });

    it("removes validate command when remove button clicked", () => {
      render(
        <EditableAnalysisEntry
          entry={mockEntry}
          entryIdx={0}
          {...mockCallbacks}
        />
      );

      // Expand
      fireEvent.click(screen.getByRole("button", { name: /\./i }));

      // Find remove buttons (Trash icons)
      const removeButtons = screen.getAllByTitle("Remove");
      expect(removeButtons.length).toBeGreaterThan(0);

      // Click the first remove button
      fireEvent.click(removeButtons[0]);
      expect(mockCallbacks.onRemoveArrayItem).toHaveBeenCalledWith("validate", 0);
    });

    it("renders worktree_setup commands list", () => {
      render(
        <EditableAnalysisEntry
          entry={mockEntry}
          entryIdx={0}
          {...mockCallbacks}
        />
      );

      // Expand
      fireEvent.click(screen.getByRole("button", { name: /\./i }));

      const commandInputs = screen.getAllByPlaceholderText("Enter command...");
      // First 2 are validate, 3rd is worktree_setup
      expect(commandInputs[2]).toHaveValue("ln -s node_modules");
    });

    it("updates array item when value changes", () => {
      render(
        <EditableAnalysisEntry
          entry={mockEntry}
          entryIdx={0}
          {...mockCallbacks}
        />
      );

      // Expand
      fireEvent.click(screen.getByRole("button", { name: /\./i }));

      const commandInputs = screen.getAllByPlaceholderText("Enter command...");
      fireEvent.change(commandInputs[0], { target: { value: "new command" } });

      expect(mockCallbacks.onUpdateArrayItem).toHaveBeenCalledWith(
        "validate",
        0,
        "new command"
      );
    });

    it("shows reset link for array fields when customized", () => {
      const customizedCallbacks = {
        ...mockCallbacks,
        isFieldCustomized: vi.fn((field) => field === "validate"),
      };

      render(
        <EditableAnalysisEntry
          entry={mockEntry}
          entryIdx={0}
          {...customizedCallbacks}
        />
      );

      // Expand
      fireEvent.click(screen.getByRole("button", { name: /\./i }));

      const resetButtons = screen.getAllByText("Reset");
      expect(resetButtons.length).toBeGreaterThan(0);
    });
  });

  describe("visual indicators", () => {
    it("applies border-l-2 to customized fields", () => {
      const customizedCallbacks = {
        ...mockCallbacks,
        isFieldCustomized: vi.fn((field) => field === "label"),
      };

      const { container } = render(
        <EditableAnalysisEntry
          entry={mockEntry}
          entryIdx={0}
          {...customizedCallbacks}
        />
      );

      // Expand
      fireEvent.click(screen.getByRole("button", { name: /\./i }));

      // The container should have elements with the customized border
      const customizedDivs = container.querySelectorAll(".border-l-2");
      expect(customizedDivs.length).toBeGreaterThan(0);
    });
  });

  describe("entry operations", () => {
    it("shows Reset Entry button for detected entries", () => {
      render(
        <EditableAnalysisEntry
          entry={mockEntry}
          entryIdx={0}
          {...mockCallbacks}
        />
      );

      // Expand
      fireEvent.click(screen.getByRole("button", { name: /\./i }));

      const resetButton = screen.getByText("Reset Entry");
      expect(resetButton).toBeInTheDocument();
    });

    it("calls onResetEntry when Reset Entry clicked", () => {
      render(
        <EditableAnalysisEntry
          entry={mockEntry}
          entryIdx={0}
          {...mockCallbacks}
        />
      );

      // Expand
      fireEvent.click(screen.getByRole("button", { name: /\./i }));

      const resetButton = screen.getByText("Reset Entry");
      fireEvent.click(resetButton);

      expect(mockCallbacks.onResetEntry).toHaveBeenCalled();
    });
  });

  describe("empty entries", () => {
    it("handles entry with no label gracefully", () => {
      const emptyLabelEntry: AnalysisEntry = {
        ...mockEntry,
        label: "",
      };

      render(
        <EditableAnalysisEntry
          entry={emptyLabelEntry}
          entryIdx={0}
          {...mockCallbacks}
        />
      );

      expect(screen.getByText("(Unnamed)")).toBeInTheDocument();
    });

    it("handles entry with empty arrays", () => {
      const emptyArrayEntry: AnalysisEntry = {
        path: ".",
        label: "Test",
        install: null,
        validate: [],
        worktree_setup: [],
      };

      render(
        <EditableAnalysisEntry
          entry={emptyArrayEntry}
          entryIdx={0}
          {...mockCallbacks}
        />
      );

      // Expand
      fireEvent.click(screen.getByRole("button", { name: /\./i }));

      // Should only have add buttons, no command inputs
      const commandInputs = screen.queryAllByPlaceholderText("Enter command...");
      expect(commandInputs.length).toBe(0);
    });
  });
});
