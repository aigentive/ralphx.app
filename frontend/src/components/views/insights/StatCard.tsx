import { HelpCircle } from "lucide-react";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";

interface StatCardProps {
  label: string;
  value: string;
  sub?: string;
  tooltip?: string;
}

export function StatCard({ label, value, sub, tooltip }: StatCardProps) {
  return (
    <div
      className="flex flex-col gap-1 rounded-xl"
      style={{ backgroundColor: "hsl(220 10% 12%)", padding: "14px 16px" }}
    >
      <span
        className="flex items-center gap-1 text-[11px] font-semibold uppercase tracking-wider"
        style={{ color: "rgba(255,255,255,0.4)", letterSpacing: "0.08em" }}
      >
        {label}
        {tooltip !== undefined && (
          <TooltipProvider delayDuration={300}>
            <Tooltip>
              <TooltipTrigger asChild>
                <HelpCircle className="inline w-3.5 h-3.5 text-muted-foreground cursor-help" />
              </TooltipTrigger>
              <TooltipContent side="top" className="max-w-[240px] text-xs normal-case tracking-normal font-normal">
                {tooltip}
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        )}
      </span>
      <span
        className="text-[22px] font-semibold"
        style={{ color: "rgba(255,255,255,0.92)", fontFamily: "system-ui" }}
      >
        {value}
      </span>
      {sub !== undefined && (
        <span className="text-[12px]" style={{ color: "rgba(255,255,255,0.4)" }}>
          {sub}
        </span>
      )}
    </div>
  );
}
