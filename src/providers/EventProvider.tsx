/**
 * EventProvider - Global event listener setup
 *
 * This component sets up global Tauri event listeners for the application.
 * It should wrap the main App content to ensure events are captured
 * throughout the application lifecycle.
 */

import type { ReactNode } from "react";
import {
  useTaskEvents,
  useSupervisorAlerts,
  useReviewEvents,
  useFileChangeEvents,
  useAgentEvents,
  useProposalEvents,
  useStepEvents,
  useExecutionErrorEvents,
} from "@/hooks/useEvents";

interface EventProviderProps {
  children: ReactNode;
}

/**
 * Global event provider component
 *
 * Sets up all global event listeners:
 * - Task events (created, updated, deleted, status_changed)
 * - Proposal events (created, updated, deleted)
 * - Step events (created, updated, deleted, reordered)
 * - Supervisor alerts
 * - Review events (placeholder for Phase 9)
 * - File change events (placeholder)
 *
 * @example
 * ```tsx
 * function App() {
 *   return (
 *     <QueryClientProvider client={queryClient}>
 *       <EventProvider>
 *         <Router>
 *           <Routes />
 *         </Router>
 *       </EventProvider>
 *     </QueryClientProvider>
 *   );
 * }
 * ```
 */
export function EventProvider({ children }: EventProviderProps) {
  // Set up global event listeners
  useTaskEvents();
  useProposalEvents(); // Listen to proposal events for Ideation view
  useStepEvents(); // Listen to step events for task execution progress
  useSupervisorAlerts();
  useReviewEvents();
  useFileChangeEvents();
  useAgentEvents(); // Listen to agent:message events for Activity view
  useExecutionErrorEvents(); // Handle agent execution errors and unstick UI

  return <>{children}</>;
}
