import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { PaneInput } from "./PaneInput";
import type { TeammateStatus } from "@/stores/teamStore";

describe("PaneInput", () => {
  it("renders placeholder with teammate name", () => {
    render(<PaneInput teammateName="worker-1" status="running" onSend={vi.fn()} />);
    expect(screen.getByPlaceholderText("Message worker-1...")).toBeInTheDocument();
  });

  it("calls onSend with trimmed value on send button click", async () => {
    const user = userEvent.setup();
    const onSend = vi.fn();
    render(<PaneInput teammateName="worker-1" status="running" onSend={onSend} />);

    const input = screen.getByPlaceholderText("Message worker-1...");
    await user.type(input, "  Hello  ");
    fireEvent.click(screen.getByLabelText("Send message to worker-1"));

    expect(onSend).toHaveBeenCalledWith("Hello");
  });

  it("clears input after sending", async () => {
    const user = userEvent.setup();
    render(<PaneInput teammateName="worker-1" status="running" onSend={vi.fn()} />);

    const input = screen.getByPlaceholderText("Message worker-1...") as HTMLInputElement;
    await user.type(input, "test");
    await user.keyboard("{Enter}");

    expect(input.value).toBe("");
  });

  it("sends on Enter key press", async () => {
    const user = userEvent.setup();
    const onSend = vi.fn();
    render(<PaneInput teammateName="worker-1" status="running" onSend={onSend} />);

    const input = screen.getByPlaceholderText("Message worker-1...");
    await user.type(input, "test{Enter}");

    expect(onSend).toHaveBeenCalledWith("test");
  });

  it("disables input when status is shutdown", () => {
    render(<PaneInput teammateName="worker-1" status="shutdown" onSend={vi.fn()} />);
    expect(screen.getByPlaceholderText("Message worker-1...")).toBeDisabled();
  });

  it("disables input when status is completed", () => {
    render(<PaneInput teammateName="worker-1" status="completed" onSend={vi.fn()} />);
    expect(screen.getByPlaceholderText("Message worker-1...")).toBeDisabled();
  });

  it("does not disable input for running/idle/spawning statuses", () => {
    for (const status of ["running", "idle", "spawning"] as TeammateStatus[]) {
      const { unmount } = render(
        <PaneInput teammateName="worker-1" status={status} onSend={vi.fn()} />,
      );
      expect(screen.getByPlaceholderText("Message worker-1...")).not.toBeDisabled();
      unmount();
    }
  });
});
