import { describe, expect, it } from "vitest";

import {
  getTrueBottomScrollTop,
  isScrollElementVisuallyAtBottom,
  shouldShowScrollToBottomControl,
  VISUAL_BOTTOM_EPSILON_PX,
} from "./ChatMessageList.scroll";

function scrollElement({
  scrollHeight,
  clientHeight,
  scrollTop,
}: {
  scrollHeight: number;
  clientHeight: number;
  scrollTop: number;
}): HTMLElement {
  const element = document.createElement("div");
  Object.defineProperties(element, {
    scrollHeight: { configurable: true, value: scrollHeight },
    clientHeight: { configurable: true, value: clientHeight },
    scrollTop: { configurable: true, writable: true, value: scrollTop },
  });
  return element;
}

describe("ChatMessageList scroll math", () => {
  it("does not treat the near-bottom threshold as visually at bottom", () => {
    const element = scrollElement({
      scrollHeight: 1000,
      clientHeight: 500,
      scrollTop: 400,
    });

    expect(isScrollElementVisuallyAtBottom(element)).toBe(false);
  });

  it("treats only the exact bottom plus a tiny subpixel epsilon as visually at bottom", () => {
    const target = 500;

    expect(
      isScrollElementVisuallyAtBottom(
        scrollElement({
          scrollHeight: 1000,
          clientHeight: 500,
          scrollTop: target,
        })
      )
    ).toBe(true);
    expect(
      isScrollElementVisuallyAtBottom(
        scrollElement({
          scrollHeight: 1000,
          clientHeight: 500,
          scrollTop: target - VISUAL_BOTTOM_EPSILON_PX + 0.25,
        })
      )
    ).toBe(true);
    expect(
      isScrollElementVisuallyAtBottom(
        scrollElement({
          scrollHeight: 1000,
          clientHeight: 500,
          scrollTop: target - VISUAL_BOTTOM_EPSILON_PX - 0.25,
        })
      )
    ).toBe(false);
  });

  it("targets the scroll container's absolute bottom", () => {
    expect(
      getTrueBottomScrollTop(
        scrollElement({
          scrollHeight: 1320,
          clientHeight: 500,
          scrollTop: 0,
        })
      )
    ).toBe(820);
    expect(
      getTrueBottomScrollTop(
        scrollElement({
          scrollHeight: 320,
          clientHeight: 500,
          scrollTop: 0,
        })
      )
    ).toBe(0);
  });

  it("shows the scroll-to-bottom control when Virtuoso range is above the last item even if bottom state is stale", () => {
    expect(
      shouldShowScrollToBottomControl({
        hasScrollerElement: true,
        hasScrollableOverflow: true,
        isAtBottom: true,
        isLastItemVisible: false,
        isVisuallyAtBottom: true,
        scrollToTimestamp: null,
        timelineLength: 10,
      })
    ).toBe(true);
  });

  it("hides the scroll-to-bottom control at the visual bottom", () => {
    expect(
      shouldShowScrollToBottomControl({
        hasScrollerElement: true,
        hasScrollableOverflow: true,
        isAtBottom: true,
        isLastItemVisible: true,
        isVisuallyAtBottom: true,
        scrollToTimestamp: null,
        timelineLength: 10,
      })
    ).toBe(false);
  });

  it("keeps existing non-scroller fallback visibility behavior", () => {
    expect(
      shouldShowScrollToBottomControl({
        hasScrollerElement: false,
        hasScrollableOverflow: false,
        isAtBottom: false,
        isLastItemVisible: null,
        isVisuallyAtBottom: true,
        scrollToTimestamp: null,
        timelineLength: 10,
      })
    ).toBe(true);
  });

  it("shows the scroll-to-bottom control for a short timeline when content overflows and is not at bottom", () => {
    expect(
      shouldShowScrollToBottomControl({
        hasScrollerElement: true,
        hasScrollableOverflow: true,
        isAtBottom: true,
        isLastItemVisible: true,
        isVisuallyAtBottom: false,
        scrollToTimestamp: null,
        timelineLength: 2,
      })
    ).toBe(true);
  });

  it("hides the scroll-to-bottom control when the scroller has no overflow", () => {
    expect(
      shouldShowScrollToBottomControl({
        hasScrollerElement: true,
        hasScrollableOverflow: false,
        isAtBottom: false,
        isLastItemVisible: true,
        isVisuallyAtBottom: false,
        scrollToTimestamp: null,
        timelineLength: 2,
      })
    ).toBe(false);
  });
});
