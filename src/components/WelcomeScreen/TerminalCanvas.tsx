/**
 * TerminalCanvas - Visual terminal element with floating code fragments
 *
 * A mock terminal displaying animated code that suggests AI orchestration.
 * Features traffic light header, typing cursor, syntax-highlighted code output,
 * and floating code fragments.
 *
 * Anti-AI-Slop: No purple/blue gradients, warm orange #ff6b35 accent
 */

interface TerminalCanvasProps {
  className?: string;
}

export default function TerminalCanvas({ className = "" }: TerminalCanvasProps) {
  return (
    <div
      className={`relative w-full max-w-2xl mx-auto rounded-xl overflow-hidden ${className}`}
      style={{
        backgroundColor: "var(--bg-surface)",
        border: "1px solid var(--border-subtle)",
        boxShadow: "var(--shadow-lg)",
      }}
    >
      {/* Terminal header with traffic lights */}
      <div
        className="flex items-center gap-2 px-4 py-3"
        style={{
          backgroundColor: "var(--bg-elevated)",
          borderBottom: "1px solid var(--border-subtle)",
        }}
      >
        <div className="flex gap-1.5">
          <div className="w-3 h-3 rounded-full bg-[#ff5f57]" />
          <div className="w-3 h-3 rounded-full bg-[#ffbd2e]" />
          <div className="w-3 h-3 rounded-full bg-[#28c840]" />
        </div>
        <span
          className="ml-2 text-xs font-medium"
          style={{
            color: "var(--text-muted)",
            fontFamily: "var(--font-mono)",
          }}
        >
          ralphx ~ orchestrator
        </span>
      </div>

      {/* Terminal body with typing animation */}
      <div
        className="p-6 min-h-[200px] relative"
        style={{ fontFamily: "var(--font-mono)" }}
      >
        {/* Typing line with cursor */}
        <div className="flex items-center gap-2 mb-4">
          <span style={{ color: "#28c840" }}>$</span>
          <span style={{ color: "var(--text-primary)" }}>
            ralphx init --agent orchestrator
          </span>
          <span
            className="inline-block w-2 h-5 ml-1 terminal-cursor"
            style={{
              backgroundColor: "var(--accent-primary)",
            }}
          />
        </div>

        {/* Code output lines */}
        <div
          className="space-y-2 text-sm"
          style={{ color: "var(--text-secondary)" }}
        >
          <p>
            <span style={{ color: "var(--text-muted)" }}>// </span>
            <span style={{ color: "#6a9955" }}>
              Initializing autonomous development environment...
            </span>
          </p>
          <p>
            <span style={{ color: "var(--accent-primary)" }}>agent</span>
            <span style={{ color: "var(--text-muted)" }}>.</span>
            <span style={{ color: "#dcdcaa" }}>spawn</span>
            <span style={{ color: "var(--text-muted)" }}>(</span>
            <span style={{ color: "#ce9178" }}>'worker'</span>
            <span style={{ color: "var(--text-muted)" }}>)</span>
          </p>
          <p>
            <span style={{ color: "var(--accent-primary)" }}>agent</span>
            <span style={{ color: "var(--text-muted)" }}>.</span>
            <span style={{ color: "#dcdcaa" }}>spawn</span>
            <span style={{ color: "var(--text-muted)" }}>(</span>
            <span style={{ color: "#ce9178" }}>'reviewer'</span>
            <span style={{ color: "var(--text-muted)" }}>)</span>
          </p>
          <p>
            <span style={{ color: "var(--accent-primary)" }}>await</span>
            <span style={{ color: "var(--text-primary)" }}> </span>
            <span style={{ color: "#dcdcaa" }}>orchestrate</span>
            <span style={{ color: "var(--text-muted)" }}>()</span>
          </p>
        </div>

        {/* Floating code fragments */}
        <div
          className="absolute top-4 right-4 text-xs px-2 py-1 rounded code-fragment"
          style={{
            backgroundColor: "rgba(255, 107, 53, 0.1)",
            color: "var(--accent-primary)",
          }}
        >
          {'{ status: "ready" }'}
        </div>

        <div
          className="absolute bottom-6 right-8 text-xs px-2 py-1 rounded code-fragment"
          style={{
            backgroundColor: "rgba(40, 200, 64, 0.1)",
            color: "#28c840",
            animationDelay: "1.5s",
          }}
        >
          {'task.complete()'}
        </div>

        <div
          className="absolute top-1/2 right-6 text-xs px-2 py-1 rounded code-fragment"
          style={{
            backgroundColor: "rgba(220, 220, 170, 0.1)",
            color: "#dcdcaa",
            animationDelay: "0.8s",
          }}
        >
          {'agents: 3'}
        </div>
      </div>
    </div>
  );
}
