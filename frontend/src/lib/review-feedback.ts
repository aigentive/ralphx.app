const ANSI_ESCAPE_RE = new RegExp(
  `${String.fromCharCode(27)}\\[[0-9;?]*[ -/]*[@-~]`,
  "g"
);

export function sanitizeReviewFeedbackText(text: string): string {
  return text
    .replace(ANSI_ESCAPE_RE, "")
    .replace(/\r\n/g, "\n")
    .replace(/\r/g, "\n")
    .trim();
}

export function buildReviewFeedbackPreview(
  text: string,
  previewCharLimit = 900
): string {
  const condensed = sanitizeReviewFeedbackText(text).replace(/\s+/g, " ").trim();
  if (condensed.length <= previewCharLimit) {
    return condensed;
  }
  return `${condensed.slice(0, previewCharLimit).trimEnd()}...`;
}

export function getReviewFeedbackHeading(
  reviewer: string,
  compact = false
): string {
  if (reviewer === "ai") {
    return compact ? "AI Feedback" : "AI Review Feedback";
  }
  if (reviewer === "system") {
    return compact ? "System Feedback" : "System Review Feedback";
  }
  return compact ? "Human Feedback" : "Human Review Feedback";
}

export function getReviewerActorLabel(reviewer: string): string {
  if (reviewer === "ai") {
    return "AI Reviewer";
  }
  if (reviewer === "system") {
    return "System Reviewer";
  }
  return "Human Reviewer";
}

export function getReviewerTypeLabel(reviewer: string): string {
  if (reviewer === "ai") {
    return "AI Review";
  }
  if (reviewer === "system") {
    return "System Review";
  }
  return "Human Review";
}
