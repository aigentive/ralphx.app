export function isTranscriptRootReadyForReveal(root: ParentNode | null): boolean {
  if (!root) {
    return false;
  }

  const virtuosoList = root.querySelector<HTMLElement>('[data-testid="virtuoso-item-list"]');
  if (virtuosoList && window.getComputedStyle(virtuosoList).visibility === "hidden") {
    return false;
  }
  if (virtuosoList) {
    return virtuosoList.children.length > 0;
  }

  return Boolean(root.querySelector('[data-chat-message-item="true"]'));
}
