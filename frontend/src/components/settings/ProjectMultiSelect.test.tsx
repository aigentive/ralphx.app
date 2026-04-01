/**
 * Tests for ProjectMultiSelect component
 *
 * Covers: loading state, empty state, project list rendering,
 * toggle selection, disabled state.
 */

import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { ProjectMultiSelect } from "./ProjectMultiSelect";

// Mock useProjects hook
vi.mock("@/hooks/useProjects", () => ({
  useProjects: vi.fn(),
}));

import { useProjects } from "@/hooks/useProjects";

const mockProjects = [
  { id: "proj-1", name: "Alpha Project", workingDirectory: "/alpha" },
  { id: "proj-2", name: "Beta Project", workingDirectory: "/beta" },
  { id: "proj-3", name: "Gamma Project", workingDirectory: "/gamma" },
];

function makeWrapper() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={qc}>{children}</QueryClientProvider>
  );
}

describe("ProjectMultiSelect", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("loading state", () => {
    it("shows spinner while loading", () => {
      vi.mocked(useProjects).mockReturnValue({
        data: undefined,
        isLoading: true,
        error: null,
      } as ReturnType<typeof useProjects>);

      render(<ProjectMultiSelect selectedIds={[]} onChange={vi.fn()} />, {
        wrapper: makeWrapper(),
      });

      // Spinner has animate-spin
      const spinner = document.querySelector(".animate-spin");
      expect(spinner).toBeInTheDocument();
    });
  });

  describe("empty state", () => {
    it("shows 'No projects found' when project list is empty", () => {
      vi.mocked(useProjects).mockReturnValue({
        data: [],
        isLoading: false,
        error: null,
      } as ReturnType<typeof useProjects>);

      render(<ProjectMultiSelect selectedIds={[]} onChange={vi.fn()} />, {
        wrapper: makeWrapper(),
      });

      expect(screen.getByText("No projects found")).toBeInTheDocument();
    });
  });

  describe("project list", () => {
    beforeEach(() => {
      vi.mocked(useProjects).mockReturnValue({
        data: mockProjects,
        isLoading: false,
        error: null,
      } as ReturnType<typeof useProjects>);
    });

    it("renders all projects", () => {
      render(<ProjectMultiSelect selectedIds={[]} onChange={vi.fn()} />, {
        wrapper: makeWrapper(),
      });

      expect(screen.getByText("Alpha Project")).toBeInTheDocument();
      expect(screen.getByText("Beta Project")).toBeInTheDocument();
      expect(screen.getByText("Gamma Project")).toBeInTheDocument();
    });

    it("shows project-multi-select container", () => {
      render(<ProjectMultiSelect selectedIds={[]} onChange={vi.fn()} />, {
        wrapper: makeWrapper(),
      });

      expect(screen.getByTestId("project-multi-select")).toBeInTheDocument();
    });

    it("renders project option test ids", () => {
      render(<ProjectMultiSelect selectedIds={[]} onChange={vi.fn()} />, {
        wrapper: makeWrapper(),
      });

      expect(screen.getByTestId("project-option-proj-1")).toBeInTheDocument();
      expect(screen.getByTestId("project-option-proj-2")).toBeInTheDocument();
    });

    it("calls onChange with added id when unselected project clicked", () => {
      const onChange = vi.fn();
      render(
        <ProjectMultiSelect selectedIds={["proj-1"]} onChange={onChange} />,
        { wrapper: makeWrapper() }
      );

      fireEvent.click(screen.getByTestId("project-option-proj-2"));

      expect(onChange).toHaveBeenCalledWith(["proj-1", "proj-2"]);
    });

    it("calls onChange with removed id when selected project clicked", () => {
      const onChange = vi.fn();
      render(
        <ProjectMultiSelect selectedIds={["proj-1", "proj-2"]} onChange={onChange} />,
        { wrapper: makeWrapper() }
      );

      fireEvent.click(screen.getByTestId("project-option-proj-1"));

      expect(onChange).toHaveBeenCalledWith(["proj-2"]);
    });

    it("does not call onChange when disabled and project clicked", () => {
      const onChange = vi.fn();
      render(
        <ProjectMultiSelect selectedIds={[]} onChange={onChange} disabled />,
        { wrapper: makeWrapper() }
      );

      fireEvent.click(screen.getByTestId("project-option-proj-1"));

      expect(onChange).not.toHaveBeenCalled();
    });
  });
});
