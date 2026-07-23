import { Component, type ErrorInfo, type ReactNode } from "react";
import { InfinityColors as C } from "../theme/colors";

interface Props { children: ReactNode }
interface State { error: Error | null }

/**
 * Ловит ошибки рендера дочерних экранов, чтобы приложение не превращалось в
 * пустой экран (раньше любое исключение в компоненте оставляло только фон).
 * Показывает понятное сообщение и кнопку перезагрузки UI.
 */
export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    // Оставляем след в консоли WebView для диагностики.
    console.error("UI error:", error, info.componentStack);
  }

  render() {
    if (this.state.error) {
      return (
        <div style={{ position: "relative", zIndex: 2, minHeight: "100vh", display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", gap: 14, padding: 24, fontFamily: "Segoe UI, system-ui, sans-serif", color: C.onSurface }}>
          <div style={{ fontSize: 40 }}>⚠️</div>
          <h2 style={{ margin: 0 }}>Что-то пошло не так</h2>
          <p style={{ color: C.muted, maxWidth: 420, textAlign: "center", margin: 0 }}>
            {this.state.error.message || "Непредвиденная ошибка интерфейса."}
          </p>
          <button
            onClick={() => window.location.reload()}
            style={{ marginTop: 6, padding: "11px 20px", borderRadius: 12, border: "none", background: C.accentIndigo, color: "#fff", fontWeight: 600, cursor: "pointer" }}
          >
            Перезагрузить
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}
