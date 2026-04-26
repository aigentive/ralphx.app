import { describe, expect, it } from "vitest";

import {
  getTrueBottomScrollTop,
  isScrollElementVisuallyAtBottom,
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
});
