/**
 * RalphX - App Shell
 * Minimal shell with dark theme and design system
 */

function App() {
  return (
    <main className="min-h-screen bg-bg-base text-text-primary font-body p-8 flex flex-col items-center justify-center">
      <div className="bg-bg-surface rounded-lg p-8 shadow-md border border-border max-w-md w-full">
        <h1 className="text-3xl font-display font-bold text-accent-primary mb-4">
          RalphX
        </h1>

        <p className="text-text-secondary mb-6">
          Autonomous AI-driven development system
        </p>

        <div className="flex flex-col gap-4">
          <div className="flex items-center gap-3">
            <span className="w-3 h-3 rounded-full bg-status-success"></span>
            <span className="text-sm text-text-muted">Backend ready</span>
          </div>

          <div className="flex items-center gap-3">
            <span className="w-3 h-3 rounded-full bg-status-success"></span>
            <span className="text-sm text-text-muted">Frontend ready</span>
          </div>

          <div className="flex items-center gap-3">
            <span className="w-3 h-3 rounded-full bg-accent-secondary"></span>
            <span className="text-sm text-text-muted">Foundation phase in progress</span>
          </div>
        </div>

        <div className="mt-8 pt-6 border-t border-border-subtle">
          <p className="text-xs text-text-muted text-center">
            Built with Tauri 2.0 + React + TypeScript
          </p>
        </div>
      </div>
    </main>
  );
}

export default App;
