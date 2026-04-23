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
          "\u001b[31m[pre-commit]\u001b[0m design-token guards failed",
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
      screen.queryByText((content) => content.includes("\u001b[31m"))
    ).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "View full details" })).not.toBeInTheDocument();
  });

  it("uses summary preview and full dialog for long feedback", async () => {
    const user = userEvent.setup();
    const longNotes = [
      "Repository commit hooks rejected the merge commit.",
      "",
      "Full hook output:",
      "```text",
      "\u001b[31m[pre-commit]\u001b[0m design-token guards failed",
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

    expect(screen.getByText("Full feedback")).toBeInTheDocument();
    expect(screen.getByText(/design-token guards failed/)).toBeInTheDocument();
    expect(
      screen.queryByText((content) => content.includes("\u001b[31m"))
    ).not.toBeInTheDocument();
  });
});
