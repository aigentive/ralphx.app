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

  it("renders a direct theme switcher with the active option selected", () => {
    render(<ThemeSelector />);

    expect(screen.getByTestId("theme-selector")).toBeInTheDocument();
    expect(screen.getByTestId("theme-option-dark")).toHaveAttribute("aria-checked", "true");
    expect(screen.getByTestId("theme-option-light")).toHaveAttribute("aria-checked", "false");
  });

  it("updates the theme store and DOM attribute when clicking a theme option", async () => {
    const user = userEvent.setup();

    render(<ThemeSelector />);

    await user.click(screen.getByTestId("theme-option-light"));

    expect(useThemeStore.getState().theme).toBe("light");
    expect(document.documentElement).toHaveAttribute("data-theme", "light");
    expect(screen.getByTestId("theme-option-light")).toHaveAttribute("aria-checked", "true");
    expect(screen.getByTestId("theme-option-dark")).toHaveAttribute("aria-checked", "false");
  });
});
