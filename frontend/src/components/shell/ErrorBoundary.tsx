import { Component, type ReactNode } from "react";

interface Props {
  children: ReactNode;
}
interface State {
  error: Error | null;
}

/** Prevents the whole app from blanking on a render error; shows a friendly panel. */
export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  render() {
    if (this.state.error) {
      return (
        <div className="card mx-auto mt-4 max-w-2xl p-6 text-sm">
          <p className="font-semibold text-(--cat-social)">Something went wrong while rendering.</p>
          <p className="mt-2 break-words text-(--text-secondary)">
            {this.state.error?.message || String(this.state.error)}
          </p>
          <pre className="mt-2 max-h-40 overflow-auto whitespace-pre-wrap rounded-lg bg-(--surface-2) p-3 text-[11px] text-(--text-secondary)">
            {this.state.error?.stack}
          </pre>
          <button
            type="button"
            className="mt-4 rounded-lg bg-(--accent) px-4 py-2 text-white"
            onClick={() => this.setState({ error: null })}
          >
            Try again
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}
