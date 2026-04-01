import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { EmptySearchState } from './EmptySearchState';

describe('EmptySearchState', () => {
  const defaultProps = {
    searchQuery: 'test query',
    onCreateTask: vi.fn(),
    onClearSearch: vi.fn(),
    showArchived: false,
  };

  it('renders the search query in heading', () => {
    render(<EmptySearchState {...defaultProps} />);
    expect(
      screen.getByText('No tasks match "test query"')
    ).toBeInTheDocument();
  });

  it('renders the subheading text', () => {
    render(<EmptySearchState {...defaultProps} />);
    expect(screen.getByText('Should this be a task?')).toBeInTheDocument();
  });

  it('renders FileText icon', () => {
    render(<EmptySearchState {...defaultProps} />);
    const fileIcon = document.querySelector('.lucide-file-text');
    expect(fileIcon).toBeInTheDocument();
  });

  it('calls onCreateTask when Create button is clicked', async () => {
    const user = userEvent.setup();
    const onCreateTask = vi.fn();
    render(<EmptySearchState {...defaultProps} onCreateTask={onCreateTask} />);

    const createButton = screen.getByRole('button', {
      name: /Create "test query"/i,
    });
    await user.click(createButton);

    expect(onCreateTask).toHaveBeenCalledTimes(1);
  });

  it('calls onClearSearch when Clear Search button is clicked', async () => {
    const user = userEvent.setup();
    const onClearSearch = vi.fn();
    render(
      <EmptySearchState {...defaultProps} onClearSearch={onClearSearch} />
    );

    const clearButton = screen.getByRole('button', { name: /Clear Search/i });
    await user.click(clearButton);

    expect(onClearSearch).toHaveBeenCalledTimes(1);
  });

  it('shows tip when showArchived is false', () => {
    render(<EmptySearchState {...defaultProps} showArchived={false} />);
    expect(
      screen.getByText(/Tip: Enable "Show archived" to search old tasks/i)
    ).toBeInTheDocument();
  });

  it('hides tip when showArchived is true', () => {
    render(<EmptySearchState {...defaultProps} showArchived={true} />);
    expect(
      screen.queryByText(/Tip: Enable "Show archived" to search old tasks/i)
    ).not.toBeInTheDocument();
  });

  it('shows Lightbulb icon when showArchived is false', () => {
    render(<EmptySearchState {...defaultProps} showArchived={false} />);
    const lightbulbIcon = document.querySelector('.lucide-lightbulb');
    expect(lightbulbIcon).toBeInTheDocument();
  });

  it('hides Lightbulb icon when showArchived is true', () => {
    render(<EmptySearchState {...defaultProps} showArchived={true} />);
    const lightbulbIcon = document.querySelector('.lucide-lightbulb');
    expect(lightbulbIcon).not.toBeInTheDocument();
  });

  it('renders both action buttons', () => {
    render(<EmptySearchState {...defaultProps} />);
    expect(
      screen.getByRole('button', { name: /Create "test query"/i })
    ).toBeInTheDocument();
    expect(
      screen.getByRole('button', { name: /Clear Search/i })
    ).toBeInTheDocument();
  });

  it('displays different search query text', () => {
    render(
      <EmptySearchState {...defaultProps} searchQuery="add user login" />
    );
    expect(
      screen.getByText('No tasks match "add user login"')
    ).toBeInTheDocument();
    expect(
      screen.getByRole('button', { name: /Create "add user login"/i })
    ).toBeInTheDocument();
  });
});
