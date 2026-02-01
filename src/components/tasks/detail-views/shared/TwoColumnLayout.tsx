/**
 * TwoColumnLayout - Consistent layout for task detail views
 *
 * Left column: Description (fixed)
 * Right column: Main content (varies by view)
 */

import { SectionTitle } from "./SectionTitle";
import { DescriptionBlock } from "./DescriptionBlock";

interface TwoColumnLayoutProps {
  description: string | null | undefined;
  children: React.ReactNode;
  testId?: string;
}

export function TwoColumnLayout({
  description,
  children,
  testId,
}: TwoColumnLayoutProps) {
  return (
    <div
      data-testid={testId}
      className="grid grid-cols-[280px_1fr] gap-6 min-h-0"
    >
      {/* Left column - Description */}
      <div className="space-y-2">
        <SectionTitle>Description</SectionTitle>
        <DescriptionBlock description={description} />
      </div>

      {/* Right column - Main content */}
      <div className="space-y-6 min-w-0">{children}</div>
    </div>
  );
}
