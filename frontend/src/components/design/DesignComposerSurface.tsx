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

  return (
    <section className="h-full min-w-0" data-testid="design-composer-surface">
      <IntegratedChatPanel
        projectId={selectedDesignSystem.primaryProjectId}
        designSystemId={selectedDesignSystem.id}
        conversationIdOverride={selectedDesignSystem.conversationId ?? undefined}
        selectedTaskIdOverride={null}
        storeContextKeyOverride={buildStoreKey("design", selectedDesignSystem.id)}
        agentProcessContextIdOverride={selectedDesignSystem.id}
        hideHeaderSessionControls
        hideSessionToolbar
        autoFocusInput={false}
        showHelperTextAlways={false}
        headerContent={
          <div className="min-w-0">
            <div className="text-[13px] font-semibold truncate" style={{ color: "var(--text-primary)" }}>
              {selectedDesignSystem.name}
            </div>
            <div className="text-[11px]" style={{ color: "var(--text-muted)" }}>
              {selectedDesignSystem.status.replace("_", " ")} / {selectedDesignSystem.sourceCount} sources
            </div>
          </div>
        }
        renderComposer={() => (
          <div
            className="rounded-lg border px-3 py-2 text-[12px]"
            style={{
              borderColor: "var(--overlay-weak)",
              color: "var(--text-muted)",
              background: "var(--bg-surface)",
            }}
            data-testid="design-chat-runtime-pending"
          >
            Review notes appear here while this draft is being prepared.
          </div>
        )}
      />
    </section>
  );
}
