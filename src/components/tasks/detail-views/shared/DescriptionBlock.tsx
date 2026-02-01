/**
 * DescriptionBlock - Clean description text with empty state
 */

interface DescriptionBlockProps {
  description: string | null | undefined;
  testId?: string;
}

export function DescriptionBlock({ description, testId }: DescriptionBlockProps) {
  if (!description) {
    return (
      <p className="text-[13px] italic text-white/30">
        No description provided
      </p>
    );
  }

  return (
    <p
      data-testid={testId}
      className="text-[13px] text-white/65 leading-relaxed"
      style={{ wordBreak: "break-word" }}
    >
      {description}
    </p>
  );
}
