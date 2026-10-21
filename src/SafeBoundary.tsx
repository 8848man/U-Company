import { Component, type ErrorInfo, type ReactNode } from 'react';

type Props = { label: string; children: ReactNode; onClose?: () => void };
type State = { error: string | null };

/** Keeps a side panel's error inside that panel, so one broken window never blanks the whole app. */
export class SafeBoundary extends Component<Props, State> {
  state: State = { error: null };
  static getDerivedStateFromError(error: unknown): State { return { error: String(error instanceof Error ? error.message : error) }; }
  componentDidCatch(error: unknown, info: ErrorInfo) { console.error(`[${this.props.label}]`, error, info.componentStack); }
  render() {
    if (!this.state.error) return this.props.children;
    return <section className="office-window is-minimized safe-boundary" role="alert">
      <header className="office-window-bar"><strong>{this.props.label}에 문제가 생겼어요</strong>
        <div className="office-window-controls"><button type="button" aria-label="다시 열기" title="다시 열기" onClick={() => this.setState({ error: null })}>↻</button>{this.props.onClose && <button type="button" aria-label="닫기" title="닫기" onClick={this.props.onClose}>×</button>}</div>
      </header>
    </section>;
  }
}
