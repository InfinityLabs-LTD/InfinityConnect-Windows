import { useEffect, useState } from "react";
import { ping, onTunnelState } from "./api/commands";
import { useAppStore } from "./state/appStore";
import { InfinityColors } from "./theme/colors";

/**
 * Фаза 0: проверка моста invoke/emit end-to-end.
 *  - invoke("ping") → ответ от Rust.
 *  - listen("tunnel://state") → событие статуса от Rust (эмитит фейковое на старте).
 * Реальные экраны Home/Auth/Profile/Settings — Фаза 4.
 */
export default function App() {
  const [name, setName] = useState("Infinity");
  const { tunnel, lastPingReply, setTunnel, setPingReply } = useAppStore();

  useEffect(() => {
    const unlisten = onTunnelState(setTunnel);
    return () => {
      unlisten.then((u) => u());
    };
  }, [setTunnel]);

  async function onPing() {
    try {
      setPingReply(await ping(name));
    } catch (e) {
      setPingReply(`Ошибка: ${String(e)}`);
    }
  }

  return (
    <main
      style={{
        minHeight: "100vh",
        background: InfinityColors.background,
        color: InfinityColors.textPrimary,
        fontFamily: "Segoe UI, system-ui, sans-serif",
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        gap: 20,
      }}
    >
      <h1 style={{ color: InfinityColors.primaryLight, margin: 0 }}>
        Infinity Connect
      </h1>
      <p style={{ color: InfinityColors.textSecondary, margin: 0 }}>
        Windows-клиент · каркас Tauri 2 (Фаза 0)
      </p>

      <div style={{ display: "flex", gap: 8 }}>
        <input
          value={name}
          onChange={(e) => setName(e.currentTarget.value)}
          style={{
            padding: "8px 12px",
            borderRadius: 8,
            border: `1px solid ${InfinityColors.surfaceElevated}`,
            background: InfinityColors.surface,
            color: InfinityColors.textPrimary,
            outline: "none",
          }}
        />
        <button
          onClick={onPing}
          style={{
            padding: "8px 16px",
            borderRadius: 8,
            border: "none",
            background: InfinityColors.primary,
            color: "#fff",
            cursor: "pointer",
            fontWeight: 600,
          }}
        >
          invoke ping
        </button>
      </div>

      {lastPingReply && (
        <p style={{ color: InfinityColors.success, margin: 0 }}>
          ↩ {lastPingReply}
        </p>
      )}

      <p style={{ color: InfinityColors.textMuted, margin: 0, fontSize: 13 }}>
        Туннель (emit): <b>{tunnel.status}</b>
        {tunnel.message ? ` — ${tunnel.message}` : ""}
      </p>
    </main>
  );
}
