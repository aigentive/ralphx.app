import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { AccessibilitySection } from "./AccessibilitySection";
import { useThemeStore } from "@/stores/themeStore";

if (!HTMLElement.prototype.hasPointerCapture) {
  Object.defineProperty(HTMLElement.prototype, "hasPointerCapture", {
    value: () => false,
    writable: true,
  });
}

if (!HTMLElement.prototype.setPointerCapture) {
  Object.defineProperty(HTMLElement.prototype, "setPointerCapture", {
    value: vi.fn(),
    writable: true,
  });
}

if (!HTMLElement.prototype.releasePointerCapture) {
  Object.defineProperty(HTMLElement.prototype, "releasePointerCapture", {
    value: vi.fn(),
    writable: true,
  });
}

if (!HTMLElement.prototype.scrollIntoView) {
  Object.defineProperty(HTMLElement.prototype, "scrollIntoView", {
    value: vi.fn(),
    writable: true,
  });
}

function openSelect(testId: string) {
  const trigger = screen.getByTestId(testId);
  fireEvent.keyDown(trigger, { key: "ArrowDown", code: "ArrowDown" });
}

describe("AccessibilitySection", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.removeAttribute("data-theme");
    document.documentElement.removeAttribute("data-motion");
    document.documentElement.removeAttribute("data-font-scale");
    document.documentElement.classList.remove("dark");
    useThemeStore.setState({
      theme: "dark",
      motion: "system",
      fontScale: "default",
    });
  });

  it("switches from stored high contrast to dark via the theme selector only", async () => {
    useThemeStore.getState().setTheme("high-contrast");
    render(<AccessibilitySection />);

    expect(screen.queryByTestId("theme-high-contrast")).not.toBeInTheDocument();

    openSelect("theme-selector");
    fireEvent.click(screen.getByRole("option", { name: /Dark \(default\)/ }));

    await waitFor(() => {
      expect(useThemeStore.getState().theme).toBe("dark");
    });

    expect(document.documentElement).toHaveAttribute("data-theme", "dark");
    expect(localStorage.getItem("ralphx-theme")).toBe("dark");
  });

  it("supports dark to high-contrast to dark roundtrip through the selector", async () => {
    render(<AccessibilitySection />);

    expect(screen.queryByTestId("theme-high-contrast")).not.toBeInTheDocument();

    openSelect("theme-selector");
    fireEvent.click(screen.getByRole("option", { name: /High contrast/ }));

    await waitFor(() => {
      expect(useThemeStore.getState().theme).toBe("high-contrast");
    });

    openSelect("theme-selector");
    fireEvent.click(screen.getByRole("option", { name: /Dark \(default\)/ }));

    await waitFor(() => {
      expect(useThemeStore.getState().theme).toBe("dark");
    });

    expect(document.documentElement).toHaveAttribute("data-theme", "dark");
    expect(localStorage.getItem("ralphx-theme")).toBe("dark");
  });

  it("selecting Large font scale updates store, localStorage, and root attribute", async () => {
    render(<AccessibilitySection />);

    openSelect("font-scale");
    fireEvent.click(screen.getByRole("option", { name: /Large/ }));

    await waitFor(() => {
      expect(useThemeStore.getState().fontScale).toBe("lg");
    });

    expect(document.documentElement).toHaveAttribute("data-font-scale", "lg");
    expect(localStorage.getItem("ralphx-font-scale")).toBe("lg");
  });

  it("selecting Extra large font scale updates store, localStorage, and root attribute", async () => {
    render(<AccessibilitySection />);

    openSelect("font-scale");
    fireEvent.click(screen.getByRole("option", { name: /Extra large/ }));

    await waitFor(() => {
      expect(useThemeStore.getState().fontScale).toBe("xl");
    });

    expect(document.documentElement).toHaveAttribute("data-font-scale", "xl");
    expect(localStorage.getItem("ralphx-font-scale")).toBe("xl");
  });

  it("resetting font scale to Default removes attribute and clears localStorage", async () => {
    useThemeStore.getState().setFontScale("lg");
    render(<AccessibilitySection />);

    openSelect("font-scale");
    fireEvent.click(screen.getByRole("option", { name: /Default/ }));

    await waitFor(() => {
      expect(useThemeStore.getState().fontScale).toBe("default");
    });

    expect(document.documentElement).not.toHaveAttribute("data-font-scale");
    expect(localStorage.getItem("ralphx-font-scale")).toBeNull();
  });
});
