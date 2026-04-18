import { useEffect, useState } from "react";
import { X } from "lucide-react";

import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogTitle,
} from "@/components/ui/dialog";
import { useUiStore } from "@/stores/uiStore";
import type { ProjectSettings } from "@/types/settings";

import {
  SETTINGS_GROUPS,
  SETTINGS_SECTIONS,
  type SettingsSectionId,
} from "./settings-registry";
import { loadActiveSection, saveActiveSection } from "./settings-ui-state";
import { AccessibilitySection } from "./AccessibilitySection";
import { ApiKeysSection } from "./ApiKeysSection";
import { ExternalMcpSettingsPanel } from "./ExternalMcpSettingsPanel";
import { RepositorySettingsSection } from "./RepositorySettingsSection";

import {
  ExecutionHarnessSection,
  IdeationHarnessSection,
} from "./IdeationHarnessSection";
import { IdeationSettingsPanel } from "./IdeationSettingsPanel";
import { ProjectAnalysisSection } from "./ProjectAnalysisSection";
import ExecutionSection from "./sections/ExecutionSection";
import GlobalExecutionSection from "./sections/GlobalExecutionSection";
import ReviewPolicySection from "./sections/ReviewPolicySection";

export interface SettingsDialogProps {
  executionSettings: ProjectSettings | null;
  isLoadingSettings: boolean;
  isSavingSettings: boolean;
  settingsError: string | null;
  onSettingsChange: (settings: ProjectSettings) => void;
}

export default function SettingsDialog({
  executionSettings,
  isLoadingSettings,
  isSavingSettings,
  settingsError,
  onSettingsChange,
}: SettingsDialogProps) {
  const activeModal = useUiStore((s) => s.activeModal);
  const modalContext = useUiStore((s) => s.modalContext);
  const closeModal = useUiStore((s) => s.closeModal);

  const isOpen = activeModal === "settings";

  const [activeSection, setActiveSectionState] = useState<SettingsSectionId>(
    () => loadActiveSection() ?? "execution",
  );

  const setActiveSection = (section: SettingsSectionId) => {
    setActiveSectionState(section);
    saveActiveSection(section);
  };

  useEffect(() => {
    if (isOpen) {
      const section = modalContext?.["section"] as SettingsSectionId | undefined;
      if (section) {
        setActiveSection(section);
      }
    }
  }, [isOpen, modalContext]);

  const activeSectionMeta = SETTINGS_SECTIONS.find((s) => s.id === activeSection);

  const disabled = isLoadingSettings || isSavingSettings;

  const sectionRenderers = {
    execution: () =>
      executionSettings ? (
        <ExecutionSection
          settings={executionSettings.execution}
          onChange={(changes) =>
            onSettingsChange({ ...executionSettings, execution: { ...executionSettings.execution, ...changes } })
          }
          disabled={disabled}
        />
      ) : null,
    "execution-harnesses": () => <ExecutionHarnessSection />,
    "global-execution": () => <GlobalExecutionSection />,
    review: () => <ReviewPolicySection />,
    repository: () => <RepositorySettingsSection />,
    "project-analysis": () => <ProjectAnalysisSection />,
    "ideation-workflow": () => <IdeationSettingsPanel />,
    "ideation-harnesses": () => <IdeationHarnessSection />,
    "api-keys": () => <ApiKeysSection />,
    "external-mcp": () => <ExternalMcpSettingsPanel />,
    accessibility: () => <AccessibilitySection />,
  };

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && closeModal()}>
      <DialogContent
        data-testid="settings-dialog"
        className="p-0 gap-0 overflow-hidden flex flex-col max-w-[95vw] w-[95vw] h-[95vh] bg-[var(--bg-elevated)] border border-[var(--border-subtle)]"
        hideCloseButton={true}
      >
        <DialogTitle className="sr-only">Settings</DialogTitle>
        <DialogDescription className="sr-only">
          Configure execution, ideation, workspace, and access settings.
        </DialogDescription>
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b border-[var(--border-subtle)] bg-[var(--bg-elevated)] shrink-0">
          <div className="flex items-center gap-2">
            <span className="text-sm font-semibold text-[var(--text-primary)]">
              Settings
            </span>
            {activeSectionMeta && (
              <>
                <span className="text-[var(--text-secondary)] text-sm">/</span>
                <span className="text-sm text-[var(--text-secondary)]">
                  {activeSectionMeta.label}
                </span>
              </>
            )}
          </div>
          <button
            type="button"
            onClick={closeModal}
            className="rounded-md p-1.5 text-[var(--text-secondary)] transition-colors hover:bg-[var(--bg-hover)] hover:text-[var(--text-primary)] focus:outline-none focus-visible:outline-none focus-visible:ring-0"
            aria-label="Close settings"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Body */}
        <div className="flex flex-1 overflow-hidden">
          {/* Left rail — hidden below lg breakpoint */}
          <nav className="hidden lg:flex w-[280px] flex-shrink-0 flex-col overflow-y-auto border-r border-[var(--border-subtle)] py-3">
            {SETTINGS_GROUPS.map((group) => {
              const groupSections = SETTINGS_SECTIONS.filter(
                (s) => s.groupId === group.id
              );
              return (
                <div key={group.id} className="mb-4">
                  <p className="px-4 py-1 text-[11px] font-semibold uppercase tracking-wider text-[var(--text-secondary)] opacity-60">
                    {group.label}
                  </p>
                  {groupSections.map((section) => {
                    const isActive = section.id === activeSection;
                    return (
                      <div
                        key={section.id}
                        role="button"
                        tabIndex={0}
                        onClick={() => setActiveSection(section.id)}
                        onKeyDown={(e) => {
                          if (e.key === "Enter" || e.key === " ") {
                            e.preventDefault();
                            setActiveSection(section.id);
                          }
                        }}
                        aria-current={isActive ? "page" : undefined}
                        className={`mx-2 flex min-h-[36px] items-center rounded-md px-3 py-1.5 text-sm cursor-pointer transition-colors ${
                          isActive
                            ? "bg-[var(--nav-active-bg)] text-[var(--nav-active-text)] font-semibold"
                            : "text-[var(--text-primary)] hover:bg-[var(--bg-hover)]"
                        }`}
                      >
                        <span className="block truncate">{section.label}</span>
                      </div>
                    );
                  })}
                </div>
              );
            })}
          </nav>

          {/* Mobile section selector — visible below lg breakpoint */}
          <div className="block lg:hidden w-full px-4 py-2 border-b border-[var(--border-subtle)] shrink-0">
            <select
              value={activeSection}
              onChange={(e) => setActiveSection(e.target.value as SettingsSectionId)}
              className="w-full rounded-md px-3 py-1.5 text-sm text-[var(--text-primary)] bg-[var(--bg-surface)] border border-[var(--border-subtle)] focus:outline-none"
            >
              {SETTINGS_GROUPS.map((group) => {
                const groupSections = SETTINGS_SECTIONS.filter(
                  (s) => s.groupId === group.id
                );
                return (
                  <optgroup key={group.id} label={group.label}>
                    {groupSections.map((section) => (
                      <option key={section.id} value={section.id}>
                        {section.label}
                      </option>
                    ))}
                  </optgroup>
                );
              })}
            </select>
          </div>

          {/* Right pane */}
          <div className="flex-1 overflow-hidden flex flex-col">
            <ScrollArea className="flex-1">
              <div className="p-6">
                {sectionRenderers[activeSection]?.() ?? (
                  <p className="text-sm text-[var(--text-secondary)]">
                    Section not found.
                  </p>
                )}
              </div>
            </ScrollArea>
          </div>
        </div>

        {settingsError && (
          <div className="px-4 py-2 text-sm shrink-0 border-t border-[var(--border-subtle)] text-[var(--status-error)]">
            {settingsError}
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
