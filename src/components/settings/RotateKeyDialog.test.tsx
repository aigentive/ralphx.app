/**
 * Tests for RotateKeyDialog component
 *
 * Covers: confirm step rendering, cancel, rotation execution, reveal step,
 * copy button, done button gating, error handling.
 */

import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { RotateKeyDialog } from "./RotateKeyDialog";

// Mock useRotateApiKey hook
vi.mock("@/hooks/useApiKeys", () => ({
  useRotateApiKey: vi.fn(),
  useApiKeys: vi.fn(),
  useApiKeyAuditLog: vi.fn(),
  useCreateApiKey: vi.fn(),
  useRevokeApiKey: vi.fn(),
  useUpdateKeyProjects: vi.fn(),
  useUpdateKeyPermissions: vi.fn(),
}));

import { useRotateApiKey } from "@/hooks/useApiKeys";

const defaultProps = {
  open: true,
  keyId: "key-001",
  keyName: "CI Key",
  onClose: vi.fn(),
  onRotated: vi.fn(),
};

function makeWrapper() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={qc}>{children}</QueryClientProvider>
  );
}

describe("RotateKeyDialog", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.stubGlobal("navigator", {
      clipboard: {
        writeText: vi.fn().mockResolvedValue(undefined),
      },
    });
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  describe("confirm step", () => {
    beforeEach(() => {
      vi.mocked(useRotateApiKey).mockReturnValue({
        mutateAsync: vi.fn(),
        isPending: false,
      } as unknown as ReturnType<typeof useRotateApiKey>);
    });

    it("renders rotate-key-dialog", () => {
      render(<RotateKeyDialog {...defaultProps} />, { wrapper: makeWrapper() });
      expect(screen.getByTestId("rotate-key-dialog")).toBeInTheDocument();
    });

    it("shows Rotate API Key title", () => {
      render(<RotateKeyDialog {...defaultProps} />, { wrapper: makeWrapper() });
      expect(screen.getByText("Rotate API Key")).toBeInTheDocument();
    });

    it("shows key name in description", () => {
      render(<RotateKeyDialog {...defaultProps} />, { wrapper: makeWrapper() });
      expect(screen.getByText(/CI Key/)).toBeInTheDocument();
    });

    it("shows 60 second grace period warning", () => {
      render(<RotateKeyDialog {...defaultProps} />, { wrapper: makeWrapper() });
      expect(screen.getByText(/60 seconds/)).toBeInTheDocument();
    });

    it("renders cancel and confirm-rotate buttons", () => {
      render(<RotateKeyDialog {...defaultProps} />, { wrapper: makeWrapper() });
      expect(screen.getByTestId("cancel-rotate-button")).toBeInTheDocument();
      expect(screen.getByTestId("confirm-rotate-button")).toBeInTheDocument();
    });

    it("calls onClose when Cancel clicked", () => {
      const onClose = vi.fn();
      render(<RotateKeyDialog {...defaultProps} onClose={onClose} />, {
        wrapper: makeWrapper(),
      });

      fireEvent.click(screen.getByTestId("cancel-rotate-button"));

      expect(onClose).toHaveBeenCalled();
    });
  });

  describe("rotation execution", () => {
    it("calls mutateAsync with keyId when Rotate Key confirmed", async () => {
      const mutateAsync = vi
        .fn()
        .mockResolvedValue({ raw_key: "rxk_live_newkey123", key: {} });

      vi.mocked(useRotateApiKey).mockReturnValue({
        mutateAsync,
        isPending: false,
      } as unknown as ReturnType<typeof useRotateApiKey>);

      render(<RotateKeyDialog {...defaultProps} />, { wrapper: makeWrapper() });

      fireEvent.click(screen.getByTestId("confirm-rotate-button"));

      await waitFor(() => {
        expect(mutateAsync).toHaveBeenCalledWith("key-001");
      });
    });

    it("shows reveal step with new key after successful rotation", async () => {
      vi.mocked(useRotateApiKey).mockReturnValue({
        mutateAsync: vi
          .fn()
          .mockResolvedValue({ raw_key: "rxk_live_newkey123", key: {} }),
        isPending: false,
      } as unknown as ReturnType<typeof useRotateApiKey>);

      render(<RotateKeyDialog {...defaultProps} />, { wrapper: makeWrapper() });

      fireEvent.click(screen.getByTestId("confirm-rotate-button"));

      await waitFor(() => {
        expect(screen.getByText("Key Rotated")).toBeInTheDocument();
      });

      expect(screen.getByText("rxk_live_newkey123")).toBeInTheDocument();
    });

    it("shows error and stays on confirm step when rotation fails", async () => {
      vi.mocked(useRotateApiKey).mockReturnValue({
        mutateAsync: vi
          .fn()
          .mockRejectedValue(new Error("Rotation failed")),
        isPending: false,
      } as unknown as ReturnType<typeof useRotateApiKey>);

      render(<RotateKeyDialog {...defaultProps} />, { wrapper: makeWrapper() });

      fireEvent.click(screen.getByTestId("confirm-rotate-button"));

      await waitFor(() => {
        expect(screen.getByText("Rotation failed")).toBeInTheDocument();
      });

      // Still on confirm step
      expect(screen.getByText("Rotate API Key")).toBeInTheDocument();
    });
  });

  describe("reveal step", () => {
    async function openRevealStep() {
      vi.mocked(useRotateApiKey).mockReturnValue({
        mutateAsync: vi
          .fn()
          .mockResolvedValue({ raw_key: "rxk_live_abc123", key: {} }),
        isPending: false,
      } as unknown as ReturnType<typeof useRotateApiKey>);

      render(<RotateKeyDialog {...defaultProps} />, { wrapper: makeWrapper() });

      fireEvent.click(screen.getByTestId("confirm-rotate-button"));

      await waitFor(() => {
        expect(screen.getByText("Key Rotated")).toBeInTheDocument();
      });
    }

    it("shows copy button in reveal step", async () => {
      await openRevealStep();
      expect(screen.getByTestId("copy-rotated-key-button")).toBeInTheDocument();
    });

    it("Done button is disabled before copying", async () => {
      await openRevealStep();
      expect(screen.getByTestId("done-rotate-button")).toBeDisabled();
    });

    it("copy button triggers clipboard write", async () => {
      await openRevealStep();

      fireEvent.click(screen.getByTestId("copy-rotated-key-button"));

      await waitFor(() => {
        expect(navigator.clipboard.writeText).toHaveBeenCalledWith("rxk_live_abc123");
      });
    });

    it("Done button enabled after copying", async () => {
      await openRevealStep();

      fireEvent.click(screen.getByTestId("copy-rotated-key-button"));

      await waitFor(() => {
        expect(screen.getByTestId("done-rotate-button")).not.toBeDisabled();
      });
    });

    it("Done button calls onRotated and onClose", async () => {
      const onRotated = vi.fn();
      const onClose = vi.fn();

      vi.mocked(useRotateApiKey).mockReturnValue({
        mutateAsync: vi
          .fn()
          .mockResolvedValue({ raw_key: "rxk_live_abc123", key: {} }),
        isPending: false,
      } as unknown as ReturnType<typeof useRotateApiKey>);

      render(
        <RotateKeyDialog
          open={true}
          keyId="key-001"
          keyName="CI Key"
          onClose={onClose}
          onRotated={onRotated}
        />,
        { wrapper: makeWrapper() }
      );

      fireEvent.click(screen.getByTestId("confirm-rotate-button"));

      await waitFor(() =>
        expect(screen.getByTestId("copy-rotated-key-button")).toBeInTheDocument()
      );

      fireEvent.click(screen.getByTestId("copy-rotated-key-button"));

      await waitFor(() =>
        expect(screen.getByTestId("done-rotate-button")).not.toBeDisabled()
      );

      fireEvent.click(screen.getByTestId("done-rotate-button"));

      expect(onRotated).toHaveBeenCalled();
      expect(onClose).toHaveBeenCalled();
    });
  });

  describe("closed dialog", () => {
    it("does not render dialog content when open=false", () => {
      vi.mocked(useRotateApiKey).mockReturnValue({
        mutateAsync: vi.fn(),
        isPending: false,
      } as unknown as ReturnType<typeof useRotateApiKey>);

      render(
        <RotateKeyDialog {...defaultProps} open={false} />,
        { wrapper: makeWrapper() }
      );

      expect(screen.queryByTestId("rotate-key-dialog")).not.toBeInTheDocument();
    });
  });
});
