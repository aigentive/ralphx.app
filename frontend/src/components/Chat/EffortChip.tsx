import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";

interface EffortChipProps {
  effort: string;
}

function formatEffortLabel(effort: string): string {
  const normalized = effort.trim().toLowerCase();
  switch (normalized) {
    case "low":
      return "Low";
    case "medium":
      return "Medium";
    case "high":
      return "High";
    case "xhigh":
      return "XHigh";
    default:
      return effort;
  }
}

export function EffortChip({ effort }: EffortChipProps) {
  const label = formatEffortLabel(effort);

  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <span className="text-xs text-text-primary/32 shrink-0 cursor-default select-none">
            {label}
          </span>
        </TooltipTrigger>
        <TooltipContent>Reasoning effort: {label}</TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}
