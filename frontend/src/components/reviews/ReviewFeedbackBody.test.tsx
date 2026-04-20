import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ReviewFeedbackBody } from "./ReviewFeedbackBody";

describe("ReviewFeedbackBody", () => {
  it("uses summary for preview and sanitizes ANSI in the full dialog", async () => {
    const user = userEvent.setup();

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

    await user.click(screen.getByRole("button", { name: "View full details" }));

    expect(screen.getByText("Full feedback")).toBeInTheDocument();
    expect(screen.getByText(/design-token guards failed/)).toBeInTheDocument();
    expect(
      screen.queryByText((content) => content.includes("\u001b[31m"))
    ).not.toBeInTheDocument();
  });
});
