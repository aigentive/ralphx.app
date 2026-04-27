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

      expect(screen.getByText('Step 1:')).toBeInTheDocument();
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
      expect(screen.getByText('Step 6:')).toBeInTheDocument();
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
    it('should keep in-progress row neutral and tint only the icon', () => {
      const inProgressStep = { ...baseStep, status: 'in_progress' as const };
      const { container } = render(<StepItem step={inProgressStep} index={0} />);
      const stepContainer = container.firstChild as HTMLElement;
      expect(stepContainer).toHaveStyle({
        backgroundColor: 'var(--overlay-faint)',
      });
      const icon = container.querySelector('svg');
      expect(icon).toHaveStyle({
        color: 'color-mix(in srgb, var(--accent-primary) 82%, var(--text-muted))',
      });
    });

    it('should apply completed styles with opacity', () => {
      const completedStep = { ...baseStep, status: 'completed' as const };
      const { container } = render(<StepItem step={completedStep} index={0} />);
      const stepContainer = container.firstChild as HTMLElement;
      expect(stepContainer.getAttribute('style')).toContain('background-color: transparent');
    });

    it('should apply skipped styles with opacity and line-through', () => {
      const skippedStep = { ...baseStep, status: 'skipped' as const };
      const { container } = render(<StepItem step={skippedStep} index={0} />);
      const stepContainer = container.firstChild as HTMLElement;
      expect(stepContainer).toHaveStyle({ opacity: '0.5' });
      // Check for line-through on title
      const titleElement = screen.getByText('Implement authentication').parentElement;
      expect(titleElement).toHaveStyle({ textDecoration: 'line-through' });
    });

    it('should keep failed row neutral and tint only the icon', () => {
      const failedStep = { ...baseStep, status: 'failed' as const };
      const { container } = render(<StepItem step={failedStep} index={0} />);
      const stepContainer = container.firstChild as HTMLElement;
      expect(stepContainer).toHaveStyle({
        backgroundColor: 'var(--overlay-faint)',
      });
      const icon = container.querySelector('svg');
      expect(icon).toHaveStyle({
        color: 'color-mix(in srgb, var(--status-error) 78%, var(--text-muted))',
      });
    });
  });

  describe('Editable Mode', () => {
    it('should not show skip button when not editable', () => {
      render(<StepItem step={baseStep} index={0} editable={false} />);
      expect(screen.queryByLabelText('Skip step')).not.toBeInTheDocument();
    });

    it('should show skip button when editable and pending', async () => {
      const onSkip = vi.fn();
      render(<StepItem step={baseStep} index={0} editable={true} onSkip={onSkip} />);

      const skipButton = screen.getByLabelText('Skip step');
      expect(skipButton).toBeInTheDocument();
    });

    it('should not show skip button for non-pending steps', () => {
      const completedStep = { ...baseStep, status: 'completed' as const };
      const onSkip = vi.fn();
      render(<StepItem step={completedStep} index={0} editable={true} onSkip={onSkip} />);

      expect(screen.queryByLabelText('Skip step')).not.toBeInTheDocument();
    });

    it('should call onSkip when skip button is clicked', async () => {
      const user = userEvent.setup();
      const onSkip = vi.fn();
      render(<StepItem step={baseStep} index={0} editable={true} onSkip={onSkip} />);

      const skipButton = screen.getByLabelText('Skip step');
      await user.click(skipButton);

      expect(onSkip).toHaveBeenCalledWith('step-1');
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
