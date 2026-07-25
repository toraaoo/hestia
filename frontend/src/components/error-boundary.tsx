import { Component, type ErrorInfo, type ReactNode } from 'react';

import { Button } from '@/components/ui/button';
import { report } from '@/lib/crash';

interface Props {
  children: ReactNode;
}

interface State {
  error: Error | null;
}

export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    report('ui-render', error, info.componentStack ?? '');
  }

  render() {
    const { error } = this.state;
    if (!error) return this.props.children;

    return (
      <div className="flex h-screen flex-col items-center justify-center gap-4 p-8 text-center">
        <div className="space-y-1">
          <p className="font-medium text-sm">Something broke.</p>
          <p className="text-muted-foreground text-xs">
            A crash report was saved to the Hestia log directory.
          </p>
        </div>
        <pre className="max-h-40 max-w-lg overflow-auto border border-border p-3 text-left text-[11px] text-muted-foreground">
          {error.message}
        </pre>
        <Button size="sm" onClick={() => this.setState({ error: null })}>
          Try again
        </Button>
      </div>
    );
  }
}
