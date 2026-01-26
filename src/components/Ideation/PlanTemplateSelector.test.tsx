/**
 * Tests for PlanTemplateSelector component
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { PlanTemplateSelector } from "./PlanTemplateSelector";
import * as methodologiesApi from "@/lib/api/methodologies";

// Mock the methodologies API
vi.mock("@/lib/api/methodologies", () => ({
  getActiveMethodology: vi.fn(),
}));

describe("PlanTemplateSelector", () => {
  const mockOnTemplateSelect = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders nothing when no methodology is active", async () => {
    vi.mocked(methodologiesApi.getActiveMethodology).mockResolvedValue(null);

    const { container } = render(
      <PlanTemplateSelector onTemplateSelect={mockOnTemplateSelect} />
    );

    await waitFor(() => {
      expect(container.firstChild).toBeNull();
    });
  });

  it("renders nothing when methodology has no plan templates", async () => {
    vi.mocked(methodologiesApi.getActiveMethodology).mockResolvedValue({
      id: "test-methodology",
      name: "Test Methodology",
      description: null,
      agent_profiles: [],
      skills: [],
      workflow_id: "workflow-1",
      workflow_name: "Test Workflow",
      phases: [],
      templates: [],
      is_active: true,
      phase_count: 0,
      agent_count: 0,
      created_at: new Date().toISOString(),
      plan_templates: [], // Empty array
    } as unknown as Awaited<ReturnType<typeof methodologiesApi.getActiveMethodology>>);

    const { container } = render(
      <PlanTemplateSelector onTemplateSelect={mockOnTemplateSelect} />
    );

    await waitFor(() => {
      expect(container.firstChild).toBeNull();
    });
  });

  it("renders template selector when methodology has plan templates", async () => {
    vi.mocked(methodologiesApi.getActiveMethodology).mockResolvedValue({
      id: "test-methodology",
      name: "Test Methodology",
      description: null,
      agent_profiles: [],
      skills: [],
      workflow_id: "workflow-1",
      workflow_name: "Test Workflow",
      phases: [],
      templates: [],
      is_active: true,
      phase_count: 0,
      agent_count: 0,
      created_at: new Date().toISOString(),
      plan_templates: [
        {
          id: "template-1",
          name: "Basic Plan",
          description: "A basic plan template",
          template_content: "# Plan\n\nContent here",
        },
        {
          id: "template-2",
          name: "Advanced Plan",
          description: "An advanced plan template",
          template_content: "# Advanced Plan\n\nAdvanced content",
        },
      ],
    } as unknown as Awaited<ReturnType<typeof methodologiesApi.getActiveMethodology>>);

    render(<PlanTemplateSelector onTemplateSelect={mockOnTemplateSelect} />);

    await waitFor(() => {
      expect(screen.getByText("Start from template")).toBeInTheDocument();
    });

    // Check placeholder text
    expect(screen.getByText("Select a template (optional)")).toBeInTheDocument();
  });

  it("renders template options in dropdown", async () => {
    vi.mocked(methodologiesApi.getActiveMethodology).mockResolvedValue({
      id: "test-methodology",
      name: "Test Methodology",
      description: null,
      agent_profiles: [],
      skills: [],
      workflow_id: "workflow-1",
      workflow_name: "Test Workflow",
      phases: [],
      templates: [],
      is_active: true,
      phase_count: 0,
      agent_count: 0,
      created_at: new Date().toISOString(),
      plan_templates: [
        {
          id: "template-1",
          name: "Basic Plan",
          description: "A basic plan template",
          template_content: "# Plan\n\nContent here",
        },
        {
          id: "template-2",
          name: "Advanced Plan",
          description: "An advanced plan template",
          template_content: "# Advanced Plan\n\nAdvanced content",
        },
      ],
    } as unknown as Awaited<ReturnType<typeof methodologiesApi.getActiveMethodology>>);

    render(<PlanTemplateSelector onTemplateSelect={mockOnTemplateSelect} />);

    await waitFor(() => {
      expect(screen.getByText("Start from template")).toBeInTheDocument();
    });

    // Verify combobox is rendered
    const trigger = screen.getByRole("combobox");
    expect(trigger).toBeInTheDocument();
    expect(screen.getByText("Select a template (optional)")).toBeInTheDocument();
  });

  it("disables selector when disabled prop is true", async () => {
    vi.mocked(methodologiesApi.getActiveMethodology).mockResolvedValue({
      id: "test-methodology",
      name: "Test Methodology",
      description: null,
      agent_profiles: [],
      skills: [],
      workflow_id: "workflow-1",
      workflow_name: "Test Workflow",
      phases: [],
      templates: [],
      is_active: true,
      phase_count: 0,
      agent_count: 0,
      created_at: new Date().toISOString(),
      plan_templates: [
        {
          id: "template-1",
          name: "Basic Plan",
          description: "A basic plan template",
          template_content: "# Plan\n\nContent here",
        },
      ],
    } as unknown as Awaited<ReturnType<typeof methodologiesApi.getActiveMethodology>>);

    render(
      <PlanTemplateSelector onTemplateSelect={mockOnTemplateSelect} disabled={true} />
    );

    await waitFor(() => {
      expect(screen.getByText("Start from template")).toBeInTheDocument();
    });

    const trigger = screen.getByRole("combobox");
    expect(trigger).toBeDisabled();
  });

  it("handles API errors gracefully", async () => {
    const consoleErrorSpy = vi.spyOn(console, "error").mockImplementation(() => {});

    vi.mocked(methodologiesApi.getActiveMethodology).mockRejectedValue(
      new Error("API Error")
    );

    const { container } = render(
      <PlanTemplateSelector onTemplateSelect={mockOnTemplateSelect} />
    );

    await waitFor(() => {
      expect(consoleErrorSpy).toHaveBeenCalledWith(
        "Failed to fetch plan templates:",
        expect.any(Error)
      );
    });

    // Should render nothing on error
    expect(container.firstChild).toBeNull();

    consoleErrorSpy.mockRestore();
  });
});
