import { Palette, Plus, Upload } from "lucide-react";

import { Button } from "@/components/ui/button";

interface DesignStartComposerProps {
  onNewDesignSystem: () => void;
  onImportDesignSystem: () => void;
}

export function DesignStartComposer({
  onNewDesignSystem,
  onImportDesignSystem,
}: DesignStartComposerProps) {
  return (
    <div className="h-full flex items-center justify-center px-6" data-testid="design-empty-state">
      <div className="max-w-sm w-full text-center">
        <div
          className="mx-auto h-12 w-12 rounded-lg flex items-center justify-center"
          style={{
            background: "var(--accent-muted)",
            color: "var(--accent-primary)",
          }}
        >
          <Palette className="w-5 h-5" />
        </div>
        <h2 className="mt-4 text-[18px] font-semibold" style={{ color: "var(--text-primary)" }}>
          No design system selected
        </h2>
        <div className="mt-5 flex items-center justify-center gap-2">
          <Button type="button" size="sm" onClick={onNewDesignSystem} className="gap-2">
            <Plus className="w-4 h-4" />
            New design system
          </Button>
          <Button type="button" variant="outline" size="sm" onClick={onImportDesignSystem} className="gap-2">
            <Upload className="w-4 h-4" />
            Import
          </Button>
        </div>
      </div>
    </div>
  );
}
