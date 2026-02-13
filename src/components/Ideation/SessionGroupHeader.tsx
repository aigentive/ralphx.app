import type { LucideIcon } from "lucide-react";
import { ChevronDown } from "lucide-react";
import { cn } from "@/lib/utils";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";

export interface SessionGroupHeaderProps {
  icon: LucideIcon;
  label: string;
  count: number;
  isOpen: boolean;
  onToggle: (open: boolean) => void;
  /** HSL string e.g. "hsl(14 100% 60%)" — used for count badge tint */
  accentColor?: string;
  children: React.ReactNode;
}

/** Convert "hsl(H S% L%)" to "hsla(H S% L% / alpha)" */
function withAlpha(hsl: string, alpha: number): string {
  return hsl.replace("hsl(", "hsla(").replace(")", ` / ${alpha})`);
}

export function SessionGroupHeader({
  icon: Icon,
  label,
  count,
  isOpen,
  onToggle,
  accentColor,
  children,
}: SessionGroupHeaderProps) {
  return (
    <Collapsible open={isOpen} onOpenChange={onToggle} className="mt-3">
      <CollapsibleTrigger asChild>
        <button
          className="w-full flex items-center gap-2 px-2 py-1.5 rounded-md transition-colors duration-150"
          style={{
            color: "hsl(220 10% 50%)",
          }}
          onMouseEnter={(e) => {
            e.currentTarget.style.background = "hsla(220 10% 100% / 0.04)";
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.background = "transparent";
          }}
        >
          <Icon className="w-3.5 h-3.5" />
          <span className="text-[11px] font-medium tracking-[-0.01em]">
            {label}
          </span>
          {count > 0 && (
            <span
              className="text-[9px] px-1.5 rounded-full font-medium leading-[16px]"
              style={{
                background: accentColor
                  ? withAlpha(accentColor, 0.15)
                  : "hsla(220 10% 50% / 0.15)",
                color: accentColor ?? "hsl(220 10% 60%)",
              }}
            >
              {count}
            </span>
          )}
          <ChevronDown
            className={cn(
              "w-3 h-3 ml-auto transition-transform duration-200",
              isOpen && "rotate-180"
            )}
          />
        </button>
      </CollapsibleTrigger>
      <CollapsibleContent className="mt-1 space-y-1">
        {children}
      </CollapsibleContent>
    </Collapsible>
  );
}
