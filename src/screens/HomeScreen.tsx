import { useEffect } from "react";
import { keys, keyServers, logout } from "../api/commands";
import { useAppStore } from "../state/appStore";
import { InfinityColors as C } from "../theme/colors";
import { pingColor } from "../theme/colors";

/**
 * Главный экран (Фаза 1): список ключей (подписок) и их серверов — стиль Happ
 * (списки раскрыты сразу). Туннеля ещё нет — connect/пинг придут на Фазах 2/5.
 */
export default function HomeScreen() {
  const store = useAppStore();
  const { keys: keyList, serversByKey, setKeys, setServers, setRoute, setError } = store;

  useEffect(() => {
    (async () => {
      try {
        const ks = await keys();
        setKeys(ks);
        // Грузим серверы всех ключей (список раскрыт как в Happ).
        for (const k of ks) {
          try {
            setServers(k.id, await keyServers(k.id));
          } catch {
            /* сервер отдельного ключа мог не ответить — пропускаем */
          }
        }
      } catch (e) {
        setError(errMessage(e));
      }
    })();
  }, [setKeys, setServers, setError]);

  async function onLogout() {
    await logout();
    setRoute("auth");
  }

  return (
    <div style={wrap}>
      <header style={header}>
        <h2 style={{ color: C.primaryLight, margin: 0 }}>Серверы</h2>
        <button style={linkBtn} onClick={onLogout}>Выйти</button>
      </header>

      {keyList.length === 0 && (
        <p style={{ color: C.textMuted }}>Нет подписок</p>
      )}

      {keyList.map((k) => (
        <section key={k.id} style={card}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <b style={{ color: C.textPrimary }}>{k.name ?? `Ключ ${k.id}`}</b>
            {k.is_premium && <span style={premium}>PREMIUM</span>}
          </div>
          {k.status && <span style={{ color: C.textMuted, fontSize: 12 }}>{k.status}</span>}

          <div style={{ marginTop: 8, display: "flex", flexDirection: "column", gap: 6 }}>
            {(serversByKey[k.id] ?? []).map((s) => (
              <div key={s.index} style={serverRow}>
                <span>{s.remark}</span>
                <span style={{ color: C.textMuted, fontSize: 12 }}>
                  {s.protocol} · {s.address}{s.port ? `:${s.port}` : ""}
                </span>
                {/* Пинг-пилл (Фаза 5): цвет по качеству */}
                <span style={{ ...pill, background: pingColor(-1) }}>—</span>
              </div>
            ))}
            {(serversByKey[k.id] ?? []).length === 0 && (
              <span style={{ color: C.textMuted, fontSize: 12 }}>Загрузка серверов…</span>
            )}
          </div>
        </section>
      ))}
    </div>
  );
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
const header: React.CSSProperties = {
  display: "flex", justifyContent: "space-between", alignItems: "center",
};
const linkBtn: React.CSSProperties = {
  background: "transparent", border: "none", color: C.textSecondary,
  cursor: "pointer", fontSize: 13,
};
const card: React.CSSProperties = {
  background: C.surface, borderRadius: 12, padding: 12,
  border: `1px solid ${C.surfaceElevated}`,
};
const serverRow: React.CSSProperties = {
  display: "flex", alignItems: "center", gap: 8, justifyContent: "space-between",
  padding: "6px 8px", borderRadius: 8, background: C.surfaceElevated,
};
const pill: React.CSSProperties = {
  color: "#fff", fontSize: 11, borderRadius: 999, padding: "2px 8px", minWidth: 28,
  textAlign: "center",
};
const premium: React.CSSProperties = {
  color: C.warning, fontSize: 10, fontWeight: 700, letterSpacing: 0.5,
};
