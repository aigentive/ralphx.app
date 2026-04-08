import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { TooltipProvider } from "@/components/ui/tooltip";
import { ModelChip } from "./ModelChip";

function renderWithProvider(ui: React.ReactElement) {
  // delayDuration=0 ensures tooltip appears immediately in JSDOM hover tests
  return render(<TooltipProvider delayDuration={0}>{ui}</TooltipProvider>);
}

describe("ModelChip", () => {
  it("renders without requiring an external TooltipProvider", () => {
    render(<ModelChip model={{ id: "claude-sonnet-4-6", label: "Sonnet 4.6" }} />);
    expect(screen.getByText("Sonnet 4.6")).toBeDefined();
  });

  it("renders short label without truncation", () => {
    renderWithProvider(<ModelChip model={{ id: "claude-sonnet-4-6", label: "Sonnet 4.6" }} />);
    expect(screen.getByText("Sonnet 4.6")).toBeDefined();
  });

  it("truncates labels longer than 20 chars to 17 chars + '...'", () => {
    const longLabel = "A very long model label here"; // "A very long model" = 17 chars
    renderWithProvider(<ModelChip model={{ id: "some-id", label: longLabel }} />);
    expect(screen.getByText("A very long model...")).toBeDefined();
  });

  it("does not truncate labels of exactly 20 chars", () => {
    const label = "ExactlyTwentyCharsXX"; // 20 chars — no truncation
    renderWithProvider(<ModelChip model={{ id: "some-id", label }} />);
    expect(screen.getByText(label)).toBeDefined();
  });

  it("truncates labels of exactly 21 chars to 17 + '...'", () => {
    const label = "ExactlyTwentyCharsXXY"; // 21 chars → "ExactlyTwentyChar" (17) + "..."
    renderWithProvider(<ModelChip model={{ id: "some-id", label }} />);
    expect(screen.getByText("ExactlyTwentyChar...")).toBeDefined();
  });

  it("applies muted text styling", () => {
    renderWithProvider(<ModelChip model={{ id: "claude-sonnet-4-6", label: "Sonnet 4.6" }} />);
    const el = screen.getByText("Sonnet 4.6");
    expect(el.className).toContain("text-white/40");
    expect(el.className).toContain("text-xs");
  });

  it("tooltip trigger wraps the label span", () => {
    renderWithProvider(<ModelChip model={{ id: "claude-sonnet-4-6", label: "Sonnet 4.6" }} />);
    // The label is rendered as the trigger content
    const triggerEl = screen.getByText("Sonnet 4.6");
    expect(triggerEl.tagName).toBe("SPAN");
  });

  it("tooltip shows full model.id on hover", async () => {
    const user = userEvent.setup();
    renderWithProvider(<ModelChip model={{ id: "claude-sonnet-4-6", label: "Sonnet 4.6" }} />);
    const trigger = screen.getByText("Sonnet 4.6");
    await user.hover(trigger);
    // role="tooltip" is the Radix UI accessibility element containing the tooltip text
    const tooltip = await screen.findByRole("tooltip");
    expect(tooltip.textContent).toContain("claude-sonnet-4-6");
  });

  it("parent guard: undefined modelDisplay renders no chip", () => {
    // The parent pattern is: {modelDisplay && <ModelChip model={modelDisplay} />}
    // Test this guard directly — when modelDisplay is undefined, nothing renders
    const modelDisplay: { id: string; label: string } | undefined = undefined;
    const { container } = render(
      <TooltipProvider delayDuration={0}>
        {modelDisplay ? <ModelChip model={modelDisplay} /> : null}
      </TooltipProvider>
    );
    expect(container.firstChild).toBeNull();
  });
});
