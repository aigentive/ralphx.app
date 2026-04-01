import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QASettingsPanel } from "./QASettingsPanel";
import type { QASettings } from "@/types/qa-config";
import { DEFAULT_QA_SETTINGS } from "@/types/qa-config";

// Mock useQASettings hook
const mockUpdateSettings = vi.fn();
const mockRefetch = vi.fn();

vi.mock("@/hooks/useQA", () => ({
  useQASettings: vi.fn(() => ({
    settings: DEFAULT_QA_SETTINGS,
    isLoading: false,
    isUpdating: false,
    error: null,
    updateSettings: mockUpdateSettings,
    refetch: mockRefetch,
  })),
}));

import { useQASettings } from "@/hooks/useQA";
const mockUseQASettings = vi.mocked(useQASettings);

function createMockSettings(overrides: Partial<QASettings> = {}): QASettings {
  return { ...DEFAULT_QA_SETTINGS, ...overrides };
}

describe("QASettingsPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockUseQASettings.mockReturnValue({
      settings: DEFAULT_QA_SETTINGS,
      isLoading: false,
      isUpdating: false,
      error: null,
      updateSettings: mockUpdateSettings,
      refetch: mockRefetch,
    });
  });

  describe("rendering", () => {
    it("renders panel title", () => {
      render(<QASettingsPanel />);

      expect(screen.getByText("QA Settings")).toBeInTheDocument();
    });

    it("renders global QA toggle", () => {
      render(<QASettingsPanel />);

      expect(screen.getByTestId("qa-enabled-toggle")).toBeInTheDocument();
      expect(screen.getByText(/enable qa system/i)).toBeInTheDocument();
    });

    it("renders auto-QA for UI tasks checkbox", () => {
      render(<QASettingsPanel />);

      expect(screen.getByTestId("auto-qa-ui-toggle")).toBeInTheDocument();
      expect(screen.getByText(/auto-qa for ui tasks/i)).toBeInTheDocument();
    });

    it("renders auto-QA for API tasks checkbox", () => {
      render(<QASettingsPanel />);

      expect(screen.getByTestId("auto-qa-api-toggle")).toBeInTheDocument();
      expect(screen.getByText(/auto-qa for api tasks/i)).toBeInTheDocument();
    });

    it("renders QA Prep phase toggle", () => {
      render(<QASettingsPanel />);

      expect(screen.getByTestId("qa-prep-toggle")).toBeInTheDocument();
      expect(screen.getByText(/qa prep phase/i)).toBeInTheDocument();
    });

    it("renders browser testing toggle", () => {
      render(<QASettingsPanel />);

      expect(screen.getByTestId("browser-testing-toggle")).toBeInTheDocument();
      expect(screen.getByLabelText(/browser testing$/i)).toBeInTheDocument();
    });

    it("renders browser testing URL input", () => {
      render(<QASettingsPanel />);

      expect(screen.getByTestId("browser-testing-url-input")).toBeInTheDocument();
      expect(screen.getByPlaceholderText(/http:\/\/localhost/i)).toBeInTheDocument();
    });
  });

  describe("initial values", () => {
    it("reflects qa_enabled setting in toggle", () => {
      mockUseQASettings.mockReturnValue({
        settings: createMockSettings({ qa_enabled: true }),
        isLoading: false,
        isUpdating: false,
        error: null,
        updateSettings: mockUpdateSettings,
        refetch: mockRefetch,
      });

      render(<QASettingsPanel />);

      const toggle = screen.getByTestId("qa-enabled-toggle");
      expect(toggle).toHaveAttribute("aria-checked", "true");
    });

    it("reflects qa_enabled=false setting in toggle", () => {
      mockUseQASettings.mockReturnValue({
        settings: createMockSettings({ qa_enabled: false }),
        isLoading: false,
        isUpdating: false,
        error: null,
        updateSettings: mockUpdateSettings,
        refetch: mockRefetch,
      });

      render(<QASettingsPanel />);

      const toggle = screen.getByTestId("qa-enabled-toggle");
      expect(toggle).toHaveAttribute("aria-checked", "false");
    });

    it("reflects auto_qa_for_ui_tasks setting", () => {
      mockUseQASettings.mockReturnValue({
        settings: createMockSettings({ auto_qa_for_ui_tasks: false }),
        isLoading: false,
        isUpdating: false,
        error: null,
        updateSettings: mockUpdateSettings,
        refetch: mockRefetch,
      });

      render(<QASettingsPanel />);

      const toggle = screen.getByTestId("auto-qa-ui-toggle");
      expect(toggle).toHaveAttribute("aria-checked", "false");
    });

    it("reflects browser_testing_url setting in input", () => {
      mockUseQASettings.mockReturnValue({
        settings: createMockSettings({ browser_testing_url: "http://localhost:3000" }),
        isLoading: false,
        isUpdating: false,
        error: null,
        updateSettings: mockUpdateSettings,
        refetch: mockRefetch,
      });

      render(<QASettingsPanel />);

      const input = screen.getByTestId("browser-testing-url-input") as HTMLInputElement;
      expect(input.value).toBe("http://localhost:3000");
    });
  });

  describe("toggle interactions", () => {
    it("calls updateSettings when global toggle is clicked", async () => {
      const user = userEvent.setup();
      render(<QASettingsPanel />);

      await user.click(screen.getByTestId("qa-enabled-toggle"));

      expect(mockUpdateSettings).toHaveBeenCalledWith({ qa_enabled: false });
    });

    it("calls updateSettings when auto-QA UI toggle is clicked", async () => {
      const user = userEvent.setup();
      render(<QASettingsPanel />);

      await user.click(screen.getByTestId("auto-qa-ui-toggle"));

      expect(mockUpdateSettings).toHaveBeenCalledWith({ auto_qa_for_ui_tasks: false });
    });

    it("calls updateSettings when auto-QA API toggle is clicked", async () => {
      const user = userEvent.setup();
      mockUseQASettings.mockReturnValue({
        settings: createMockSettings({ auto_qa_for_api_tasks: false }),
        isLoading: false,
        isUpdating: false,
        error: null,
        updateSettings: mockUpdateSettings,
        refetch: mockRefetch,
      });

      render(<QASettingsPanel />);

      await user.click(screen.getByTestId("auto-qa-api-toggle"));

      expect(mockUpdateSettings).toHaveBeenCalledWith({ auto_qa_for_api_tasks: true });
    });

    it("calls updateSettings when QA prep toggle is clicked", async () => {
      const user = userEvent.setup();
      render(<QASettingsPanel />);

      await user.click(screen.getByTestId("qa-prep-toggle"));

      expect(mockUpdateSettings).toHaveBeenCalledWith({ qa_prep_enabled: false });
    });

    it("calls updateSettings when browser testing toggle is clicked", async () => {
      const user = userEvent.setup();
      render(<QASettingsPanel />);

      await user.click(screen.getByTestId("browser-testing-toggle"));

      expect(mockUpdateSettings).toHaveBeenCalledWith({ browser_testing_enabled: false });
    });
  });

  describe("URL input interactions", () => {
    it("calls updateSettings when URL input loses focus", async () => {
      const user = userEvent.setup();
      render(<QASettingsPanel />);

      const input = screen.getByTestId("browser-testing-url-input");
      await user.clear(input);
      await user.type(input, "http://localhost:3000");
      await user.tab(); // Blur

      expect(mockUpdateSettings).toHaveBeenCalledWith({
        browser_testing_url: "http://localhost:3000",
      });
    });

    it("calls updateSettings when Enter is pressed in URL input", async () => {
      const user = userEvent.setup();
      render(<QASettingsPanel />);

      const input = screen.getByTestId("browser-testing-url-input");
      await user.clear(input);
      await user.type(input, "http://localhost:5000{Enter}");

      expect(mockUpdateSettings).toHaveBeenCalledWith({
        browser_testing_url: "http://localhost:5000",
      });
    });

    it("does not call updateSettings if URL unchanged", async () => {
      const user = userEvent.setup();
      mockUseQASettings.mockReturnValue({
        settings: createMockSettings({ browser_testing_url: "http://localhost:1420" }),
        isLoading: false,
        isUpdating: false,
        error: null,
        updateSettings: mockUpdateSettings,
        refetch: mockRefetch,
      });

      render(<QASettingsPanel />);

      const input = screen.getByTestId("browser-testing-url-input");
      await user.click(input);
      await user.tab(); // Blur without changing

      expect(mockUpdateSettings).not.toHaveBeenCalled();
    });
  });

  describe("disabled states", () => {
    it("disables auto-QA toggles when QA is disabled", () => {
      mockUseQASettings.mockReturnValue({
        settings: createMockSettings({ qa_enabled: false }),
        isLoading: false,
        isUpdating: false,
        error: null,
        updateSettings: mockUpdateSettings,
        refetch: mockRefetch,
      });

      render(<QASettingsPanel />);

      expect(screen.getByTestId("auto-qa-ui-toggle")).toBeDisabled();
      expect(screen.getByTestId("auto-qa-api-toggle")).toBeDisabled();
      expect(screen.getByTestId("qa-prep-toggle")).toBeDisabled();
      expect(screen.getByTestId("browser-testing-toggle")).toBeDisabled();
    });

    it("disables URL input when browser testing is disabled", () => {
      mockUseQASettings.mockReturnValue({
        settings: createMockSettings({ browser_testing_enabled: false }),
        isLoading: false,
        isUpdating: false,
        error: null,
        updateSettings: mockUpdateSettings,
        refetch: mockRefetch,
      });

      render(<QASettingsPanel />);

      expect(screen.getByTestId("browser-testing-url-input")).toBeDisabled();
    });

    it("disables URL input when QA is disabled", () => {
      mockUseQASettings.mockReturnValue({
        settings: createMockSettings({ qa_enabled: false }),
        isLoading: false,
        isUpdating: false,
        error: null,
        updateSettings: mockUpdateSettings,
        refetch: mockRefetch,
      });

      render(<QASettingsPanel />);

      expect(screen.getByTestId("browser-testing-url-input")).toBeDisabled();
    });
  });

  describe("loading state", () => {
    it("shows loading skeleton when isLoading is true", () => {
      mockUseQASettings.mockReturnValue({
        settings: DEFAULT_QA_SETTINGS,
        isLoading: true,
        isUpdating: false,
        error: null,
        updateSettings: mockUpdateSettings,
        refetch: mockRefetch,
      });

      render(<QASettingsPanel />);

      expect(screen.getByTestId("qa-settings-skeleton")).toBeInTheDocument();
    });

    it("disables controls when isUpdating is true", () => {
      mockUseQASettings.mockReturnValue({
        settings: DEFAULT_QA_SETTINGS,
        isLoading: false,
        isUpdating: true,
        error: null,
        updateSettings: mockUpdateSettings,
        refetch: mockRefetch,
      });

      render(<QASettingsPanel />);

      expect(screen.getByTestId("qa-enabled-toggle")).toBeDisabled();
    });
  });

  describe("error state", () => {
    it("shows error message when error is present", () => {
      mockUseQASettings.mockReturnValue({
        settings: DEFAULT_QA_SETTINGS,
        isLoading: false,
        isUpdating: false,
        error: "Failed to save settings",
        updateSettings: mockUpdateSettings,
        refetch: mockRefetch,
      });

      render(<QASettingsPanel />);

      expect(screen.getByText(/failed to save settings/i)).toBeInTheDocument();
    });
  });

  describe("help text", () => {
    it("shows help text for global QA toggle", () => {
      render(<QASettingsPanel />);

      expect(screen.getByText(/master toggle.*qa system/i)).toBeInTheDocument();
    });

    it("shows help text for auto-QA for UI tasks", () => {
      render(<QASettingsPanel />);

      expect(screen.getByText(/automatically enable qa.*ui/i)).toBeInTheDocument();
    });

    it("shows help text for browser testing URL", () => {
      render(<QASettingsPanel />);

      expect(screen.getByText(/url.*dev server/i)).toBeInTheDocument();
    });
  });

  describe("accessibility", () => {
    it("has proper labels for all controls", () => {
      render(<QASettingsPanel />);

      expect(screen.getByLabelText(/enable qa system/i)).toBeInTheDocument();
      expect(screen.getByLabelText(/auto-qa for ui tasks/i)).toBeInTheDocument();
      expect(screen.getByLabelText(/auto-qa for api tasks/i)).toBeInTheDocument();
      expect(screen.getByLabelText(/qa prep phase/i)).toBeInTheDocument();
      expect(screen.getByLabelText(/browser testing$/i)).toBeInTheDocument();
      expect(screen.getByLabelText(/browser testing url/i)).toBeInTheDocument();
    });

    it("has proper aria-describedby for controls with help text", () => {
      render(<QASettingsPanel />);

      const qaToggle = screen.getByTestId("qa-enabled-toggle");
      const describedBy = qaToggle.getAttribute("aria-describedby");
      expect(describedBy).toBeTruthy();
      expect(document.getElementById(describedBy!)).toBeInTheDocument();
    });
  });
});
