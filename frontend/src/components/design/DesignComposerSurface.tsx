import { IntegratedChatPanel } from "@/components/Chat/IntegratedChatPanel";
import { buildStoreKey } from "@/lib/chat-context-registry";
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
  if (!selectedDesignSystem) {
    return (
      <DesignStartComposer
        onNewDesignSystem={onNewDesignSystem}
        onImportDesignSystem={onImportDesignSystem}
      />
    );
  }

  const sourceLabel = selectedDesignSystem.sourceCount === 1 ? "source" : "sources";
  const statusLabel = selectedDesignSystem.status.replace("_", " ");

  return (
    <section
      className="h-full min-w-0"
      data-testid="design-composer-surface"
      data-design-system-id={selectedDesignSystem.id}
      data-conversation-id={selectedDesignSystem.conversationId ?? ""}
    >
      <IntegratedChatPanel
        key={selectedDesignSystem.id}
        projectId={selectedDesignSystem.primaryProjectId}
        designSystemId={selectedDesignSystem.id}
        conversationIdOverride={selectedDesignSystem.conversationId ?? undefined}
        selectedTaskIdOverride={null}
        storeContextKeyOverride={buildStoreKey("design", selectedDesignSystem.id)}
        agentProcessContextIdOverride={selectedDesignSystem.id}
        {...(selectedDesignSystem.conversationId
          ? { sendOptions: { conversationId: selectedDesignSystem.conversationId } }
          : {})}
        hideHeaderSessionControls
        hideSessionToolbar
        autoFocusInput={false}
        showHelperTextAlways={false}
        headerContent={
          <div className="min-w-0">
            <div className="text-[13px] font-semibold truncate" style={{ color: "var(--text-primary)" }}>
              {selectedDesignSystem.name}
            </div>
            <div
              className="truncate text-[11px]"
              style={{ color: "var(--text-muted)" }}
              data-testid="design-chat-context"
            >
              Design steward · {statusLabel} · {selectedDesignSystem.sourceCount} {sourceLabel}
            </div>
          </div>
        }
      />
    </section>
  );
}
