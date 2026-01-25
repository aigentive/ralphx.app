import { Component, type ReactNode } from "react";

interface Props {
  children: ReactNode;
  fallback?: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
  errorInfo: React.ErrorInfo | null;
}

/**
 * Error Boundary component that catches React errors and displays them visually.
 * In development, shows full error details. In production, shows a generic message.
 */
export class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false, error: null, errorInfo: null };
  }

  static getDerivedStateFromError(error: Error): Partial<State> {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    this.setState({ errorInfo });
    // Log to console for visibility
    console.error("ErrorBoundary caught an error:", error, errorInfo);
  }

  render() {
    if (this.state.hasError) {
      // Custom fallback provided
      if (this.props.fallback) {
        return this.props.fallback;
      }

      const isDev = import.meta.env.DEV;

      return (
        <div
          style={{
            padding: "20px",
            margin: "20px",
            borderRadius: "8px",
            backgroundColor: "rgba(239, 68, 68, 0.1)",
            border: "1px solid rgba(239, 68, 68, 0.3)",
            fontFamily: "SF Pro, system-ui, sans-serif",
          }}
        >
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: "8px",
              marginBottom: "12px",
            }}
          >
            <span style={{ fontSize: "20px" }}>⚠️</span>
            <h2
              style={{
                margin: 0,
                fontSize: "16px",
                fontWeight: 600,
                color: "#ef4444",
              }}
            >
              Something went wrong
            </h2>
          </div>

          {isDev && this.state.error && (
            <>
              <div
                style={{
                  padding: "12px",
                  borderRadius: "6px",
                  backgroundColor: "rgba(0, 0, 0, 0.4)",
                  marginBottom: "12px",
                  overflow: "auto",
                }}
              >
                <code
                  style={{
                    fontSize: "13px",
                    color: "#fca5a5",
                    whiteSpace: "pre-wrap",
                    wordBreak: "break-word",
                  }}
                >
                  {this.state.error.toString()}
                </code>
              </div>

              {this.state.errorInfo && (
                <details style={{ marginTop: "8px" }}>
                  <summary
                    style={{
                      cursor: "pointer",
                      fontSize: "13px",
                      color: "#9ca3af",
                      marginBottom: "8px",
                    }}
                  >
                    Component Stack
                  </summary>
                  <div
                    style={{
                      padding: "12px",
                      borderRadius: "6px",
                      backgroundColor: "rgba(0, 0, 0, 0.4)",
                      overflow: "auto",
                      maxHeight: "300px",
                    }}
                  >
                    <pre
                      style={{
                        margin: 0,
                        fontSize: "11px",
                        color: "#9ca3af",
                        whiteSpace: "pre-wrap",
                      }}
                    >
                      {this.state.errorInfo.componentStack}
                    </pre>
                  </div>
                </details>
              )}
            </>
          )}

          <button
            onClick={() => this.setState({ hasError: false, error: null, errorInfo: null })}
            style={{
              marginTop: "12px",
              padding: "8px 16px",
              borderRadius: "6px",
              border: "none",
              backgroundColor: "#ef4444",
              color: "white",
              fontSize: "13px",
              fontWeight: 500,
              cursor: "pointer",
            }}
          >
            Try Again
          </button>
        </div>
      );
    }

    return this.props.children;
  }
}
