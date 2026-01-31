/**
 * Activity components barrel export
 */

// Main component
export { ActivityView } from "./ActivityView";
export type { ActivityViewProps } from "./ActivityView";

// Sub-components
export { ActivityMessage } from "./ActivityMessage";
export type { ActivityMessageProps } from "./ActivityMessage";

export {
  ViewModeToggle,
  StatusFilter,
  FilterTabs,
  SearchBar,
  EmptyState,
} from "./ActivityFilters";
export type {
  ViewModeToggleProps,
  StatusFilterProps,
  FilterTabsProps,
  SearchBarProps,
  EmptyStateProps,
} from "./ActivityFilters";

export { ActivityContext } from "./ActivityContext";
export type { ActivityContextProps } from "./ActivityContext";

// Types
export type {
  MessageTypeFilter,
  ViewMode,
  ExpandedState,
  CopiedState,
  UnifiedActivityMessage,
} from "./ActivityView.types";
export { MESSAGE_TYPES, STATUS_OPTIONS } from "./ActivityView.types";

// Utilities
export {
  getMessageIcon,
  getMessageColor,
  getMessageBgColor,
  formatTimestamp,
  getToolName,
  generateMessageKey,
  highlightJSON,
  toUnifiedMessage,
  fromRealtimeMessage,
} from "./ActivityView.utils";
