import { describe, it, expect, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { PrefixKeyOverlay } from "./PrefixKeyOverlay";
import { useSplitPaneStore } from "@/stores/splitPaneStore";

describe("PrefixKeyOverlay", () => {
  beforeEach(() => {
    useSplitPaneStore.setState({ isPrefixKeyActive: false });
  });

  it("renders nothing when prefix key is not active", () => {
    const { container } = render(<PrefixKeyOverlay />);
    expect(container.firstChild).toBeNull();
  });

  it("renders overlay with keyboard shortcut when prefix key is active", () => {
    useSplitPaneStore.setState({ isPrefixKeyActive: true });
    render(<PrefixKeyOverlay />);
    expect(screen.getByText("Ctrl+B")).toBeInTheDocument();
    expect(screen.getByText("active — press arrow or 1-5")).toBeInTheDocument();
  });
});
