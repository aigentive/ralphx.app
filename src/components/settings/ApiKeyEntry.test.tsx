/**
 * Tests for ApiKeyEntry component
 *
 * Covers: collapsed rendering, expand/collapse toggle, revoke flow (in expanded
 * view), project/permission save actions, audit log display.
 */

import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { ApiKeyEntry } from "./ApiKeyEntry";
import type { ApiKey } from "@/types/api-key";

// Mock all hooks used by ApiKeyEntry and its children
vi.mock("@/hooks/useApiKeys", () => ({
  useApiKeys: vi.fn(),
  useApiKeyAuditLog: vi.fn(() => ({ data: [], isLoading: false, error: null })),
  useCreateApiKey: vi.fn(),
  useRevokeApiKey: vi.fn(),
  useRotateApiKey: vi.fn(() => ({ mutateAsync: vi.fn(), isPending: false })),
  useUpdateKeyProjects: vi.fn(),
  useUpdateKeyPermissions: vi.fn(),
}));

vi.mock("@/hooks/useProjects", () => ({
  useProjects: vi.fn(() => ({ data: [], isLoading: false, error: null })),
}));

import {
  useRevokeApiKey,
  useUpdateKeyProjects,
  useUpdateKeyPermissions,
} from "@/hooks/useApiKeys";

const mockApiKey: ApiKey = {
  id: "key-001",
  name: "CI Key",
  key_prefix: "rxk_live_a3f2",
  permissions: 3,
  created_at: "2024-01-15T10:00:00Z",
  revoked_at: null,
  last_used_at: "2024-03-01T08:30:00Z",
  project_ids: ["proj-1"],
};

const mockNeverUsedKey: ApiKey = {
  id: "key-002",
  name: "New Key",
  key_prefix: "rxk_live_b9e1",
  permissions: 1,
  created_at: "2024-06-01T00:00:00Z",
  revoked_at: null,
  last_used_at: null,
  project_ids: [],
};

function makeWrapper() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={qc}>{children}</QueryClientProvider>
  );
}

function setupMockMutations() {
  const revokeMutate = vi.fn();
  const updateProjectsMutate = vi.fn();
  const updatePermsMutate = vi.fn();

  vi.mocked(useRevokeApiKey).mockReturnValue({
    mutate: revokeMutate,
    isPending: false,
  } as unknown as ReturnType<typeof useRevokeApiKey>);

  vi.mocked(useUpdateKeyProjects).mockReturnValue({
    mutate: updateProjectsMutate,
    isPending: false,
  } as unknown as ReturnType<typeof useUpdateKeyProjects>);

  vi.mocked(useUpdateKeyPermissions).mockReturnValue({
    mutate: updatePermsMutate,
    isPending: false,
  } as unknown as ReturnType<typeof useUpdateKeyPermissions>);

  return { revokeMutate, updateProjectsMutate, updatePermsMutate };
}

describe("ApiKeyEntry", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    setupMockMutations();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe("collapsed rendering", () => {
    it("renders key name and prefix", () => {
      render(<ApiKeyEntry apiKey={mockApiKey} onKeyChanged={vi.fn()} />, {
        wrapper: makeWrapper(),
      });

      expect(screen.getByText("CI Key")).toBeInTheDocument();
      expect(screen.getByText("rxk_live_a3f2...")).toBeInTheDocument();
    });

    it("shows creation date", () => {
      render(<ApiKeyEntry apiKey={mockApiKey} onKeyChanged={vi.fn()} />, {
        wrapper: makeWrapper(),
      });

      expect(screen.getByText(/Jan 15, 2024/)).toBeInTheDocument();
    });

    it("shows last used date when present", () => {
      render(<ApiKeyEntry apiKey={mockApiKey} onKeyChanged={vi.fn()} />, {
        wrapper: makeWrapper(),
      });

      expect(screen.getByText(/Mar 1, 2024/)).toBeInTheDocument();
    });

    it("shows Never for last_used_at when null", () => {
      render(<ApiKeyEntry apiKey={mockNeverUsedKey} onKeyChanged={vi.fn()} />, {
        wrapper: makeWrapper(),
      });

      expect(screen.getByText(/Last used Never/)).toBeInTheDocument();
    });

    it("shows expand toggle button", () => {
      render(<ApiKeyEntry apiKey={mockApiKey} onKeyChanged={vi.fn()} />, {
        wrapper: makeWrapper(),
      });

      expect(screen.getByTestId("expand-key-key-001")).toBeInTheDocument();
    });

    it("revoke button is NOT visible in collapsed state", () => {
      render(<ApiKeyEntry apiKey={mockApiKey} onKeyChanged={vi.fn()} />, {
        wrapper: makeWrapper(),
      });

      expect(screen.queryByTestId("revoke-key-key-001")).not.toBeInTheDocument();
    });

    it("uses key id in entry testid", () => {
      render(<ApiKeyEntry apiKey={mockApiKey} onKeyChanged={vi.fn()} />, {
        wrapper: makeWrapper(),
      });

      expect(screen.getByTestId("api-key-entry-key-001")).toBeInTheDocument();
    });

    it("uses key id in expand button testid for second key", () => {
      render(<ApiKeyEntry apiKey={mockNeverUsedKey} onKeyChanged={vi.fn()} />, {
        wrapper: makeWrapper(),
      });

      expect(screen.getByTestId("expand-key-key-002")).toBeInTheDocument();
    });
  });

  describe("expand/collapse", () => {
    it("clicking expand shows expanded content", () => {
      render(<ApiKeyEntry apiKey={mockApiKey} onKeyChanged={vi.fn()} />, {
        wrapper: makeWrapper(),
      });

      fireEvent.click(screen.getByTestId("expand-key-key-001"));

      expect(screen.getByTestId("revoke-key-key-001")).toBeInTheDocument();
      expect(screen.getByTestId("rotate-key-key-001")).toBeInTheDocument();
      expect(screen.getByTestId("save-projects-key-001")).toBeInTheDocument();
      expect(screen.getByTestId("save-permissions-key-001")).toBeInTheDocument();
    });

    it("clicking expand again collapses", () => {
      render(<ApiKeyEntry apiKey={mockApiKey} onKeyChanged={vi.fn()} />, {
        wrapper: makeWrapper(),
      });

      fireEvent.click(screen.getByTestId("expand-key-key-001"));
      expect(screen.getByTestId("revoke-key-key-001")).toBeInTheDocument();

      fireEvent.click(screen.getByTestId("expand-key-key-001"));
      expect(screen.queryByTestId("revoke-key-key-001")).not.toBeInTheDocument();
    });
  });

  describe("two-click revoke flow", () => {
    function expandEntry() {
      fireEvent.click(screen.getByTestId("expand-key-key-001"));
    }

    it("shows Revoke button after expand", () => {
      render(<ApiKeyEntry apiKey={mockApiKey} onKeyChanged={vi.fn()} />, {
        wrapper: makeWrapper(),
      });
      expandEntry();

      expect(screen.getByTestId("revoke-key-key-001")).toBeInTheDocument();
      expect(screen.getByText("Revoke")).toBeInTheDocument();
    });

    it("first click shows Confirm? state", () => {
      render(<ApiKeyEntry apiKey={mockApiKey} onKeyChanged={vi.fn()} />, {
        wrapper: makeWrapper(),
      });
      expandEntry();

      fireEvent.click(screen.getByTestId("revoke-key-key-001"));

      expect(screen.getByText("Confirm?")).toBeInTheDocument();
      expect(screen.getByText("Cancel")).toBeInTheDocument();
    });

    it("Cancel button resets to initial Revoke state", () => {
      render(<ApiKeyEntry apiKey={mockApiKey} onKeyChanged={vi.fn()} />, {
        wrapper: makeWrapper(),
      });
      expandEntry();

      fireEvent.click(screen.getByTestId("revoke-key-key-001"));
      expect(screen.getByText("Confirm?")).toBeInTheDocument();

      fireEvent.click(screen.getByText("Cancel"));

      expect(screen.getByText("Revoke")).toBeInTheDocument();
      expect(screen.queryByText("Confirm?")).not.toBeInTheDocument();
      expect(screen.queryByText("Cancel")).not.toBeInTheDocument();
    });

    it("second click calls revoke mutation and triggers onRevoked on success", async () => {
      const onRevoked = vi.fn();
      const revokeMutate = vi.fn().mockImplementation(
        (_id: string, { onSuccess }: { onSuccess?: () => void }) => {
          onSuccess?.();
        }
      );

      vi.mocked(useRevokeApiKey).mockReturnValue({
        mutate: revokeMutate,
        isPending: false,
      } as unknown as ReturnType<typeof useRevokeApiKey>);

      render(<ApiKeyEntry apiKey={mockApiKey} onKeyChanged={onRevoked} />, {
        wrapper: makeWrapper(),
      });
      expandEntry();

      fireEvent.click(screen.getByTestId("revoke-key-key-001"));
      fireEvent.click(screen.getByTestId("revoke-key-key-001"));

      await waitFor(() => {
        expect(revokeMutate).toHaveBeenCalledWith(
          "key-001",
          expect.objectContaining({ onSuccess: expect.any(Function) })
        );
        expect(onRevoked).toHaveBeenCalledOnce();
      });
    });

    it("shows error message when revoke fails", async () => {
      const revokeMutate = vi
        .fn()
        .mockImplementation(
          (_id: string, { onError }: { onError?: (err: Error) => void }) => {
            onError?.(new Error("Forbidden"));
          }
        );

      vi.mocked(useRevokeApiKey).mockReturnValue({
        mutate: revokeMutate,
        isPending: false,
      } as unknown as ReturnType<typeof useRevokeApiKey>);

      render(<ApiKeyEntry apiKey={mockApiKey} onKeyChanged={vi.fn()} />, {
        wrapper: makeWrapper(),
      });
      expandEntry();

      fireEvent.click(screen.getByTestId("revoke-key-key-001"));
      fireEvent.click(screen.getByTestId("revoke-key-key-001"));

      await waitFor(() => {
        expect(screen.getByText("Forbidden")).toBeInTheDocument();
      });
    });
  });

  describe("save projects", () => {
    it("calls updateProjects mutation with key id and selected ids", () => {
      const updateProjectsMutate = vi.fn();
      vi.mocked(useUpdateKeyProjects).mockReturnValue({
        mutate: updateProjectsMutate,
        isPending: false,
      } as unknown as ReturnType<typeof useUpdateKeyProjects>);

      render(<ApiKeyEntry apiKey={mockApiKey} onKeyChanged={vi.fn()} />, {
        wrapper: makeWrapper(),
      });
      fireEvent.click(screen.getByTestId("expand-key-key-001"));
      fireEvent.click(screen.getByTestId("save-projects-key-001"));

      expect(updateProjectsMutate).toHaveBeenCalledWith(
        { id: "key-001", projectIds: ["proj-1"] },
        expect.any(Object)
      );
    });
  });

  describe("save permissions", () => {
    it("calls updatePermissions mutation with key id and permissions value", () => {
      const updatePermsMutate = vi.fn();
      vi.mocked(useUpdateKeyPermissions).mockReturnValue({
        mutate: updatePermsMutate,
        isPending: false,
      } as unknown as ReturnType<typeof useUpdateKeyPermissions>);

      render(<ApiKeyEntry apiKey={mockApiKey} onKeyChanged={vi.fn()} />, {
        wrapper: makeWrapper(),
      });
      fireEvent.click(screen.getByTestId("expand-key-key-001"));
      fireEvent.click(screen.getByTestId("save-permissions-key-001"));

      expect(updatePermsMutate).toHaveBeenCalledWith(
        { id: "key-001", permissions: 3 },
        expect.any(Object)
      );
    });
  });

  describe("rotate key", () => {
    it("clicking Rotate Key opens RotateKeyDialog", () => {
      render(<ApiKeyEntry apiKey={mockApiKey} onKeyChanged={vi.fn()} />, {
        wrapper: makeWrapper(),
      });
      fireEvent.click(screen.getByTestId("expand-key-key-001"));
      fireEvent.click(screen.getByTestId("rotate-key-key-001"));

      expect(screen.getByTestId("rotate-key-dialog")).toBeInTheDocument();
    });
  });
});
