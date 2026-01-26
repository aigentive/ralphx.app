/**
 * StepList component tests
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { StepList } from './StepList';
import * as useTaskStepsModule from '@/hooks/useTaskSteps';
import * as useStepMutationsModule from '@/hooks/useStepMutations';
import type { TaskStep } from '@/types/task-step';

// Mock the hooks
vi.mock('@/hooks/useTaskSteps');
vi.mock('@/hooks/useStepMutations');

const mockUseTaskSteps = vi.spyOn(useTaskStepsModule, 'useTaskSteps');
const mockUseStepMutations = vi.spyOn(useStepMutationsModule, 'useStepMutations');

const queryClient = new QueryClient({
  defaultOptions: {
    queries: { retry: false },
  },
});

function renderWithProviders(ui: React.ReactElement) {
  return render(
    <QueryClientProvider client={queryClient}>
      {ui}
    </QueryClientProvider>
  );
}

const mockSteps: TaskStep[] = [
  {
    id: 'step-1',
    taskId: 'task-1',
    title: 'First step',
    description: 'Do this first',
    status: 'completed',
    sortOrder: 0,
    dependsOn: null,
    createdBy: 'user',
    completionNote: 'Done!',
    createdAt: '2024-01-01T00:00:00Z',
    updatedAt: '2024-01-01T00:00:00Z',
    startedAt: '2024-01-01T00:00:00Z',
    completedAt: '2024-01-01T00:00:00Z',
  },
  {
    id: 'step-2',
    taskId: 'task-1',
    title: 'Second step',
    description: null,
    status: 'in_progress',
    sortOrder: 1,
    dependsOn: null,
    createdBy: 'user',
    completionNote: null,
    createdAt: '2024-01-01T00:00:00Z',
    updatedAt: '2024-01-01T00:00:00Z',
    startedAt: '2024-01-01T00:00:00Z',
    completedAt: null,
  },
  {
    id: 'step-3',
    taskId: 'task-1',
    title: 'Third step',
    description: null,
    status: 'pending',
    sortOrder: 2,
    dependsOn: null,
    createdBy: 'user',
    completionNote: null,
    createdAt: '2024-01-01T00:00:00Z',
    updatedAt: '2024-01-01T00:00:00Z',
    startedAt: null,
    completedAt: null,
  },
];

describe('StepList', () => {
  const mockDelete = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    mockUseStepMutations.mockReturnValue({
      create: { mutate: vi.fn(), isPending: false },
      update: { mutate: vi.fn(), isPending: false },
      delete: { mutate: mockDelete, isPending: false },
      reorder: { mutate: vi.fn(), isPending: false },
      isCreating: false,
      isUpdating: false,
      isDeleting: false,
      isReordering: false,
    } as ReturnType<typeof useStepMutationsModule.useStepMutations>);
  });

  it('renders loading skeleton when loading', () => {
    mockUseTaskSteps.mockReturnValue({
      data: undefined,
      isLoading: true,
      isError: false,
    } as ReturnType<typeof useTaskStepsModule.useTaskSteps>);

    renderWithProviders(<StepList taskId="task-1" />);

    const skeletons = document.querySelectorAll('.animate-pulse');
    expect(skeletons.length).toBeGreaterThan(0);
  });

  it('renders error state when error occurs', () => {
    mockUseTaskSteps.mockReturnValue({
      data: undefined,
      isLoading: false,
      isError: true,
    } as ReturnType<typeof useTaskStepsModule.useTaskSteps>);

    renderWithProviders(<StepList taskId="task-1" />);

    expect(screen.getByText('Failed to load steps')).toBeInTheDocument();
  });

  it('renders empty state when no steps exist', () => {
    mockUseTaskSteps.mockReturnValue({
      data: [],
      isLoading: false,
      isError: false,
    } as ReturnType<typeof useTaskStepsModule.useTaskSteps>);

    renderWithProviders(<StepList taskId="task-1" />);

    expect(screen.getByText('No steps yet')).toBeInTheDocument();
  });

  it('renders empty state with editable message when editable=true', () => {
    mockUseTaskSteps.mockReturnValue({
      data: [],
      isLoading: false,
      isError: false,
    } as ReturnType<typeof useTaskStepsModule.useTaskSteps>);

    renderWithProviders(<StepList taskId="task-1" editable={true} />);

    expect(screen.getByText('Add steps to track progress on this task')).toBeInTheDocument();
  });

  it('renders steps when data is available', () => {
    mockUseTaskSteps.mockReturnValue({
      data: mockSteps,
      isLoading: false,
      isError: false,
    } as ReturnType<typeof useTaskStepsModule.useTaskSteps>);

    renderWithProviders(<StepList taskId="task-1" />);

    expect(screen.getByText('First step')).toBeInTheDocument();
    expect(screen.getByText('Second step')).toBeInTheDocument();
    expect(screen.getByText('Third step')).toBeInTheDocument();
  });

  it('passes editable prop to StepItem components', () => {
    mockUseTaskSteps.mockReturnValue({
      data: mockSteps,
      isLoading: false,
      isError: false,
    } as ReturnType<typeof useTaskStepsModule.useTaskSteps>);

    renderWithProviders(<StepList taskId="task-1" editable={true} />);

    // When editable=true and step is pending, delete button should be rendered
    // Step 3 is pending, so it should have a delete button
    const deleteButtons = screen.getAllByLabelText('Delete step');
    expect(deleteButtons.length).toBeGreaterThan(0);
  });

  it('does not render delete buttons when editable=false', () => {
    mockUseTaskSteps.mockReturnValue({
      data: mockSteps,
      isLoading: false,
      isError: false,
    } as ReturnType<typeof useTaskStepsModule.useTaskSteps>);

    renderWithProviders(<StepList taskId="task-1" editable={false} />);

    const deleteButtons = screen.queryAllByLabelText('Delete step');
    expect(deleteButtons.length).toBe(0);
  });
});
