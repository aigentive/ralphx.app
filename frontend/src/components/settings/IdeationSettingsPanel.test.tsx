/**
 * IdeationSettingsPanel Tests
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { IdeationSettingsPanel } from "./IdeationSettingsPanel";
import { ideationApi } from "@/api/ideation";
import type { IdeationSettings } from "@/types/ideation-config";

// Mock the ideation API
vi.mock("@/api/ideation", () => ({
  ideationApi: {
    settings: {
      get: vi.fn(),
      update: vi.fn(),
    },
  },
}));

const defaultSettings: IdeationSettings = {
  planMode: "optional",
  requirePlanApproval: false,
  suggestPlansForComplex: true,
  autoLinkProposals: true,
};

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  return ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
}

describe("IdeationSettingsPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(ideationApi.settings.get).mockResolvedValue(defaultSettings);
  });

  it("renders section with Lightbulb icon and title", async () => {
    render(<IdeationSettingsPanel />, { wrapper: createWrapper() });

    expect(screen.getByText("Ideation")).toBeInTheDocument();
    expect(screen.getByText("Configure implementation plan workflow")).toBeInTheDocument();
  });

  it("renders all plan mode options", async () => {
    render(<IdeationSettingsPanel />, { wrapper: createWrapper() });

    await waitFor(() => {
      expect(screen.getByTestId("plan-mode-required")).toBeInTheDocument();
      expect(screen.getByTestId("plan-mode-optional")).toBeInTheDocument();
      expect(screen.getByTestId("plan-mode-parallel")).toBeInTheDocument();
    });
  });

  it("renders all checkbox settings", async () => {
    render(<IdeationSettingsPanel />, { wrapper: createWrapper() });

    await waitFor(() => {
      expect(screen.getByTestId("require-plan-approval")).toBeInTheDocument();
      expect(screen.getByTestId("suggest-plans-for-complex")).toBeInTheDocument();
      expect(screen.getByTestId("auto-link-proposals")).toBeInTheDocument();
    });
  });

  it("selects the correct plan mode based on settings", async () => {
    vi.mocked(ideationApi.settings.get).mockResolvedValue({
      ...defaultSettings,
      planMode: "required",
    });

    render(<IdeationSettingsPanel />, { wrapper: createWrapper() });

    await waitFor(() => {
      const requiredRadio = screen.getByTestId("plan-mode-required");
      expect(requiredRadio).toBeChecked();
    });
  });

  it("disables 'require plan approval' when not in Required mode", async () => {
    vi.mocked(ideationApi.settings.get).mockResolvedValue({
      ...defaultSettings,
      planMode: "optional",
    });

    render(<IdeationSettingsPanel />, { wrapper: createWrapper() });

    await waitFor(() => {
      const checkbox = screen.getByTestId("require-plan-approval");
      expect(checkbox).toBeDisabled();
    });
  });

  it("enables 'require plan approval' when in Required mode", async () => {
    vi.mocked(ideationApi.settings.get).mockResolvedValue({
      ...defaultSettings,
      planMode: "required",
    });

    render(<IdeationSettingsPanel />, { wrapper: createWrapper() });

    await waitFor(() => {
      const checkbox = screen.getByTestId("require-plan-approval");
      expect(checkbox).not.toBeDisabled();
    });
  });

  it("calls update when plan mode changes", async () => {
    const user = userEvent.setup();
    vi.mocked(ideationApi.settings.update).mockResolvedValue({
      ...defaultSettings,
      planMode: "required",
    });

    render(<IdeationSettingsPanel />, { wrapper: createWrapper() });

    await waitFor(() => {
      expect(screen.getByTestId("plan-mode-required")).toBeInTheDocument();
    });

    const requiredRadio = screen.getByTestId("plan-mode-required");
    await user.click(requiredRadio);

    await waitFor(() => {
      expect(ideationApi.settings.update).toHaveBeenCalledWith(
        expect.objectContaining({
          planMode: "required",
        })
      );
    });
  });

  it("calls update when checkbox is toggled", async () => {
    const user = userEvent.setup();
    vi.mocked(ideationApi.settings.update).mockResolvedValue({
      ...defaultSettings,
      suggestPlansForComplex: false,
    });

    render(<IdeationSettingsPanel />, { wrapper: createWrapper() });

    await waitFor(() => {
      expect(screen.getByTestId("suggest-plans-for-complex")).toBeInTheDocument();
    });

    const checkbox = screen.getByTestId("suggest-plans-for-complex");
    await user.click(checkbox);

    await waitFor(() => {
      expect(ideationApi.settings.update).toHaveBeenCalledWith(
        expect.objectContaining({
          suggestPlansForComplex: false,
        })
      );
    });
  });

  it("reflects checkbox state from settings", async () => {
    vi.mocked(ideationApi.settings.get).mockResolvedValue({
      ...defaultSettings,
      suggestPlansForComplex: false,
      autoLinkProposals: false,
    });

    render(<IdeationSettingsPanel />, { wrapper: createWrapper() });

    await waitFor(() => {
      const suggestCheckbox = screen.getByTestId("suggest-plans-for-complex");
      const autoLinkCheckbox = screen.getByTestId("auto-link-proposals");
      expect(suggestCheckbox).not.toBeChecked();
      expect(autoLinkCheckbox).not.toBeChecked();
    });
  });
});
