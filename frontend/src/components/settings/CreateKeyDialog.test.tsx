/**
 * Tests for CreateKeyDialog component
 *
 * Covers: input step rendering, name validation, reveal step with warning,
 * copy button, done button gating on copy.
 */

import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { CreateKeyDialog } from "./CreateKeyDialog";
import { useCreateApiKey } from "@/hooks/useApiKeys";
import type { ApiKey } from "@/types/api-key";

vi.mock("@/hooks/useApiKeys", () => ({
  useCreateApiKey: vi.fn(),
}));

const defaultProps = {
  open: true,
  onClose: vi.fn(),
  onCreated: vi.fn(),
};

const mockKey: ApiKey = {
  id: "key-001",
  name: "Test",
  keyPrefix: "rxk_live_a3f",
  permissions: 3,
  createdAt: "2024-01-01T00:00:00Z",
  revokedAt: null,
  lastUsedAt: null,
  projectIds: [],
};

function makeWrapper() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={qc}>{children}</QueryClientProvider>
  );
}

function makeSuccessMutation(rawKey = "rxk_live_supersecretkey") {
  return {
    mutateAsync: vi.fn().mockResolvedValue({ id: mockKey.id, name: mockKey.name, rawKey, keyPrefix: mockKey.keyPrefix, permissions: mockKey.permissions }),
    isPending: false,
    reset: vi.fn(),
  };
}

describe("CreateKeyDialog", () => {
  beforeEach(() => {
    // Mock clipboard API
    vi.stubGlobal("navigator", {
      clipboard: {
        writeText: vi.fn().mockResolvedValue(undefined),
      },
    });
    vi.mocked(useCreateApiKey).mockReturnValue(
      makeSuccessMutation() as unknown as ReturnType<typeof useCreateApiKey>
    );
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.clearAllMocks();
  });

  describe("input step", () => {
    it("renders Create API Key title and input", () => {
      render(<CreateKeyDialog {...defaultProps} />, { wrapper: makeWrapper() });

      expect(screen.getByText("Create API Key")).toBeInTheDocument();
      expect(screen.getByTestId("key-name-input")).toBeInTheDocument();
    });

    it("shows Cancel and Create Key buttons", () => {
      render(<CreateKeyDialog {...defaultProps} />, { wrapper: makeWrapper() });

      expect(screen.getByTestId("cancel-button")).toBeInTheDocument();
      expect(screen.getByTestId("create-button")).toBeInTheDocument();
    });

    it("Create Key button is disabled when name is empty", () => {
      render(<CreateKeyDialog {...defaultProps} />, { wrapper: makeWrapper() });

      const createBtn = screen.getByTestId("create-button");
      expect(createBtn).toBeDisabled();
    });

    it("Create Key button enabled after entering name", () => {
      render(<CreateKeyDialog {...defaultProps} />, { wrapper: makeWrapper() });

      const input = screen.getByTestId("key-name-input");
      fireEvent.change(input, { target: { value: "My CI Key" } });

      expect(screen.getByTestId("create-button")).not.toBeDisabled();
    });

    it("calls onClose when Cancel clicked", () => {
      const onClose = vi.fn();
      render(<CreateKeyDialog {...defaultProps} onClose={onClose} />, {
        wrapper: makeWrapper(),
      });

      fireEvent.click(screen.getByTestId("cancel-button"));

      expect(onClose).toHaveBeenCalled();
    });
  });

  describe("reveal step", () => {
    async function openRevealStep(wrapper = makeWrapper()) {
      render(<CreateKeyDialog {...defaultProps} />, { wrapper });

      const input = screen.getByTestId("key-name-input");
      fireEvent.change(input, { target: { value: "My Key" } });
      fireEvent.click(screen.getByTestId("create-button"));

      await waitFor(() => {
        expect(screen.getByText("Key Created")).toBeInTheDocument();
      });
    }

    it("shows Key Created title in reveal step", async () => {
      await openRevealStep();
      expect(screen.getByText("Key Created")).toBeInTheDocument();
    });

    it("shows warning message about one-time display", async () => {
      await openRevealStep();

      expect(
        screen.getByText(/This key will only be shown once/)
      ).toBeInTheDocument();
    });

    it("displays the raw key value", async () => {
      await openRevealStep();

      expect(screen.getByText("rxk_live_supersecretkey")).toBeInTheDocument();
    });

    it("shows Done button (initially disabled before copy)", async () => {
      await openRevealStep();

      const doneBtn = screen.getByTestId("done-button");
      expect(doneBtn).toBeInTheDocument();
      expect(doneBtn).toBeDisabled();
    });

    it("Copy button triggers clipboard write", async () => {
      await openRevealStep();

      fireEvent.click(screen.getByTestId("copy-key-button"));

      await waitFor(() => {
        expect(navigator.clipboard.writeText).toHaveBeenCalledWith(
          "rxk_live_supersecretkey"
        );
      });
    });

    it("Done button becomes enabled after copying", async () => {
      await openRevealStep();

      fireEvent.click(screen.getByTestId("copy-key-button"));

      await waitFor(() => {
        expect(screen.getByTestId("done-button")).not.toBeDisabled();
      });
    });

    it("clicking Done calls onCreated and onClose", async () => {
      const onCreated = vi.fn();
      const onClose = vi.fn();

      vi.mocked(useCreateApiKey).mockReturnValue(
        makeSuccessMutation("rxk_live_abc") as unknown as ReturnType<
          typeof useCreateApiKey
        >
      );

      render(
        <CreateKeyDialog open={true} onClose={onClose} onCreated={onCreated} />,
        { wrapper: makeWrapper() }
      );

      fireEvent.change(screen.getByTestId("key-name-input"), {
        target: { value: "test key" },
      });
      fireEvent.click(screen.getByTestId("create-button"));

      await waitFor(() =>
        expect(screen.getByTestId("done-button")).toBeInTheDocument()
      );

      // Copy first to enable Done
      fireEvent.click(screen.getByTestId("copy-key-button"));

      await waitFor(() =>
        expect(screen.getByTestId("done-button")).not.toBeDisabled()
      );

      fireEvent.click(screen.getByTestId("done-button"));

      expect(onCreated).toHaveBeenCalled();
      expect(onClose).toHaveBeenCalled();
    });
  });

  describe("error handling", () => {
    it("shows error when create request fails", async () => {
      vi.mocked(useCreateApiKey).mockReturnValue({
        mutateAsync: vi.fn().mockRejectedValue(new Error("Server error")),
        isPending: false,
        reset: vi.fn(),
      } as unknown as ReturnType<typeof useCreateApiKey>);

      render(<CreateKeyDialog {...defaultProps} />, { wrapper: makeWrapper() });

      const input = screen.getByTestId("key-name-input");
      fireEvent.change(input, { target: { value: "My Key" } });
      fireEvent.click(screen.getByTestId("create-button"));

      await waitFor(() => {
        expect(screen.getByText("Server error")).toBeInTheDocument();
      });

      // Should remain on input step
      expect(screen.getByText("Create API Key")).toBeInTheDocument();
    });
  });

  describe("dialog closed when not open", () => {
    it("does not render content when open=false", () => {
      render(<CreateKeyDialog open={false} onClose={vi.fn()} onCreated={vi.fn()} />, {
        wrapper: makeWrapper(),
      });

      expect(screen.queryByTestId("create-key-dialog")).not.toBeInTheDocument();
    });
  });
});
