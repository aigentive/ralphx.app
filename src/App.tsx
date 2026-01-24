/**
 * RalphX - App Shell
 * Root component with QueryClientProvider and EventProvider
 */

import { QueryClientProvider } from "@tanstack/react-query";
import { ReactQueryDevtools } from "@tanstack/react-query-devtools";
import { getQueryClient } from "@/lib/queryClient";
import { EventProvider } from "@/providers/EventProvider";
import { TaskBoard } from "@/components/tasks/TaskBoard";

const queryClient = getQueryClient();

// Temporary hardcoded IDs until project selection is implemented
const DEFAULT_PROJECT_ID = "demo-project";
const DEFAULT_WORKFLOW_ID = "ralphx-default";

function AppContent() {
  return (
    <main className="min-h-screen flex flex-col" style={{ backgroundColor: "var(--bg-base)", color: "var(--text-primary)" }}>
      {/* Header */}
      <header className="flex items-center justify-between p-4 border-b" style={{ borderColor: "var(--border-subtle)" }}>
        <h1 className="text-xl font-bold" style={{ color: "var(--accent-primary)" }}>
          RalphX
        </h1>
        <span className="text-sm" style={{ color: "var(--text-muted)" }}>
          Demo Project
        </span>
      </header>

      {/* Main content - TaskBoard */}
      <div className="flex-1 overflow-hidden">
        <TaskBoard
          projectId={DEFAULT_PROJECT_ID}
          workflowId={DEFAULT_WORKFLOW_ID}
        />
      </div>
    </main>
  );
}

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <EventProvider>
        <AppContent />
      </EventProvider>
      <ReactQueryDevtools initialIsOpen={false} />
    </QueryClientProvider>
  );
}

export default App;
