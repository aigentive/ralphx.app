import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";

import { useThemeStore } from "@/stores/themeStore";
import { ThemeSelector } from "./ThemeSelector";

describe("ThemeSelector", () => {
  beforeEach(() => {
    localStorage.clear();
    useThemeStore.getState().setTheme("dark");
  });

  it("renders the active theme in the trigger", () => {
    render(<ThemeSelector />);

    expect(screen.getByTestId("theme-selector-trigger")).toHaveTextContent("Dark");
  });

  it("updates the theme store and DOM attribute when selecting a new theme", async () => {
    const user = userEvent.setup();

    render(<ThemeSelector />);

    await user.click(screen.getByTestId("theme-selector-trigger"));
    await user.click(screen.getByTestId("theme-option-light"));

    expect(useThemeStore.getState().theme).toBe("light");
    expect(document.documentElement).toHaveAttribute("data-theme", "light");
    expect(screen.getByTestId("theme-selector-trigger")).toHaveTextContent("Light");
  });
});
