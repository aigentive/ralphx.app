export const VISUAL_BOTTOM_EPSILON_PX = 2;

export function getTrueBottomScrollTop(
  element: Pick<HTMLElement, "scrollHeight" | "clientHeight">
): number {
  return Math.max(0, element.scrollHeight - element.clientHeight);
}

export function getScrollBottomDelta(
  element: Pick<HTMLElement, "scrollHeight" | "clientHeight" | "scrollTop">
): number {
  return Math.max(0, getTrueBottomScrollTop(element) - element.scrollTop);
}

export function isScrollElementVisuallyAtBottom(
  element: Pick<HTMLElement, "scrollHeight" | "clientHeight" | "scrollTop">
): boolean {
  return getScrollBottomDelta(element) <= VISUAL_BOTTOM_EPSILON_PX;
}

export interface ScrollToBottomControlState {
  timelineLength: number;
  scrollToTimestamp: string | null | undefined;
  hasScrollerElement: boolean;
  hasScrollableOverflow: boolean;
  isAtBottom: boolean;
  isVisuallyAtBottom: boolean;
  isLastItemVisible: boolean | null;
}

export function shouldShowScrollToBottomControl({
  timelineLength,
  scrollToTimestamp,
  hasScrollerElement,
  hasScrollableOverflow,
  isAtBottom,
  isVisuallyAtBottom,
  isLastItemVisible,
}: ScrollToBottomControlState): boolean {
  if (scrollToTimestamp || timelineLength === 0) {
    return false;
  }

  if (hasScrollerElement && !hasScrollableOverflow && isLastItemVisible !== false) {
    return false;
  }

  if (isLastItemVisible === false) {
    return true;
  }

  return hasScrollerElement ? !isVisuallyAtBottom : !isAtBottom;
}
