/**
 * Integration test: Effective Model — event→store→render pipeline
 *
 * Tests the full pipeline:
 * 1. Store receives effectiveModel (as would happen after agent:run_started event)
 * 2. StatusActivityBadge renders ModelChip with correct label
 * 3. Error-path: no effectiveModelId → no chip, no console.warn
 * 4. Truncation: 21-char label → rendered as 17 chars + '...'
 */

import { describe, it, expect, beforeEach, vi, afterEach } from "vitest";
import { render, screen, act } from "@testing-library/react";
import { TooltipProvider } from "@/components/ui/tooltip";
import { useChatStore } from "@/stores/chatStore";
import { StatusActivityBadge } from "./StatusActivityBadge";

// ============================================================================
// Store mocks for StatusActivityBadge dependencies
// ============================================================================

// We use the real chatStore for effectiveModel to test the pipeline end-to-end.
// Other store slices that StatusActivityBadge reads are mocked to isolate the test.
vi.mock("@/stores/uiStore", () => ({
  useUiStore: vi.fn((selector: (s: { setActivityFilter: () => void; setCurrentView: () => void }) => unknown) =>
    selector({ setActivityFilter: vi.fn(), setCurrentView: vi.fn() })
  ),
}));

vi.mock("@/stores/ideationStore", () => ({
  useIdeationStore: vi.fn(
    (selector: (s: { activeVerificationChildId: Record<string, string | null> }) => unknown) =>
      selector({ activeVerificationChildId: {} })
  ),
}));

// ============================================================================
// Helpers
// ============================================================================

const STORE_KEY = "task_execution:task-abc";

/** Renders StatusActivityBadge in a generating state with the given modelDisplay. */
function renderBadge(modelDisplay?: { id: string; label: string }) {
  return render(
    <TooltipProvider delayDuration={0}>
      <StatusActivityBadge
        isAgentActive={true}
        agentType="worker"
        contextType="task_execution"
        contextId="task-abc"
        agentStatus="generating"
        storeKey={STORE_KEY}
        modelDisplay={modelDisplay}
      />
    </TooltipProvider>
  );
}

// ============================================================================
// Tests
// ============================================================================

describe("Effective Model pipeline (event→store→render)", () => {
  beforeEach(() => {
    // Reset real chatStore between tests using the store's actual state reset
    useChatStore.setState({ effectiveModel: {} });
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  // --------------------------------------------------------------------------
  // Store population
  // --------------------------------------------------------------------------

  it("setEffectiveModel populates the store at the correct key", () => {
    const model = { id: "claude-sonnet-4-6", label: "Sonnet 4.6" };

    act(() => {
      useChatStore.getState().setEffectiveModel(STORE_KEY, model);
    });

    const stored = useChatStore.getState().effectiveModel[STORE_KEY];
    expect(stored).toEqual(model);
  });

  it("setEffectiveModel does not affect other store keys", () => {
    const model = { id: "claude-sonnet-4-6", label: "Sonnet 4.6" };
    const otherKey = "task_execution:task-other";

    act(() => {
      useChatStore.getState().setEffectiveModel(STORE_KEY, model);
    });

    expect(useChatStore.getState().effectiveModel[otherKey]).toBeUndefined();
  });

  // --------------------------------------------------------------------------
  // Render pipeline: store value → chip visible
  // --------------------------------------------------------------------------

  it("StatusActivityBadge renders ModelChip when modelDisplay is provided", () => {
    const model = { id: "claude-sonnet-4-6", label: "Sonnet 4.6" };
    renderBadge(model);
    expect(screen.getByText("Sonnet 4.6")).toBeDefined();
  });

  it("chip reflects label from store after setEffectiveModel", () => {
    const model = { id: "claude-sonnet-4-6", label: "Sonnet 4.6" };

    act(() => {
      useChatStore.getState().setEffectiveModel(STORE_KEY, model);
    });

    const stored = useChatStore.getState().effectiveModel[STORE_KEY];
    renderBadge(stored);

    expect(screen.getByText("Sonnet 4.6")).toBeDefined();
  });

  // --------------------------------------------------------------------------
  // Error-path: undefined modelDisplay → no chip
  // --------------------------------------------------------------------------

  it("no chip rendered when modelDisplay is undefined", () => {
    renderBadge(undefined);
    // ModelChip only renders inside StatusActivityBadge when modelDisplay is truthy.
    // The agent badge text confirms it rendered, but no model chip should be present.
    expect(screen.queryByText(/claude-/)).toBeNull();
    // Badge text confirms component rendered
    expect(screen.getByText("Worker running...")).toBeDefined();
  });

  it("no console.warn when effectiveModelId is absent from store", () => {
    const warnSpy = vi.spyOn(console, "warn");
    // Store has no entry for this key — simulate no event received
    renderBadge(undefined);
    expect(warnSpy).not.toHaveBeenCalled();
  });

  // --------------------------------------------------------------------------
  // Truncation: 21-char label renders as 17 chars + '...'
  // --------------------------------------------------------------------------

  it("truncates 21-char label to 17 chars + '...' in the chip", () => {
    const label = "ExactlyTwentyCharsXXY"; // 21 chars
    const model = { id: "some-model-id", label };

    act(() => {
      useChatStore.getState().setEffectiveModel(STORE_KEY, model);
    });

    const stored = useChatStore.getState().effectiveModel[STORE_KEY];
    renderBadge(stored);

    // ModelChip.tsx: label.slice(0, 17) + "..."
    expect(screen.getByText("ExactlyTwentyChar...")).toBeDefined();
    // Original label must not appear
    expect(screen.queryByText(label)).toBeNull();
  });
});
