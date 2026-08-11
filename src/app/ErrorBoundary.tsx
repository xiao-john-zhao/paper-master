import { Component, type ErrorInfo, type ReactNode } from "react";
import { uiLogger } from "../diagnostics/uiLogger";

interface Props {
  children: ReactNode;
}

interface State {
  failed: boolean;
}

export class ErrorBoundary extends Component<Props, State> {
  state: State = { failed: false };

  static getDerivedStateFromError(): State {
    return { failed: true };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    uiLogger.error("render", "The React view failed to render", {
      errorCode: "UI_RENDER_FAILED",
      detail: `${error.message}\n${info.componentStack ?? ""}`,
    });
  }

  render() {
    if (this.state.failed) {
      return (
        <main className="fatal-error">
          <h1>paper-master could not display this view</h1>
          <p>Your local library is safe. Restart the app and inspect Diagnostics if the problem continues.</p>
          <button onClick={() => window.location.reload()}>Reload application</button>
        </main>
      );
    }
    return this.props.children;
  }
}

