'use client';

import { Component, ReactNode, ErrorInfo } from 'react';
import { AlertCircle, RotateCcw } from 'lucide-react';

interface Props {
  children: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error('Error caught by boundary:', error, errorInfo);
  }

  resetError = () => {
    this.setState({ hasError: false, error: null });
  };

  render() {
    if (this.state.hasError) {
      return (
        <div className="min-h-screen bg-slate-950 flex items-center justify-center p-4">
          <div className="rounded-lg border border-red-700/50 bg-red-900/20 p-8 max-w-md">
            <div className="flex items-start gap-4">
              <AlertCircle className="w-6 h-6 text-red-400 flex-shrink-0 mt-1" />
              <div className="flex-1">
                <h2 className="text-xl font-semibold text-red-300 mb-2">Something went wrong</h2>
                <p className="text-sm text-red-200 mb-4">
                  {this.state.error?.message || 'An unexpected error occurred'}
                </p>
                <button
                  onClick={this.resetError}
                  className="inline-flex items-center gap-2 px-4 py-2 bg-red-900/40 hover:bg-red-900/60 text-red-300 rounded-lg transition-colors border border-red-700/50"
                >
                  <RotateCcw className="w-4 h-4" />
                  Try again
                </button>
              </div>
            </div>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}
