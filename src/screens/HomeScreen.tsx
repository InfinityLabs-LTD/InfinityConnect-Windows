import { useEffect } from "react";
import { connect, disconnect, keys, keyServers, type Key, type SubscriptionServer } from "../api/commands";
import { useAppStore } from "../state/appStore";
import { InfinityColors as C, InfinityGradients as G, pingColor } from "../theme/colors";
import { formatBytes, formatSpeed } from "../util/format";
import { ConnectHero } from "../components/ConnectHero";
import { GlassCard, StatusPill, EmojiBadge, Eyebrow, Chip } from "../components/ui";

/**
 * Главный экран (Фаза 4): hero-кнопка connect/disconnect + панель статистики +
 * аккордеон подписок в стиле Happ (выбранный ключ раскрыт со списком серверов,
 * бейдж «⚡ Быстрейший»). Пинг (Фаза 5) пока «—».
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
      if (connected || connecting) await disconnect();
      else if (selection) await connect(selection.keyId, selection.serverIndex);
    } catch (e) {
      setError(errMessage(e));
    }
  }

  const selectedServer = selection
    ? serversByKey[selection.keyId]?.find((x) => x.index === selection.serverIndex)
    : undefined;

  return (
    <div style={{ minHeight: "100vh", background: G.screen, color: C.onSurface, fontFamily: FONT, padding: 16, display: "flex", flexDirection: "column", gap: 14 }}>
      {/* Топбар */}
      <header style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <b style={{ fontSize: 18, letterSpacing: -0.4 }}>Infinity Connect</b>
        <div style={{ display: "flex", gap: 4 }}>
          <IconBtn title="Профиль" onClick={() => setRoute("profile")}>👤</IconBtn>
          <IconBtn title="Настройки" onClick={() => setRoute("settings")}>⚙️</IconBtn>
        </div>
      </header>

      {/* Hero */}
      <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 6 }}>
        <ConnectHero status={tunnel.status} enabled={!!selection || connected || connecting} onToggle={onHero} compact={!connected && !connecting} />
        <div style={{ color: C.muted, fontSize: 13 }}>
          {statusLabel(tunnel.status)}{selectedServer ? ` · ${selectedServer.remark}` : ""}
        </div>
        {tunnel.status === "error" && tunnel.message && (
          <div style={{ color: C.coral, fontSize: 12, maxWidth: 340, textAlign: "center" }}>{tunnel.message}</div>
        )}
      </div>

      {/* Статистика */}
      {connected && stats && (
        <div style={{ display: "flex", gap: 8 }}>
          <Stat label="↓ Скачано" value={formatBytes(stats.downBytes)} sub={formatSpeed(stats.downSpeed)} />
          <Stat label="↑ Отправлено" value={formatBytes(stats.upBytes)} sub={formatSpeed(stats.upSpeed)} />
        </div>
      )}

      {s.error && <div style={{ color: C.coral, fontSize: 13 }}>{s.error}</div>}

      {/* Аккордеон подписок (стиль Happ) */}
      <Eyebrow>Серверы</Eyebrow>
      {keyList.length === 0 && <div style={{ color: C.mutedDim, fontSize: 13 }}>Нет подписок</div>}
      {keyList.map((k, i) => {
        const servers = serversByKey[k.id] ?? [];
        const isSelectedKey = selection?.keyId === k.id;
        const fastest = fastestIndex(servers);
        return (
          <div key={k.id} style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            <KeyCard k={k} number={i + 1} selected={isSelectedKey}
              onClick={() => servers[0] && setSelection({ keyId: k.id, serverIndex: servers[0].index })} />
            {/* Серверы раскрыты у выбранного ключа (аккордеон Happ). */}
            {isSelectedKey && servers.map((srv) => (
              <div key={srv.index} style={{ paddingLeft: 12 }}>
                <ServerRow server={srv}
                  selected={selection?.serverIndex === srv.index}
                  isFastest={srv.index === fastest}
                  onClick={() => setSelection({ keyId: k.id, serverIndex: srv.index })} />
              </div>
            ))}
            {isSelectedKey && servers.length === 0 && (
              <div style={{ paddingLeft: 12, color: C.mutedDim, fontSize: 12 }}>Загрузка серверов…</div>
            )}
          </div>
        );
      })}
    </div>
  );
}

// ── подкомпоненты экрана ──

function KeyCard({ k, number, selected, onClick }: { k: Key; number: number; selected: boolean; onClick: () => void }) {
  const title = keyTitle(number, k.name);
  return (
    <GlassCard highlighted={selected} onClick={onClick}>
      <div style={{ display: "flex", alignItems: "center", gap: 14 }}>
        <EmojiBadge emoji={k.is_premium ? "👑" : "🌐"} size={46} />
        <div style={{ flex: 1, display: "flex", flexDirection: "column", gap: 3 }}>
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <b style={{ flex: 1 }}>{title}</b>
            {k.protocol && <Chip text={protocolLabel(k.protocol)} />}
          </div>
          <span style={{ color: statusColor(k.status), fontSize: 12 }}>{statusLine(k)}</span>
          {k.device_limit != null && k.device_limit > 0 && (
            <span style={{ color: C.mutedDim, fontSize: 12 }}>Устройств: {k.devices_used ?? 0} / {k.device_limit}</span>
          )}
        </div>
        <div style={{ width: 10, height: 10, borderRadius: 5, background: statusDot(k.status) }} />
      </div>
    </GlassCard>
  );
}

function ServerRow({ server, selected, isFastest, onClick }: { server: SubscriptionServer; selected: boolean; isFastest: boolean; onClick: () => void }) {
  return (
    <div onClick={onClick}
      style={{
        display: "flex", alignItems: "center", gap: 12, cursor: "pointer",
        background: selected ? C.surfaceHi : C.surface,
        border: `1px solid ${selected ? `${C.accentBlue}8C` : C.stroke}`,
        borderRadius: 16, padding: "11px 12px",
      }}>
      <EmojiBadge emoji="🌐" size={38} />
      <div style={{ flex: 1, display: "flex", flexDirection: "column", gap: 2 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
          <span style={{ fontWeight: 600 }}>{server.remark}</span>
          {isFastest && (
            <span style={{ color: C.mint, background: `${C.mint}1F`, borderRadius: 6, padding: "2px 7px", fontSize: 11, fontWeight: 600 }}>
              ⚡ Быстрейший
            </span>
          )}
        </div>
        <span style={{ color: C.muted, fontSize: 12 }}>
          {server.protocol}{server.port ? ` · ${server.address}:${server.port}` : ""}
        </span>
      </div>
      {/* Пинг-пилл (Фаза 5): пока «—», цвет по качеству. */}
      <StatusPill text="—" color={pingColor(null)} />
    </div>
  );
}

function Stat({ label, value, sub }: { label: string; value: string; sub: string }) {
  return (
    <div style={{ flex: 1, background: C.surface, border: `1px solid ${C.stroke}`, borderRadius: 14, padding: 12, display: "flex", flexDirection: "column", gap: 2 }}>
      <span style={{ color: C.mutedDim, fontSize: 11 }}>{label}</span>
      <b style={{ fontSize: 16 }}>{value}</b>
      <span style={{ color: C.muted, fontSize: 11 }}>{sub}</span>
    </div>
  );
}

function IconBtn({ children, title, onClick }: { children: string; title: string; onClick: () => void }) {
  return (
    <button title={title} onClick={onClick}
      style={{ background: C.surface, border: `1px solid ${C.stroke}`, borderRadius: 10, width: 38, height: 38, fontSize: 16, cursor: "pointer" }}>
      {children}
    </button>
  );
}

// ── хелперы ──

const FONT = "Segoe UI, system-ui, sans-serif";

function statusLabel(s: string): string {
  return s === "connected" ? "Подключено" : s === "connecting" ? "Подключение…" : s === "error" ? "Ошибка" : "Отключено";
}
function fastestIndex(_servers: SubscriptionServer[]): number {
  // Пинг появится на Фазе 5 — тогда выбираем минимальный. Пока «нет быстрейшего».
  return -1;
}
function keyTitle(number: number, name?: string): string {
  const label = (name ?? "").trim();
  const hasName = label && !label.startsWith("Ключ #");
  return hasName ? `Ключ ${number} (${label})` : `Ключ ${number}`;
}
function protocolLabel(p: string): string {
  return p.toUpperCase() === "HYSTERIA2" ? "Hysteria2" : "VLESS";
}
function statusLine(k: Key): string {
  if (k.status === "EXPIRED") return "Срок истёк";
  if (k.status === "DISABLED") return "Отключена";
  if (k.status === "LIMITED") return "Достигнут лимит";
  return k.expires_at ? `Активна до ${k.expires_at.slice(0, 10)}` : "Активна";
}
function statusColor(status?: string): string {
  if (status === "EXPIRED") return C.coral;
  if (status === "LIMITED") return C.amber;
  if (status === "DISABLED") return C.mutedDim;
  return C.muted;
}
function statusDot(status?: string): string {
  if (status === "EXPIRED") return C.coral;
  if (status === "LIMITED") return C.amber;
  if (status === "DISABLED") return C.mutedDim;
  return C.mint;
}
function errMessage(e: unknown): string {
  if (e && typeof e === "object" && "message" in e) return String((e as { message?: string }).message ?? "Ошибка");
  return String(e);
}
