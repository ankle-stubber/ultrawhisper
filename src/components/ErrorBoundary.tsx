import { Component, ReactNode, ErrorInfo } from "react";
import { AlertTriangle } from "lucide-react";

interface Props {
  children: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
  errorInfo: ErrorInfo | null;
}

export class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false, error: null, errorInfo: null };
  }

  static getDerivedStateFromError(error: Error): State {
    // Update state so the next render will show the fallback UI
    return { hasError: true, error, errorInfo: null };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    // Log the error to console for debugging
    console.error("ErrorBoundary caught an error:", error, errorInfo);
    this.setState({ errorInfo });
  }

  handleReset = () => {
    this.setState({ hasError: false, error: null, errorInfo: null });
    // Optionally reload the app
    window.location.reload();
  };

  render() {
    if (this.state.hasError) {
      return (
        <div className="flex h-screen items-center justify-center bg-gray-950 p-8">
          <div className="max-w-md w-full space-y-6">
            <div className="flex items-center justify-center">
              <div className="p-4 bg-red-500/10 rounded-full">
                <AlertTriangle className="w-12 h-12 text-red-500" />
              </div>
            </div>

            <div className="text-center space-y-2">
              <h1 className="text-xl font-semibold text-gray-100">
                Something went wrong
              </h1>
              <p className="text-sm text-gray-400">
                An unexpected error occurred in the application UI
              </p>
            </div>

            {this.state.error && (
              <div className="bg-gray-900 border border-gray-800 rounded-lg p-4 space-y-2">
                <p className="text-xs text-gray-500 uppercase tracking-wider">
                  Error Details
                </p>
                <p className="text-sm font-mono text-red-400">
                  {this.state.error.message}
                </p>
                {import.meta.env.DEV && this.state.errorInfo && (
                  <details className="mt-2">
                    <summary className="text-xs text-gray-500 cursor-pointer hover:text-gray-400">
                      Stack trace
                    </summary>
                    <pre className="mt-2 text-xs text-gray-600 overflow-auto max-h-40">
                      {this.state.errorInfo.componentStack}
                    </pre>
                  </details>
                )}
              </div>
            )}

            <div className="flex gap-3">
              <button
                onClick={this.handleReset}
                className="flex-1 px-4 py-2 bg-green-500 hover:bg-green-600 text-gray-950 font-medium rounded-lg transition-colors"
              >
                Reload Application
              </button>
              <button
                onClick={() => window.history.back()}
                className="flex-1 px-4 py-2 bg-gray-800 hover:bg-gray-700 text-gray-200 font-medium rounded-lg transition-colors"
              >
                Go Back
              </button>
            </div>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}
