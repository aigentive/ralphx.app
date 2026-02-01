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
  /** Hide completion notes (useful for historical views before execution) */
  hideCompletionNote?: boolean;
  onUpdate?: (step: TaskStep) => void;
  onDelete?: (stepId: string) => void;
}

/**
 * Render the appropriate status icon
 */
function StatusIcon({ status, className, style }: { status: TaskStepStatus; className: string; style?: React.CSSProperties }) {
  switch (status) {
    case 'pending':
      return <Circle className={className} style={style} />;
    case 'in_progress':
      return <Loader2 className={`${className} animate-spin`} style={style} />;
    case 'completed':
      return <CheckCircle2 className={className} style={style} />;
    case 'skipped':
      return <MinusCircle className={className} style={style} />;
    case 'failed':
      return <XCircle className={className} style={style} />;
    case 'cancelled':
      return <XCircle className={className} style={style} />;
  }
}

/**
 * Get color for step status icon (Tahoe HSL colors)
 */
function getStatusColor(status: TaskStepStatus): string {
  switch (status) {
    case 'pending':
      return 'hsl(220 10% 40%)';
    case 'in_progress':
      return 'hsl(14 100% 60%)'; // accent orange
    case 'completed':
      return 'hsl(142 70% 45%)'; // green
    case 'skipped':
      return 'hsl(220 10% 40%)';
    case 'failed':
      return 'hsl(0 70% 55%)'; // red
    case 'cancelled':
      return 'hsl(220 10% 40%)';
  }
}

/**
 * Get container styles based on step status (Tahoe design - minimal, no borders)
 */
function getContainerStyles(status: TaskStepStatus): React.CSSProperties {
  const base: React.CSSProperties = {
    display: 'flex',
    alignItems: 'flex-start',
    gap: '10px',
    padding: '10px 12px',
    borderRadius: '6px',
    transition: 'all 150ms ease',
  };

  switch (status) {
    case 'in_progress':
      return {
        ...base,
        backgroundColor: 'hsla(14 100% 60% / 0.06)',
      };
    case 'completed':
      return {
        ...base,
        backgroundColor: 'transparent',
      };
    case 'skipped':
      return {
        ...base,
        backgroundColor: 'transparent',
        opacity: 0.5,
      };
    case 'failed':
      return {
        ...base,
        backgroundColor: 'hsla(0 70% 55% / 0.06)',
      };
    default:
      return {
        ...base,
        backgroundColor: 'transparent',
      };
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
export function StepItem({ step, index, editable = false, hideCompletionNote = false, onDelete }: StepItemProps) {
  const iconColor = getStatusColor(step.status);
  const containerStyles = getContainerStyles(step.status);
  const isSkipped = step.status === 'skipped';

  return (
    <div style={containerStyles}>
      {/* Status Icon */}
      <div className="flex-shrink-0 mt-0.5">
        <StatusIcon status={step.status} className="h-4 w-4" style={{ color: iconColor }} />
      </div>

      {/* Content */}
      <div className="flex-1 min-w-0">
        {/* Step number and title */}
        <div
          className="flex items-baseline gap-1"
          style={{ textDecoration: isSkipped ? 'line-through' : 'none' }}
        >
          <span
            style={{
              fontSize: '13px',
              fontWeight: 400,
              color: 'hsl(220 10% 45%)',
            }}
          >
            Step {index + 1}:
          </span>
          <span
            style={{
              fontSize: '13px',
              fontWeight: 400,
              color: 'hsl(220 10% 80%)',
              marginLeft: '4px',
            }}
          >
            {step.title}
          </span>
        </div>

        {/* Description */}
        {step.description && (
          <p
            style={{
              marginTop: '2px',
              fontSize: '12px',
              color: 'hsl(220 10% 50%)',
              lineHeight: 1.5,
              textDecoration: isSkipped ? 'line-through' : 'none',
            }}
          >
            {step.description}
          </p>
        )}

        {/* Completion note (shown for completed, skipped, failed - hidden in historical views) */}
        {step.completionNote && !hideCompletionNote && (
          <p
            style={{
              marginTop: '4px',
              fontSize: '11px',
              color: 'hsl(220 10% 45%)',
              fontStyle: 'italic',
              lineHeight: 1.5,
            }}
          >
            {step.completionNote}
          </p>
        )}
      </div>

      {/* Delete button (only for editable pending steps) */}
      {editable && step.status === 'pending' && onDelete && (
        <Button
          variant="ghost"
          size="sm"
          onClick={() => onDelete(step.id)}
          className="flex-shrink-0 h-7 w-7 p-0"
          aria-label="Delete step"
        >
          <Trash2 className="h-3.5 w-3.5" style={{ color: 'hsl(220 10% 50%)' }} />
        </Button>
      )}
    </div>
  );
}
