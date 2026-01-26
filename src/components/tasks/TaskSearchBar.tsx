import { useEffect, useRef } from 'react';
import { Search, X, Loader2 } from 'lucide-react';

interface TaskSearchBarProps {
  value: string;
  onChange: (value: string) => void;
  onClose: () => void;
  resultCount: number;
  isSearching: boolean;
}

export function TaskSearchBar({
  value,
  onChange,
  onClose,
  resultCount,
  isSearching,
}: TaskSearchBarProps) {
  const inputRef = useRef<HTMLInputElement>(null);

  // Auto-focus on mount
  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  const resultText =
    resultCount === 0
      ? 'No results'
      : `${resultCount} task${resultCount === 1 ? '' : 's'} found`;

  return (
    <div className="flex items-center gap-2 bg-background border rounded-lg shadow-md p-2">
      {/* Search icon */}
      <Search className="w-5 h-5 text-muted-foreground flex-shrink-0" />

      {/* Input field */}
      <input
        ref={inputRef}
        type="text"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder="Search tasks..."
        className="flex-1 bg-transparent border-none outline-none focus:outline-none focus-visible:outline-none focus:ring-0 focus-visible:ring-0 [&:focus]:outline-none [&:focus-visible]:outline-none text-foreground placeholder:text-muted-foreground"
        style={{ outline: 'none' }}
      />

      {/* Loading spinner */}
      {isSearching && (
        <Loader2 className="w-4 h-4 text-muted-foreground animate-spin" />
      )}

      {/* Result count */}
      {!isSearching && value && (
        <span className="text-sm text-muted-foreground whitespace-nowrap">
          {resultText}
        </span>
      )}

      {/* Close button */}
      <button
        onClick={onClose}
        className="flex-shrink-0 p-1 rounded hover:bg-muted transition-colors"
        aria-label="Close search"
      >
        <X className="w-4 h-4" />
      </button>
    </div>
  );
}
