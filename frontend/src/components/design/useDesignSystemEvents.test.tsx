import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, renderHook } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { designSystemKeys } from "./useProjectDesignSystems";
import { useDesignSystemEvents } from "./useDesignSystemEvents";

const { callbacks, subscribeMock } = vi.hoisted(() => ({
  callbacks: {} as Record<string, (payload: unknown) => void>,
  subscribeMock: vi.fn((eventName: string, callback: (payload: unknown) => void) => {
    callbacks[eventName] = callback;
    return () => {
      delete callbacks[eventName];
    };
  }),
}));

vi.mock("@/providers/EventProvider", () => ({
  useEventBus: () => ({
    subscribe: subscribeMock,
  }),
}));

function wrapperFor(queryClient: QueryClient) {
  return function Wrapper({ children }: { children: ReactNode }) {
    return <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>;
  };
}

describe("useDesignSystemEvents", () => {
  beforeEach(() => {
    for (const eventName of Object.keys(callbacks)) {
      delete callbacks[eventName];
    }
    subscribeMock.mockClear();
  });

  it("subscribes to design workspace events", () => {
    const queryClient = new QueryClient();
    const { unmount } = renderHook(() => useDesignSystemEvents(), {
      wrapper: wrapperFor(queryClient),
    });

    expect(subscribeMock).toHaveBeenCalledWith("design:schema_published", expect.any(Function));
    expect(subscribeMock).toHaveBeenCalledWith("design:artifact_created", expect.any(Function));
    expect(subscribeMock).toHaveBeenCalledWith(
      "design:styleguide_item_approved",
      expect.any(Function),
    );
    expect(subscribeMock).toHaveBeenCalledWith("design:run_started", expect.any(Function));
    expect(subscribeMock).toHaveBeenCalledWith("design:export_completed", expect.any(Function));
    expect(subscribeMock).toHaveBeenCalledWith("design:import_completed", expect.any(Function));

    unmount();
    expect(callbacks["design:schema_published"]).toBeUndefined();
  });

  it("invalidates design detail, project, and styleguide queries from schema publish events", () => {
    const queryClient = new QueryClient();
    const invalidateSpy = vi.spyOn(queryClient, "invalidateQueries");
    renderHook(() => useDesignSystemEvents(), {
      wrapper: wrapperFor(queryClient),
    });

    act(() => {
      callbacks["design:schema_published"]?.({
        designSystem: {
          id: "design-system-1",
          primaryProjectId: "project-1",
        },
      });
    });

    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: designSystemKeys.all,
    });
    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: designSystemKeys.project("project-1"),
    });
    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: designSystemKeys.detail("design-system-1"),
    });
    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: designSystemKeys.styleguideItems("design-system-1"),
    });
    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: designSystemKeys.styleguideViewModel("design-system-1"),
    });
  });
});
