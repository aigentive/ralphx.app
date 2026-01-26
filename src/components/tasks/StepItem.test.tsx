/**
 * StepItem component tests
 */

import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { userEvent } from '@testing-library/user-event';
import { StepItem } from './StepItem';
import { TaskStep } from '@/types/task-step';

describe('StepItem', () => {
  const baseStep: TaskStep = {
    id: 'step-1',
    taskId: 'task-1',
    title: 'Implement authentication',
    description: 'Add OAuth provider',
    status: 'pending',
    sortOrder: 0,
    dependsOn: null,
    createdBy: 'user',
    completionNote: null,
    createdAt: '2024-01-01T00:00:00+00:00',
    updatedAt: '2024-01-01T00:00:00+00:00',
    startedAt: null,
    completedAt: null,
  };

  describe('Rendering', () => {
    it('should render step with title and description', () => {
      render(<StepItem step={baseStep} index={0} />);

      expect(screen.getByText('1.')).toBeInTheDocument();
      expect(screen.getByText('Implement authentication')).toBeInTheDocument();
      expect(screen.getByText('Add OAuth provider')).toBeInTheDocument();
    });

    it('should render step without description', () => {
      const stepNoDesc = { ...baseStep, description: null };
      render(<StepItem step={stepNoDesc} index={0} />);

      expect(screen.getByText('Implement authentication')).toBeInTheDocument();
      expect(screen.queryByText('Add OAuth provider')).not.toBeInTheDocument();
    });

    it('should render completion note when present', () => {
      const stepWithNote = {
        ...baseStep,
        status: 'completed' as const,
        completionNote: 'Successfully implemented',
      };
      render(<StepItem step={stepWithNote} index={0} />);

      expect(screen.getByText('Successfully implemented')).toBeInTheDocument();
    });

    it('should render correct step number based on index', () => {
      render(<StepItem step={baseStep} index={5} />);
      expect(screen.getByText('6.')).toBeInTheDocument();
    });
  });

  describe('Status Icons', () => {
    it('should render Circle icon for pending status', () => {
      const { container } = render(<StepItem step={baseStep} index={0} />);
      // Circle icon should be present
      const icon = container.querySelector('svg');
      expect(icon).toBeInTheDocument();
    });

    it('should render CheckCircle2 icon for completed status', () => {
      const completedStep = { ...baseStep, status: 'completed' as const };
      render(<StepItem step={completedStep} index={0} />);
      // Icon should be present with success color
      expect(screen.getByText('Implement authentication')).toBeInTheDocument();
    });

    it('should render Loader2 icon for in_progress status', () => {
      const inProgressStep = { ...baseStep, status: 'in_progress' as const };
      render(<StepItem step={inProgressStep} index={0} />);
      expect(screen.getByText('Implement authentication')).toBeInTheDocument();
    });

    it('should render MinusCircle icon for skipped status', () => {
      const skippedStep = { ...baseStep, status: 'skipped' as const };
      render(<StepItem step={skippedStep} index={0} />);
      expect(screen.getByText('Implement authentication')).toBeInTheDocument();
    });

    it('should render XCircle icon for failed status', () => {
      const failedStep = { ...baseStep, status: 'failed' as const };
      render(<StepItem step={failedStep} index={0} />);
      expect(screen.getByText('Implement authentication')).toBeInTheDocument();
    });
  });

  describe('Visual Styling', () => {
    it('should apply in_progress styles with border and background', () => {
      const inProgressStep = { ...baseStep, status: 'in_progress' as const };
      const { container } = render(<StepItem step={inProgressStep} index={0} />);
      const stepContainer = container.firstChild as HTMLElement;
      expect(stepContainer.className).toContain('border-accent-primary');
      expect(stepContainer.className).toContain('bg-accent-muted');
    });

    it('should apply completed styles with opacity', () => {
      const completedStep = { ...baseStep, status: 'completed' as const };
      const { container } = render(<StepItem step={completedStep} index={0} />);
      const stepContainer = container.firstChild as HTMLElement;
      expect(stepContainer.className).toContain('opacity-75');
    });

    it('should apply skipped styles with opacity and line-through', () => {
      const skippedStep = { ...baseStep, status: 'skipped' as const };
      const { container } = render(<StepItem step={skippedStep} index={0} />);
      const stepContainer = container.firstChild as HTMLElement;
      expect(stepContainer.className).toContain('opacity-50');
      // Check for line-through on title
      const titleElement = screen.getByText('Implement authentication').parentElement;
      expect(titleElement?.className).toContain('line-through');
    });

    it('should apply failed styles with error border', () => {
      const failedStep = { ...baseStep, status: 'failed' as const };
      const { container } = render(<StepItem step={failedStep} index={0} />);
      const stepContainer = container.firstChild as HTMLElement;
      expect(stepContainer.className).toContain('border-status-error');
    });
  });

  describe('Editable Mode', () => {
    it('should not show delete button when not editable', () => {
      render(<StepItem step={baseStep} index={0} editable={false} />);
      expect(screen.queryByLabelText('Delete step')).not.toBeInTheDocument();
    });

    it('should show delete button when editable and pending', async () => {
      const onDelete = vi.fn();
      render(<StepItem step={baseStep} index={0} editable={true} onDelete={onDelete} />);

      const deleteButton = screen.getByLabelText('Delete step');
      expect(deleteButton).toBeInTheDocument();
    });

    it('should not show delete button for non-pending steps', () => {
      const completedStep = { ...baseStep, status: 'completed' as const };
      const onDelete = vi.fn();
      render(<StepItem step={completedStep} index={0} editable={true} onDelete={onDelete} />);

      expect(screen.queryByLabelText('Delete step')).not.toBeInTheDocument();
    });

    it('should call onDelete when delete button is clicked', async () => {
      const user = userEvent.setup();
      const onDelete = vi.fn();
      render(<StepItem step={baseStep} index={0} editable={true} onDelete={onDelete} />);

      const deleteButton = screen.getByLabelText('Delete step');
      await user.click(deleteButton);

      expect(onDelete).toHaveBeenCalledWith('step-1');
    });
  });

  describe('Edge Cases', () => {
    it('should handle step with all fields populated', () => {
      const fullStep: TaskStep = {
        ...baseStep,
        status: 'completed',
        completionNote: 'Done with tests',
        startedAt: '2024-01-01T10:00:00+00:00',
        completedAt: '2024-01-01T11:00:00+00:00',
      };
      render(<StepItem step={fullStep} index={0} />);

      expect(screen.getByText('Implement authentication')).toBeInTheDocument();
      expect(screen.getByText('Add OAuth provider')).toBeInTheDocument();
      expect(screen.getByText('Done with tests')).toBeInTheDocument();
    });

    it('should handle cancelled status', () => {
      const cancelledStep = { ...baseStep, status: 'cancelled' as const };
      render(<StepItem step={cancelledStep} index={0} />);
      expect(screen.getByText('Implement authentication')).toBeInTheDocument();
    });
  });
});
