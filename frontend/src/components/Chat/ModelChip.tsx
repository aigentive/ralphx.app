import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import type { ModelDisplay } from "@/types/chat-conversation";

interface ModelChipProps {
  model: ModelDisplay;
}

export function ModelChip({ model }: ModelChipProps) {
  const displayLabel =
    model.label.length > 20 ? model.label.slice(0, 17) + "..." : model.label;

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <span className="text-xs text-white/40 shrink-0 cursor-default select-none">
          {displayLabel}
        </span>
      </TooltipTrigger>
      <TooltipContent>Full model: {model.id}</TooltipContent>
    </Tooltip>
  );
}
