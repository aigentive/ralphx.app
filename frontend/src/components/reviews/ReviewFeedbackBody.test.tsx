import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ReviewFeedbackBody } from "./ReviewFeedbackBody";

describe("ReviewFeedbackBody", () => {
  it("renders medium feedback inline and sanitizes ANSI", () => {
    render(
      <ReviewFeedbackBody
        summary="Repository commit hooks rejected the merge commit."
        notes={[
          "Repository commit hooks rejected the merge commit.",
          "",
          "Full hook output:",
          "```text",
          "[31m[pre-commit][0m design-token guards failed",
          "TS2307 Cannot find module 'zod'",
          "```",
        ].join("\n")}
      />
    );

    expect(
      screen.getByText("Repository commit hooks rejected the merge commit.")
    ).toBeInTheDocument();
    expect(screen.getByText(/design-token guards failed/)).toBeInTheDocument();
    expect(
      screen.queryByText((content) => content.includes("[31m"))
    ).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "View full details" })).not.toBeInTheDocument();
  });

  it("expands medium-long feedback inline when toggled", async () => {
    const user = userEvent.setup();
    const longNotes = [
      "Repository commit hooks rejected the merge commit.",
      "",
      "Full hook output:",
      "```text",
      "[31m[pre-commit][0m design-token guards failed",
      ...Array.from({ length: 70 }, (_, index) => `TS2307 module failure ${index}`),
      "```",
    ].join("\n");

    render(
      <ReviewFeedbackBody
        summary="Repository commit hooks rejected the merge commit."
        notes={longNotes}
      />
    );

    expect(
      screen.getByText("Repository commit hooks rejected the merge commit.")
    ).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "View full details" }));

    // Inline-expand path: no dialog, but the trailing lines must now be visible.
    expect(screen.queryByText("Full feedback")).not.toBeInTheDocument();
    expect(screen.getByText(/design-token guards failed/)).toBeInTheDocument();
    expect(screen.getByText(/TS2307 module failure 69/)).toBeInTheDocument();
    expect(
      screen.queryByText((content) => content.includes("[31m"))
    ).not.toBeInTheDocument();

    // Toggle collapses back to the preview.
    await user.click(screen.getByRole("button", { name: "Show less" }));
    expect(screen.queryByText(/TS2307 module failure 69/)).not.toBeInTheDocument();
  });

  it("opens a dialog for very large feedback bodies", async () => {
    const user = userEvent.setup();
    const veryLongNotes = [
      "Repository commit hooks rejected the merge commit.",
      "",
      "Full hook output:",
      "```text",
      "[31m[pre-commit][0m design-token guards failed",
      ...Array.from(
        { length: 240 },
        (_, index) =>
          `TS2307 module failure ${index} — stack frame details follow here.`
      ),
      "```",
    ].join("\n");

    render(
      <ReviewFeedbackBody
        summary="Repository commit hooks rejected the merge commit."
        notes={veryLongNotes}
      />
    );

    await user.click(screen.getByRole("button", { name: "View full details" }));

    expect(screen.getByText("Full feedback")).toBeInTheDocument();
    expect(screen.getByText(/TS2307 module failure 239/)).toBeInTheDocument();
    expect(
      screen.queryByText((content) => content.includes("[31m"))
    ).not.toBeInTheDocument();
  });
});
