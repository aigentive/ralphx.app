/**
 * StepItem component
 *
 * Displays a single task step with status icon, title, description, and completion note.
 * Visual styling adapts based on step status (in_progress, completed, skipped, failed).
 */

import { Circle, Loader2, CheckCircle2, MinusCircle, XCircle, Trash2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import type { TaskStep, TaskStepStatus } from '@/types/task-step';

interface StepItemProps {
  step: TaskStep;
  index: number;
  editable?: boolean;
  onUpdate?: (step: TaskStep) => void;
  onDelete?: (stepId: string) => void;
}

/**
 * Render the appropriate status icon
 */
function StatusIcon({ status, className }: { status: TaskStepStatus; className: string }) {
  switch (status) {
    case 'pending':
      return <Circle className={className} />;
    case 'in_progress':
      return <Loader2 className={`${className} animate-spin`} />;
    case 'completed':
      return <CheckCircle2 className={className} />;
    case 'skipped':
      return <MinusCircle className={className} />;
    case 'failed':
      return <XCircle className={className} />;
    case 'cancelled':
      return <XCircle className={className} />;
  }
}

/**
 * Get color classes for step status
 */
function getStatusColor(status: TaskStepStatus): string {
  switch (status) {
    case 'pending':
      return 'text-text-muted';
    case 'in_progress':
      return 'text-accent-primary';
    case 'completed':
      return 'text-status-success';
    case 'skipped':
      return 'text-text-muted';
    case 'failed':
      return 'text-status-error';
    case 'cancelled':
      return 'text-text-muted';
  }
}

/**
 * Get container classes based on step status
 */
function getContainerClasses(status: TaskStepStatus): string {
  const base = 'flex items-start gap-3 p-3 rounded-lg transition-all';

  switch (status) {
    case 'in_progress':
      return `${base} border-2 border-accent-primary bg-accent-muted`;
    case 'completed':
      return `${base} opacity-75`;
    case 'skipped':
      return `${base} opacity-50`;
    case 'failed':
      return `${base} border border-status-error bg-status-error/5`;
    default:
      return `${base} border border-border-default`;
  }
}

/**
 * StepItem Component
 *
 * Renders a single step in a task's step list with appropriate visual styling
 * based on its status. Supports editing and deletion when editable=true.
 *
 * @example
 * ```tsx
 * <StepItem
 *   step={step}
 *   index={0}
 *   editable={true}
 *   onDelete={(id) => handleDelete(id)}
 * />
 * ```
 */
export function StepItem({ step, index, editable = false, onDelete }: StepItemProps) {
  const iconColor = getStatusColor(step.status);
  const containerClasses = getContainerClasses(step.status);
  const isSkipped = step.status === 'skipped';

  return (
    <div className={containerClasses}>
      {/* Status Icon */}
      <div className="flex-shrink-0 mt-0.5">
        <StatusIcon status={step.status} className={`h-5 w-5 ${iconColor}`} />
      </div>

      {/* Content */}
      <div className="flex-1 min-w-0">
        {/* Step number and title */}
        <div className={`flex items-baseline gap-2 ${isSkipped ? 'line-through' : ''}`}>
          <span className="text-sm font-medium text-text-secondary">
            {index + 1}.
          </span>
          <h4 className="text-sm font-medium text-text-primary">
            {step.title}
          </h4>
        </div>

        {/* Description */}
        {step.description && (
          <p className={`mt-1 text-sm text-text-secondary ${isSkipped ? 'line-through' : ''}`}>
            {step.description}
          </p>
        )}

        {/* Completion note (shown for completed, skipped, failed) */}
        {step.completionNote && (
          <div className="mt-2 px-3 py-2 bg-bg-surface rounded border border-border-subtle">
            <p className="text-xs text-text-muted italic">
              {step.completionNote}
            </p>
          </div>
        )}
      </div>

      {/* Delete button (only for editable pending steps) */}
      {editable && step.status === 'pending' && onDelete && (
        <Button
          variant="ghost"
          size="sm"
          onClick={() => onDelete(step.id)}
          className="flex-shrink-0 h-8 w-8 p-0"
          aria-label="Delete step"
        >
          <Trash2 className="h-4 w-4" />
        </Button>
      )}
    </div>
  );
}
