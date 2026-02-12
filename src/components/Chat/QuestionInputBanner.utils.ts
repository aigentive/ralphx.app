/**
 * Utility functions for QuestionInputBanner component
 */

import type { AskUserQuestionPayload } from "@/types/ask-user-question";

/**
 * Estimate the required height for a question banner based on content.
 * Accounts for question text wrapping, chip rows, and fixed overhead.
 *
 * Estimation strategy:
 * - Question text: ~45 chars per line × 20px line height
 * - Chip rows: Accumulate label widths (~7px/char + 48px padding per chip),
 *   wrap at ~280px container width
 * - Fixed: Header (36px) + body padding (10px top + 12px bottom)
 * - Clamp to [120px, 320px] range
 */
export function computeQuestionHeight(question: AskUserQuestionPayload): number {
  let totalHeight = 0;

  // Header section: ? icon + header text + dismiss button (36px)
  totalHeight += 36;

  // Body padding: 10px top + 12px bottom
  totalHeight += 22;

  // Question text estimation
  // ~45 characters per line at 13px font size, 1.45 line height = 20px per line
  const questionText = question.question || "";
  const charsPerLine = 45;
  const lineHeight = 20;
  const questionLines = Math.ceil(questionText.length / charsPerLine);
  totalHeight += questionLines * lineHeight + 10; // 10px margin-bottom on paragraph

  // Chip rows estimation
  // Each chip: ~7px per character + 48px padding (number circle + gaps + padding)
  // Container width: ~280px (accounting for 12px side padding)
  const containerWidth = 280;
  const chipPadding = 48; // number circle + gaps + padding

  const options = question.options || [];
  let currentRowWidth = 0;
  let chipRowCount = 0;
  const gapBetweenChips = 6; // gap-1.5 = 6px

  for (let i = 0; i < options.length; i++) {
    const label = options[i]?.label || "";
    const chipWidth = label.length * 7 + chipPadding;

    if (currentRowWidth === 0) {
      // First chip in row
      currentRowWidth = chipWidth;
      chipRowCount = 1;
    } else if (currentRowWidth + gapBetweenChips + chipWidth <= containerWidth) {
      // Fits in current row
      currentRowWidth += gapBetweenChips + chipWidth;
    } else {
      // Need new row
      currentRowWidth = chipWidth;
      chipRowCount += 1;
    }
  }

  // Chip row height: 18px + 5px padding + gap between rows = ~28px per row
  const chipRowHeight = 28;
  if (options.length > 0) {
    totalHeight += chipRowCount * chipRowHeight;
  }

  // Clamp to [120px, 320px]
  return Math.max(120, Math.min(320, totalHeight));
}
