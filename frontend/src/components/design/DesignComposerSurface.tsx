import { Send } from "lucide-react";
import { useState } from "react";

import { Button } from "@/components/ui/button";
import { getContextConfig } from "@/lib/chat-context-registry";
import type { DesignSystem } from "./designSystems";
import { DesignStartComposer } from "./DesignStartComposer";

interface DesignComposerSurfaceProps {
  selectedDesignSystem: DesignSystem | null;
  onNewDesignSystem: () => void;
  onImportDesignSystem: () => void;
}

export function DesignComposerSurface({
  selectedDesignSystem,
  onNewDesignSystem,
  onImportDesignSystem,
}: DesignComposerSurfaceProps) {
  const [draft, setDraft] = useState("");
  const [messages, setMessages] = useState<string[]>([]);
  const placeholder = getContextConfig("design").placeholder;

  if (!selectedDesignSystem) {
    return (
      <DesignStartComposer
        onNewDesignSystem={onNewDesignSystem}
        onImportDesignSystem={onImportDesignSystem}
      />
    );
  }

  const sendDraft = () => {
    const trimmed = draft.trim();
    if (!trimmed) {
      return;
    }
    setMessages((current) => [...current, trimmed]);
    setDraft("");
  };

  return (
    <section className="h-full flex flex-col min-w-0" data-testid="design-composer-surface">
      <header className="h-12 px-4 border-b flex items-center shrink-0" style={{ borderColor: "var(--overlay-faint)" }}>
        <div className="min-w-0">
          <div className="text-[13px] font-semibold truncate" style={{ color: "var(--text-primary)" }}>
            {selectedDesignSystem.name}
          </div>
          <div className="text-[11px]" style={{ color: "var(--text-muted)" }}>
            design context
          </div>
        </div>
      </header>
      <div className="flex-1 min-h-0 overflow-y-auto px-4 py-4 space-y-3">
        <div className="text-[13px] leading-6" style={{ color: "var(--text-secondary)" }}>
          Source summary and styleguide review are ready for this design system.
        </div>
        {messages.map((message, index) => (
          <div
            key={`${message}-${index}`}
            className="ml-auto max-w-[80%] rounded-lg px-3 py-2 text-[13px]"
            style={{
              background: "var(--accent-muted)",
              color: "var(--text-primary)",
            }}
          >
            {message}
          </div>
        ))}
      </div>
      <div className="px-4 pb-4 pt-3 border-t shrink-0" style={{ borderColor: "var(--overlay-faint)" }}>
        <div
          className="flex items-end gap-2 rounded-lg border p-2"
          style={{
            borderColor: "var(--overlay-weak)",
            background: "var(--bg-surface)",
          }}
        >
          <textarea
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
            placeholder={placeholder}
            className="min-h-10 max-h-32 flex-1 resize-none bg-transparent text-[13px] leading-5 outline-none"
            style={{ color: "var(--text-primary)" }}
            data-testid="design-composer-input"
          />
          <Button
            type="button"
            size="sm"
            className="h-8 w-8 p-0"
            onClick={sendDraft}
            aria-label="Send design message"
            data-testid="design-composer-submit"
          >
            <Send className="w-4 h-4" />
          </Button>
        </div>
      </div>
    </section>
  );
}
