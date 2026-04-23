import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { TooltipProvider } from "@/components/ui/tooltip";
import { createTestQueryClient } from "@/test/store-utils";
import { AgentsArtifactPane } from "./AgentsArtifactPane";

function renderPane() {
  const queryClient = createTestQueryClient();

  return render(
    <QueryClientProvider client={queryClient}>
      <TooltipProvider>
        <div className="h-[480px]">
          <AgentsArtifactPane
            conversation={null}
            activeTab="tasks"
            taskMode="graph"
            onTabChange={() => {}}
            onTaskModeChange={() => {}}
            onClose={() => {}}
          />
        </div>
      </TooltipProvider>
    </QueryClientProvider>
  );
}

describe("AgentsArtifactPane", () => {
  it("anchors the active tab border to the bottom edge of the tab bar", () => {
    renderPane();

    const tabRow = screen.getByTestId("agents-artifact-tab-row");
    const activeTab = screen.getByTestId("agents-artifact-tab-tasks");
    const inactiveTab = screen.getByTestId("agents-artifact-tab-plan");

    expect(tabRow.getAttribute("style")).toContain(
      "border-color: var(--border-subtle);"
    );
    expect(activeTab.parentElement?.className).toContain("self-stretch");
    expect(activeTab.className).toContain("self-stretch");
    expect(activeTab.getAttribute("data-theme-button-skip")).toBe("true");
    expect(inactiveTab.getAttribute("data-theme-button-skip")).toBe("true");
    expect(activeTab.className).not.toContain("border-b-2");
    expect(activeTab.querySelector("span[style='background: var(--accent-primary);']")).not.toBeNull();
    expect(inactiveTab.querySelector("span[style='background: var(--accent-primary);']")).toBeNull();
  });
});
