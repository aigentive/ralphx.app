/**
 * Tests for AuditLogViewer component
 *
 * Covers: loading state, error state, empty state, entry rendering,
 * success/fail status display, latency formatting.
 */

import { render, screen } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { AuditLogViewer } from "./AuditLogViewer";
import type { AuditLogEntry } from "@/types/api-key";

// Mock useApiKeyAuditLog hook
vi.mock("@/hooks/useApiKeys", () => ({
  useApiKeyAuditLog: vi.fn(),
  useApiKeys: vi.fn(),
  useCreateApiKey: vi.fn(),
  useRevokeApiKey: vi.fn(),
  useRotateApiKey: vi.fn(),
  useUpdateKeyProjects: vi.fn(),
  useUpdateKeyPermissions: vi.fn(),
}));

import { useApiKeyAuditLog } from "@/hooks/useApiKeys";

const mockEntries: AuditLogEntry[] = [
  {
    id: 1,
    api_key_id: "key-001",
    tool_name: "list_tasks",
    project_id: "proj-1",
    success: true,
    latency_ms: 42,
    created_at: "2024-03-01T10:00:00Z",
  },
  {
    id: 2,
    api_key_id: "key-001",
    tool_name: "create_task",
    project_id: "proj-1",
    success: false,
    latency_ms: 1500,
    created_at: "2024-03-01T11:00:00Z",
  },
  {
    id: 3,
    api_key_id: "key-001",
    tool_name: "get_task_details",
    project_id: null,
    success: true,
    latency_ms: null,
    created_at: "2024-03-01T12:00:00Z",
  },
];

function makeWrapper() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={qc}>{children}</QueryClientProvider>
  );
}

describe("AuditLogViewer", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("loading state", () => {
    it("shows loading indicator while loading", () => {
      vi.mocked(useApiKeyAuditLog).mockReturnValue({
        data: undefined,
        isLoading: true,
        error: null,
      } as ReturnType<typeof useApiKeyAuditLog>);

      render(<AuditLogViewer keyId="key-001" />, { wrapper: makeWrapper() });

      expect(screen.getByText(/Loading audit log/)).toBeInTheDocument();
    });
  });

  describe("error state", () => {
    it("shows error message when query fails", () => {
      vi.mocked(useApiKeyAuditLog).mockReturnValue({
        data: undefined,
        isLoading: false,
        error: new Error("Network error"),
      } as ReturnType<typeof useApiKeyAuditLog>);

      render(<AuditLogViewer keyId="key-001" />, { wrapper: makeWrapper() });

      expect(screen.getByText("Network error")).toBeInTheDocument();
    });
  });

  describe("empty state", () => {
    it("shows empty message when no entries", () => {
      vi.mocked(useApiKeyAuditLog).mockReturnValue({
        data: [],
        isLoading: false,
        error: null,
      } as ReturnType<typeof useApiKeyAuditLog>);

      render(<AuditLogViewer keyId="key-001" />, { wrapper: makeWrapper() });

      expect(screen.getByText("No requests logged yet")).toBeInTheDocument();
    });
  });

  describe("entry rendering", () => {
    beforeEach(() => {
      vi.mocked(useApiKeyAuditLog).mockReturnValue({
        data: mockEntries,
        isLoading: false,
        error: null,
      } as ReturnType<typeof useApiKeyAuditLog>);
    });

    it("renders audit-log-viewer container", () => {
      render(<AuditLogViewer keyId="key-001" />, { wrapper: makeWrapper() });
      expect(screen.getByTestId("audit-log-viewer")).toBeInTheDocument();
    });

    it("renders tool names for all entries", () => {
      render(<AuditLogViewer keyId="key-001" />, { wrapper: makeWrapper() });

      expect(screen.getByText("list_tasks")).toBeInTheDocument();
      expect(screen.getByText("create_task")).toBeInTheDocument();
      expect(screen.getByText("get_task_details")).toBeInTheDocument();
    });

    it("renders entry test ids", () => {
      render(<AuditLogViewer keyId="key-001" />, { wrapper: makeWrapper() });

      expect(screen.getByTestId("audit-entry-1")).toBeInTheDocument();
      expect(screen.getByTestId("audit-entry-2")).toBeInTheDocument();
    });

    it("shows OK for successful entries", () => {
      render(<AuditLogViewer keyId="key-001" />, { wrapper: makeWrapper() });

      const okTexts = screen.getAllByText("OK");
      expect(okTexts.length).toBeGreaterThan(0);
    });

    it("shows Failed for failed entries", () => {
      render(<AuditLogViewer keyId="key-001" />, { wrapper: makeWrapper() });

      expect(screen.getByText("Failed")).toBeInTheDocument();
    });

    it("formats latency_ms < 1000 as Nms", () => {
      render(<AuditLogViewer keyId="key-001" />, { wrapper: makeWrapper() });

      expect(screen.getByText("42ms")).toBeInTheDocument();
    });

    it("formats latency_ms >= 1000 as N.Xs", () => {
      render(<AuditLogViewer keyId="key-001" />, { wrapper: makeWrapper() });

      expect(screen.getByText("1.5s")).toBeInTheDocument();
    });

    it("renders — for null latency", () => {
      render(<AuditLogViewer keyId="key-001" />, { wrapper: makeWrapper() });

      expect(screen.getByText("—")).toBeInTheDocument();
    });
  });
});
