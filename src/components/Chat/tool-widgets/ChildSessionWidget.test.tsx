import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { createElement } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ChildSessionWidget } from "./ChildSessionWidget";
import { ChildSessionNavigationContext } from "./ChildSessionNavigationContext";
import type { ToolCall } from "./shared.constants";
import type { ChildSessionStatusResponse } from "@/api/chat";

// Mock the useChildSessionStatus hook
vi.mock("@/hooks/useChildSessionStatus", () => ({
  useChildSessionStatus: vi.fn(),
}));

import { useChildSessionStatus } from "@/hooks/useChildSessionStatus";
const mockedUseChildSessionStatus = vi.mocked(useChildSessionStatus);

function makeToolCall(overrides: Partial<ToolCall> = {}): ToolCall {
  return {
    id: "child-session-1",
    name: "mcp__ralphx__create_child_session",
    arguments: {},
    ...overrides,
  };
}

function mcpWrap(obj: unknown): unknown {
  return [{ type: "text", text: JSON.stringify(obj) }];
}

function makeStatusResponse(
  estimatedStatus: "idle" | "likely_generating" | "likely_waiting",
  messages: { role: string; content: string; created_at: string | null }[] = []
): ChildSessionStatusResponse {
  return {
    session_id: "uuid-123",
    title: "Test Session",
    agent_state: { estimated_status: estimatedStatus },
    recent_messages: messages,
  };
}

function makeQueryClient() {
  return new QueryClient({ defaultOptions: { queries: { retry: false } } });
}

function renderWithProviders(
  ui: React.ReactNode,
  onNavigate?: (sessionId: string) => void
) {
  const queryClient = makeQueryClient();
  return render(
    createElement(
      QueryClientProvider,
      { client: queryClient },
      createElement(
        ChildSessionNavigationContext.Provider,
        { value: onNavigate ?? (() => {}) },
        ui
      )
    )
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  // Default: no data, not loading
  mockedUseChildSessionStatus.mockReturnValue({
    data: undefined,
    isLoading: false,
    isError: false,
    refetch: vi.fn(),
  } as ReturnType<typeof useChildSessionStatus>);
});

describe("ChildSessionWidget", () => {
  // ===== Original Phase 1 tests (preserved) =====

  it("shows loading state when no title available", () => {
    const toolCall = makeToolCall({ arguments: {}, result: mcpWrap({}) });
    renderWithProviders(<ChildSessionWidget toolCall={toolCall} />);
    expect(screen.getByText("Creating session...")).toBeInTheDocument();
  });

  it("renders session title from arguments inside WidgetCard", () => {
    const toolCall = makeToolCall({
      arguments: { title: "Plan Verification Session", purpose: "verification" },
      result: mcpWrap({ session_id: "uuid-123", orchestration_triggered: true }),
    });
    renderWithProviders(<ChildSessionWidget toolCall={toolCall} />);
    expect(screen.getAllByText("Plan Verification Session").length).toBeGreaterThan(0);
    expect(screen.getByText("verification")).toBeInTheDocument();
    expect(screen.getByText("Agent spawned")).toBeInTheDocument();
  });

  it("renders general purpose badge as muted (no Agent spawned badge)", () => {
    const toolCall = makeToolCall({
      arguments: { title: "General Session", purpose: "general" },
      result: mcpWrap({ session_id: "uuid-456", orchestration_triggered: false }),
    });
    renderWithProviders(<ChildSessionWidget toolCall={toolCall} />);
    expect(screen.getAllByText("General Session").length).toBeGreaterThan(0);
    expect(screen.getByText("general")).toBeInTheDocument();
    expect(screen.queryByText("Agent spawned")).not.toBeInTheDocument();
  });

  it("falls back to title from result when arguments has no title", () => {
    const toolCall = makeToolCall({
      arguments: {},
      result: mcpWrap({ title: "Result Title", session_id: "uuid-789" }),
    });
    renderWithProviders(<ChildSessionWidget toolCall={toolCall} />);
    expect(screen.getAllByText("Result Title").length).toBeGreaterThan(0);
  });

  it("renders without purpose badge when purpose is absent", () => {
    const toolCall = makeToolCall({
      arguments: { title: "No Purpose Session" },
      result: mcpWrap({ session_id: "uuid-000", orchestration_triggered: false }),
    });
    renderWithProviders(<ChildSessionWidget toolCall={toolCall} />);
    expect(screen.getAllByText("No Purpose Session").length).toBeGreaterThan(0);
    expect(screen.queryByText("Agent spawned")).not.toBeInTheDocument();
  });

  // ===== Phase 2 tests (new) =====

  it("shows loading skeleton while status is being fetched", () => {
    mockedUseChildSessionStatus.mockReturnValue({
      data: undefined,
      isLoading: true,
      isError: false,
      refetch: vi.fn(),
    } as ReturnType<typeof useChildSessionStatus>);

    const toolCall = makeToolCall({
      arguments: { title: "Active Session" },
      result: mcpWrap({ session_id: "uuid-123" }),
    });
    renderWithProviders(<ChildSessionWidget toolCall={toolCall} />);
    expect(screen.getByLabelText("Loading messages")).toBeInTheDocument();
  });

  it("shows error state with retry button on fetch failure", () => {
    const refetch = vi.fn();
    mockedUseChildSessionStatus.mockReturnValue({
      data: undefined,
      isLoading: false,
      isError: true,
      refetch,
    } as ReturnType<typeof useChildSessionStatus>);

    const toolCall = makeToolCall({
      arguments: { title: "Active Session" },
      result: mcpWrap({ session_id: "uuid-123" }),
    });
    renderWithProviders(<ChildSessionWidget toolCall={toolCall} />);
    expect(screen.getByLabelText("Unable to load session")).toBeInTheDocument();
    expect(screen.getByText("Retry")).toBeInTheDocument();
  });

  it("calls refetch when Retry is clicked", () => {
    const refetch = vi.fn().mockResolvedValue(undefined);
    mockedUseChildSessionStatus.mockReturnValue({
      data: undefined,
      isLoading: false,
      isError: true,
      refetch,
    } as ReturnType<typeof useChildSessionStatus>);

    const toolCall = makeToolCall({
      arguments: { title: "Active Session" },
      result: mcpWrap({ session_id: "uuid-123" }),
    });
    renderWithProviders(<ChildSessionWidget toolCall={toolCall} />);
    fireEvent.click(screen.getByText("Retry"));
    expect(refetch).toHaveBeenCalledTimes(1);
  });

  it("shows message previews when data is available", () => {
    mockedUseChildSessionStatus.mockReturnValue({
      data: makeStatusResponse("likely_generating", [
        { role: "user", content: "Hello from user", created_at: null },
        { role: "assistant", content: "Hello from assistant", created_at: null },
      ]),
      isLoading: false,
      isError: false,
      refetch: vi.fn(),
    } as ReturnType<typeof useChildSessionStatus>);

    const toolCall = makeToolCall({
      arguments: { title: "Live Session" },
      result: mcpWrap({ session_id: "uuid-123" }),
    });
    renderWithProviders(<ChildSessionWidget toolCall={toolCall} />);
    // Both texts appear in message previews (last message also appears in collapsed snippet)
    expect(screen.getAllByText("Hello from user").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Hello from assistant").length).toBeGreaterThan(0);
  });

  it("shows agent status badge when agent is generating", () => {
    mockedUseChildSessionStatus.mockReturnValue({
      data: makeStatusResponse("likely_generating"),
      isLoading: false,
      isError: false,
      refetch: vi.fn(),
    } as ReturnType<typeof useChildSessionStatus>);

    const toolCall = makeToolCall({
      arguments: { title: "Active Session" },
      result: mcpWrap({ session_id: "uuid-123" }),
    });
    renderWithProviders(<ChildSessionWidget toolCall={toolCall} />);
    expect(screen.getByText("Generating")).toBeInTheDocument();
  });

  it("shows agent status badge when agent is waiting", () => {
    mockedUseChildSessionStatus.mockReturnValue({
      data: makeStatusResponse("likely_waiting"),
      isLoading: false,
      isError: false,
      refetch: vi.fn(),
    } as ReturnType<typeof useChildSessionStatus>);

    const toolCall = makeToolCall({
      arguments: { title: "Waiting Session" },
      result: mcpWrap({ session_id: "uuid-123" }),
    });
    renderWithProviders(<ChildSessionWidget toolCall={toolCall} />);
    expect(screen.getByText("Waiting")).toBeInTheDocument();
  });

  it("calls navigation context when Open Session button is clicked", () => {
    mockedUseChildSessionStatus.mockReturnValue({
      data: makeStatusResponse("idle"),
      isLoading: false,
      isError: false,
      refetch: vi.fn(),
    } as ReturnType<typeof useChildSessionStatus>);

    const onNavigate = vi.fn();
    const toolCall = makeToolCall({
      arguments: { title: "My Session" },
      result: mcpWrap({ session_id: "uuid-123" }),
    });
    renderWithProviders(<ChildSessionWidget toolCall={toolCall} />, onNavigate);
    fireEvent.click(screen.getByText("Open Session"));
    expect(onNavigate).toHaveBeenCalledWith("uuid-123");
  });

  it("does not show Open Session button when session_id is absent", () => {
    mockedUseChildSessionStatus.mockReturnValue({
      data: undefined,
      isLoading: false,
      isError: false,
      refetch: vi.fn(),
    } as ReturnType<typeof useChildSessionStatus>);

    const toolCall = makeToolCall({
      arguments: { title: "No ID Session" },
      result: mcpWrap({}), // no session_id
    });
    renderWithProviders(<ChildSessionWidget toolCall={toolCall} />);
    expect(screen.queryByText("Open Session")).not.toBeInTheDocument();
  });

  it("passes session_id to useChildSessionStatus hook", () => {
    const toolCall = makeToolCall({
      arguments: { title: "My Session" },
      result: mcpWrap({ session_id: "test-session-id" }),
    });
    renderWithProviders(<ChildSessionWidget toolCall={toolCall} />);
    expect(mockedUseChildSessionStatus).toHaveBeenCalledWith("test-session-id");
  });

  // ===== Phase 3 tests (layout fixes) =====

  it("shows static 'Verification Session' label in header when purpose is verification", () => {
    const toolCall = makeToolCall({
      arguments: { title: "Custom Verification Title", purpose: "verification" },
      result: mcpWrap({ session_id: "uuid-123" }),
    });
    renderWithProviders(<ChildSessionWidget toolCall={toolCall} />);
    // Header shows static label
    expect(screen.getByText("Verification Session")).toBeInTheDocument();
    // Full session title also appears in body
    expect(screen.getByText("Custom Verification Title")).toBeInTheDocument();
  });

  it("shows static 'Follow-up Session' label in header when purpose is not verification", () => {
    const toolCall = makeToolCall({
      arguments: { title: "Custom Follow-up Title", purpose: "general" },
      result: mcpWrap({ session_id: "uuid-456" }),
    });
    renderWithProviders(<ChildSessionWidget toolCall={toolCall} />);
    expect(screen.getByText("Follow-up Session")).toBeInTheDocument();
    expect(screen.getByText("Custom Follow-up Title")).toBeInTheDocument();
  });

  it("Open Session button is visible in collapsed (default) state — button lives in header badge area", () => {
    const toolCall = makeToolCall({
      arguments: { title: "Collapsed Session" },
      result: mcpWrap({ session_id: "uuid-123" }),
    });
    renderWithProviders(<ChildSessionWidget toolCall={toolCall} />);
    expect(screen.getByRole("button", { name: "Open Session" })).toBeInTheDocument();
  });

  it("clicking Open Session button calls onNavigate and stops React synthetic event propagation", () => {
    const onNavigate = vi.fn();
    const parentClickHandler = vi.fn();
    const toolCall = makeToolCall({
      arguments: { title: "My Session" },
      result: mcpWrap({ session_id: "uuid-123" }),
    });
    const queryClient = makeQueryClient();
    render(
      createElement(
        QueryClientProvider,
        { client: queryClient },
        createElement(
          ChildSessionNavigationContext.Provider,
          { value: onNavigate },
          createElement("div", { onClick: parentClickHandler },
            createElement(ChildSessionWidget, { toolCall })
          )
        )
      )
    );

    fireEvent.click(screen.getByRole("button", { name: "Open Session" }));

    expect(onNavigate).toHaveBeenCalledWith("uuid-123");
    expect(parentClickHandler).not.toHaveBeenCalled();
  });
});
