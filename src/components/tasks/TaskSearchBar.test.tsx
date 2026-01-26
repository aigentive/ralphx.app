import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { TaskSearchBar } from './TaskSearchBar';

describe('TaskSearchBar', () => {
  const defaultProps = {
    value: '',
    onChange: vi.fn(),
    onClose: vi.fn(),
    resultCount: 0,
    isSearching: false,
  };

  it('renders with placeholder text', () => {
    render(<TaskSearchBar {...defaultProps} />);
    expect(
      screen.getByPlaceholderText('Search tasks...')
    ).toBeInTheDocument();
  });

  it('auto-focuses input on mount', () => {
    render(<TaskSearchBar {...defaultProps} />);
    const input = screen.getByPlaceholderText('Search tasks...');
    expect(input).toHaveFocus();
  });

  it('calls onChange when user types', async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<TaskSearchBar {...defaultProps} onChange={onChange} />);

    const input = screen.getByPlaceholderText('Search tasks...');
    await user.type(input, 'test query');

    expect(onChange).toHaveBeenCalled();
  });

  it('calls onClose when close button is clicked', async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    render(<TaskSearchBar {...defaultProps} onClose={onClose} />);

    const closeButton = screen.getByLabelText('Close search');
    await user.click(closeButton);

    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('shows loading spinner when isSearching is true', () => {
    render(<TaskSearchBar {...defaultProps} isSearching={true} />);
    const spinner = document.querySelector('.animate-spin');
    expect(spinner).toBeInTheDocument();
  });

  it('hides loading spinner when isSearching is false', () => {
    render(<TaskSearchBar {...defaultProps} isSearching={false} />);
    const spinner = document.querySelector('.animate-spin');
    expect(spinner).not.toBeInTheDocument();
  });

  it('shows "No results" when resultCount is 0 and has value', () => {
    render(
      <TaskSearchBar {...defaultProps} value="test" resultCount={0} />
    );
    expect(screen.getByText('No results')).toBeInTheDocument();
  });

  it('shows "1 task found" when resultCount is 1', () => {
    render(
      <TaskSearchBar {...defaultProps} value="test" resultCount={1} />
    );
    expect(screen.getByText('1 task found')).toBeInTheDocument();
  });

  it('shows "N tasks found" when resultCount is greater than 1', () => {
    render(
      <TaskSearchBar {...defaultProps} value="test" resultCount={5} />
    );
    expect(screen.getByText('5 tasks found')).toBeInTheDocument();
  });

  it('does not show result count when value is empty', () => {
    render(<TaskSearchBar {...defaultProps} value="" resultCount={5} />);
    expect(screen.queryByText('5 tasks found')).not.toBeInTheDocument();
  });

  it('does not show result count when isSearching is true', () => {
    render(
      <TaskSearchBar
        {...defaultProps}
        value="test"
        resultCount={5}
        isSearching={true}
      />
    );
    expect(screen.queryByText('5 tasks found')).not.toBeInTheDocument();
  });

  it('renders search icon', () => {
    render(<TaskSearchBar {...defaultProps} />);
    const searchIcon = document.querySelector('.lucide-search');
    expect(searchIcon).toBeInTheDocument();
  });

  it('renders close icon', () => {
    render(<TaskSearchBar {...defaultProps} />);
    const closeIcon = document.querySelector('.lucide-x');
    expect(closeIcon).toBeInTheDocument();
  });
});
