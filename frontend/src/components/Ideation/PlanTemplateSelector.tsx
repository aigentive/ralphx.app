/**
 * PlanTemplateSelector - Template picker for plan artifacts
 *
 * Features:
 * - Fetches plan templates from active methodology
 * - Shows dropdown only when templates are available
 * - On select: pre-populates plan content with template
 * - Hidden when no templates available (blank plan by default)
 */

import { useCallback, useEffect, useState } from "react";
import { FileText } from "lucide-react";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Label } from "@/components/ui/label";
import { getActiveMethodology } from "@/lib/api/methodologies";

// ============================================================================
// Types
// ============================================================================

/**
 * Plan template from methodology
 * Matches MethodologyPlanTemplate in Rust backend
 */
export interface PlanTemplate {
  /** Unique identifier for the template */
  id: string;
  /** Display name for the template */
  name: string;
  /** Description of when to use this template */
  description: string;
  /** Markdown template content with {{placeholders}} */
  templateContent: string;
}

export interface PlanTemplateSelectorProps {
  /** Callback when template is selected - receives template content */
  onTemplateSelect: (templateContent: string) => void;
  /** Whether selector is disabled */
  disabled?: boolean;
}

// ============================================================================
// Component
// ============================================================================

export function PlanTemplateSelector({
  onTemplateSelect,
  disabled = false,
}: PlanTemplateSelectorProps) {
  const [templates, setTemplates] = useState<PlanTemplate[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [selectedTemplateId, setSelectedTemplateId] = useState<string>("");

  // Fetch templates from active methodology
  useEffect(() => {
    async function fetchTemplates() {
      setIsLoading(true);
      try {
        const methodology = await getActiveMethodology();

        // If no methodology active, return empty array
        if (!methodology) {
          setTemplates([]);
          return;
        }

        // Convert from backend format to frontend format
        // Backend uses snake_case, we need camelCase
        // Backend response includes plan_templates field (not in MethodologyResponse type yet)
        const backendTemplates = (methodology as { plan_templates?: Array<{
          id: string;
          name: string;
          description: string;
          template_content: string;
        }> }).plan_templates;

        const planTemplates: PlanTemplate[] =
          backendTemplates?.map((t) => ({
            id: t.id,
            name: t.name,
            description: t.description,
            templateContent: t.template_content,
          })) || [];

        setTemplates(planTemplates);
      } catch (error) {
        console.error("Failed to fetch plan templates:", error);
        setTemplates([]);
      } finally {
        setIsLoading(false);
      }
    }

    fetchTemplates();
  }, []);

  // Handle template selection
  const handleTemplateChange = useCallback(
    (templateId: string) => {
      setSelectedTemplateId(templateId);

      // Find the template and pass its content to parent
      const template = templates.find((t) => t.id === templateId);
      if (template) {
        onTemplateSelect(template.templateContent);
      }
    },
    [templates, onTemplateSelect]
  );

  // Don't render anything if loading or no templates available
  if (isLoading || templates.length === 0) {
    return null;
  }

  return (
    <div className="space-y-2">
      <Label htmlFor="plan-template" className="text-sm text-[var(--text-secondary)]">
        <FileText className="h-4 w-4 inline mr-1.5" />
        Start from template
      </Label>

      <Select
        value={selectedTemplateId}
        onValueChange={handleTemplateChange}
        disabled={disabled}
      >
        <SelectTrigger
          id="plan-template"
          className="w-full border-[var(--border-primary)] bg-[var(--bg-base)] text-[var(--text-primary)] focus:ring-[var(--accent-primary)]"
        >
          <SelectValue placeholder="Select a template (optional)" />
        </SelectTrigger>

        <SelectContent className="bg-[var(--bg-elevated)] border-[var(--border-primary)]">
          {templates.map((template) => (
            <SelectItem
              key={template.id}
              value={template.id}
              className="text-[var(--text-primary)] hover:bg-[var(--bg-base)] cursor-pointer"
            >
              <div className="flex flex-col items-start gap-1">
                <span className="font-medium">{template.name}</span>
                <span className="text-xs text-[var(--text-tertiary)]">
                  {template.description}
                </span>
              </div>
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
}
