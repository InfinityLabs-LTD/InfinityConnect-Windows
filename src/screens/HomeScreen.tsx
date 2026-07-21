import { useEffect } from "react";
import {
  connect,
  disconnect,
  keys,
  keyServers,
  logout,
} from "../api/commands";
import { useAppStore } from "../state/appStore";
import { InfinityColors as C, pingColor } from "../theme/colors";
import { formatBytes, formatSpeed } from "../util/format";

/**
 * Главный экран (Фаза 2): hero-кнопка connect/disconnect + выбранный сервер +
 * список ключей и серверов (стиль Happ). Статистика тикает при подключении.
 * Пинг (Фаза 5) и полный UI-паритет (Фаза 4) — позже.
 */
export default function HomeScreen() {
  const s = useAppStore();
  const {
    keys: keyList, serversByKey, selection, tunnel, stats,
    setKeys, setServers, setSelection, setRoute, setError,
  } = s;

  useEffect(() => {
    (async () => {
      try {
        const ks = await keys();
        setKeys(ks);
        for (const k of ks) {
          try {
            const servers = await keyServers(k.id);
            setServers(k.id, servers);
            // Автовыбор первого сервера первого ключа.
            if (!useAppStore.getState().selection && servers.length > 0) {
              setSelection({ keyId: k.id, serverIndex: servers[0].index });
            }
          } catch {
            /* пропускаем ключ без ответа */
          }
        }
      } catch (e) {
        setError(errMessage(e));
      }
    })();
  }, [setKeys, setServers, setSelection, setError]);

  const connected = tunnel.status === "connected";
  const connecting = tunnel.status === "connecting";

  async function onHero() {
    setError(null);
    try {
      if (connected || connecting) {
        await disconnect();
      } else if (selection) {
        await connect(selection.keyId, selection.serverIndex);
      }
    } catch (e) {
      setError(errMessage(e));
    }
  }

  const selectedServer = selection
    ? serversByKey[selection.keyId]?.find((x) => x.index === selection.serverIndex)
    : undefined;

  return (
    <div style={wrap}>
      <header style={header}>
        <h2 style={{ color: C.primaryLight, margin: 0 }}>Infinity Connect</h2>
        <button style={linkBtn} onClick={async () => { await logout(); setRoute("auth"); }}>
          Выйти
        </button>
      </header>

      {/* Hero-кнопка */}
      <div style={heroWrap}>
        <button
          style={{ ...hero, background: heroColor(tunnel.status), opacity: selection ? 1 : 0.5 }}
          disabled={!selection && !connected}
          onClick={onHero}
        >
          {connected ? "Отключить" : connecting ? "Подключение…" : "Подключить"}
        </button>
        <div style={{ color: C.textSecondary, fontSize: 13, marginTop: 8 }}>
          {selectedServer ? selectedServer.remark : "Сервер не выбран"}
        </div>
        {tunnel.status === "error" && tunnel.message && (
          <div style={{ color: C.error, fontSize: 12, marginTop: 6, maxWidth: 340, textAlign: "center" }}>
            {tunnel.message}
          </div>
        )}
      </div>

      {/* Статистика */}
      {connected && stats && (
        <div style={statsRow}>
          <Stat label="↓ Скачано" value={formatBytes(stats.downBytes)} sub={formatSpeed(stats.downSpeed)} />
          <Stat label="↑ Отправлено" value={formatBytes(stats.upBytes)} sub={formatSpeed(stats.upSpeed)} />
        </div>
      )}

      {s.error && <p style={{ color: C.error, margin: 0 }}>{s.error}</p>}

      {/* Список ключей и серверов (стиль Happ) */}
      {keyList.map((k) => (
        <section key={k.id} style={card}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <b>{k.name ?? `Ключ ${k.id}`}</b>
            {k.is_premium && <span style={premium}>PREMIUM</span>}
          </div>
          <div style={{ marginTop: 8, display: "flex", flexDirection: "column", gap: 6 }}>
            {(serversByKey[k.id] ?? []).map((srv) => {
              const isSel = selection?.keyId === k.id && selection?.serverIndex === srv.index;
              return (
                <button
                  key={srv.index}
                  style={{ ...serverRow, border: isSel ? `1px solid ${C.primary}` : `1px solid transparent` }}
                  onClick={() => setSelection({ keyId: k.id, serverIndex: srv.index })}
                >
                  <span>{srv.remark}</span>
                  <span style={{ color: C.textMuted, fontSize: 12 }}>
                    {srv.protocol}{srv.port ? ` · ${srv.address}:${srv.port}` : ""}
                  </span>
                  <span style={{ ...pill, background: pingColor(-1) }}>—</span>
                </button>
              );
            })}
            {(serversByKey[k.id] ?? []).length === 0 && (
              <span style={{ color: C.textMuted, fontSize: 12 }}>Загрузка серверов…</span>
            )}
          </div>
        </section>
      ))}
    </div>
  );
}

function Stat({ label, value, sub }: { label: string; value: string; sub: string }) {
  return (
    <div style={statBox}>
      <span style={{ color: C.textMuted, fontSize: 11 }}>{label}</span>
      <b style={{ color: C.textPrimary, fontSize: 16 }}>{value}</b>
      <span style={{ color: C.textSecondary, fontSize: 11 }}>{sub}</span>
    </div>
  );
}

function heroColor(status: string): string {
  if (status === "connected") return C.success;
  if (status === "error") return C.error;
  return C.primary;
}

function errMessage(e: unknown): string {
  if (e && typeof e === "object" && "message" in e) {
    return String((e as { message?: string }).message ?? "Ошибка");
  }
  return String(e);
}

const wrap: React.CSSProperties = {
  minHeight: "100vh", background: C.background, color: C.textPrimary,
  fontFamily: "Segoe UI, system-ui, sans-serif", padding: 16,
  display: "flex", flexDirection: "column", gap: 12,
};
const header: React.CSSProperties = { display: "flex", justifyContent: "space-between", alignItems: "center" };
const linkBtn: React.CSSProperties = { background: "transparent", border: "none", color: C.textSecondary, cursor: "pointer", fontSize: 13 };
const heroWrap: React.CSSProperties = { display: "flex", flexDirection: "column", alignItems: "center", padding: "12px 0" };
const hero: React.CSSProperties = {
  width: 160, height: 160, borderRadius: "50%", border: "none", color: "#fff",
  fontSize: 18, fontWeight: 700, cursor: "pointer", boxShadow: "0 8px 32px rgba(124,77,255,0.4)",
};
const statsRow: React.CSSProperties = { display: "flex", gap: 8 };
const statBox: React.CSSProperties = {
  flex: 1, background: C.surface, borderRadius: 12, padding: 12,
  display: "flex", flexDirection: "column", gap: 2, border: `1px solid ${C.surfaceElevated}`,
};
const card: React.CSSProperties = { background: C.surface, borderRadius: 12, padding: 12, border: `1px solid ${C.surfaceElevated}` };
const serverRow: React.CSSProperties = {
  display: "flex", alignItems: "center", gap: 8, justifyContent: "space-between",
  padding: "8px 10px", borderRadius: 8, background: C.surfaceElevated,
  color: C.textPrimary, cursor: "pointer", textAlign: "left", width: "100%",
};
const pill: React.CSSProperties = { color: "#fff", fontSize: 11, borderRadius: 999, padding: "2px 8px", minWidth: 28, textAlign: "center" };
const premium: React.CSSProperties = { color: C.warning, fontSize: 10, fontWeight: 700, letterSpacing: 0.5 };
