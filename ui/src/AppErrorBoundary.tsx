import { Component, type ErrorInfo, type ReactNode } from "react";
import { MODEL_STORAGE } from "./storage";

interface AppErrorBoundaryProps {
  children: ReactNode;
  reloadPage?: () => void;
  clearStoredModel?: () => void;
}

interface AppErrorBoundaryState {
  error?: Error;
}

export function clearStoredModelAndReload(
  storage: Pick<Storage, "removeItem"> = localStorage,
  reload: () => void = () => window.location.reload(),
) {
  try {
    storage.removeItem(MODEL_STORAGE);
  } catch {
    // Reload even when persistent storage is unavailable.
  }
  reload();
}

export class AppErrorBoundary extends Component<
  AppErrorBoundaryProps,
  AppErrorBoundaryState
> {
  state: AppErrorBoundaryState = {};

  static getDerivedStateFromError(error: unknown): AppErrorBoundaryState {
    return {
      error: error instanceof Error ? error : new Error(String(error)),
    };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("Gacha Lab rendering failed", error, info);
  }

  render() {
    if (!this.state.error) return this.props.children;
    const reloadPage = this.props.reloadPage ?? (() => window.location.reload());
    const clearStoredModel = this.props.clearStoredModel ?? (() => clearStoredModelAndReload());
    return (
      <main className="fatal-error-shell" role="alert">
        <section className="fatal-error-card">
          <p className="fatal-error-kicker">Gacha Lab 복구 안내</p>
          <h1>앱을 표시하는 중 문제가 발생했습니다.</h1>
          <p>새로고침하거나 저장된 모델을 지우고 기본 모델로 다시 시작할 수 있습니다.</p>
          <div className="fatal-error-actions">
            <button type="button" onClick={reloadPage}>새로고침</button>
            <button type="button" className="danger" onClick={clearStoredModel}>
              저장된 모델 지우고 다시 시작
            </button>
          </div>
          <details>
            <summary>오류 세부 정보</summary>
            <pre>{this.state.error.message}</pre>
          </details>
        </section>
      </main>
    );
  }
}
