import { useEffect, useState } from "react";
import { connect, disconnect, keys, keyServers, pingServer, refreshSubscriptions, type Key, type SubscriptionServer } from "../api/commands";
import { useAppStore, pingKey } from "../state/appStore";
import { InfinityColors as C, pingColor } from "../theme/colors";
import { formatBytes, formatSpeed } from "../util/format";
import { countryCodeFromRemark } from "../util/countryFlag";
import { Flag, hasFlag } from "../components/Flag";
import { ConnectHero } from "../components/ConnectHero";
import { GlassCard, StatusPill, EmojiBadge, Chip } from "../components/ui";

/**
 * Главный экран (Фаза 4): hero-кнопка connect/disconnect + панель статистики +
 * аккордеон подписок в стиле Happ (выбранный ключ раскрыт со списком серверов,
 * бейдж «⚡ Быстрейший»). Пинг (Фаза 5) пока «—».
 */
export default function HomeScreen() {
  const s = useAppStore();
  const {
    keys: keyList, serversByKey, pings, selection, tunnel, stats,
    setKeys, setServers, setPing, setSelection, setError,
  } = s;

  const [refreshing, setRefreshing] = useState(false);

  /** Загружает ключи и серверы из кэша/сети, восстанавливает выбор. */
  async function loadKeys() {
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
  }

  useEffect(() => {
    (async () => {
      try {
        await loadKeys();
        // Автопинг всех серверов после загрузки (сериализованно на бэке).
        pingAll();
      } catch (e) {
        setError(errMessage(e));
      }
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  /** Кнопка ↻: обновляет подписки на сервере, перечитывает ключи и пингует. */
  async function onRefreshSubscriptions() {
    if (refreshing) return;
    setRefreshing(true);
    setError(null);
    try {
      await refreshSubscriptions();
      await loadKeys();
      pingAll();
    } catch (e) {
      setError(errMessage(e));
    } finally {
      setRefreshing(false);
    }
  }

  /** Пингует все загруженные серверы по очереди (бэк сериализует proxy-замеры). */
  async function pingAll() {
    const map = useAppStore.getState().serversByKey;
    for (const [keyIdStr, servers] of Object.entries(map)) {
      const keyId = Number(keyIdStr);
      for (const srv of servers) {
        try {
          const ms = await pingServer(keyId, srv.index);
          setPing(keyId, srv.index, ms);
        } catch {
          setPing(keyId, srv.index, -1);
        }
      }
    }
  }

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
    <div style={{ display: "flex", gap: 24, alignItems: "flex-start", flexWrap: "wrap" }}>
      {/* ЛЕВАЯ КОЛОНКА: hero подключения + статистика. */}
      <div style={{ flex: "1 1 340px", minWidth: 320, maxWidth: 440, display: "flex", flexDirection: "column", gap: 18 }}>
        <div style={{
          display: "flex", flexDirection: "column", alignItems: "center", gap: 14,
          padding: "34px 24px 30px", borderRadius: 24,
          background: "rgba(28,19,56,0.5)", border: `1px solid ${C.stroke}`, backdropFilter: "blur(12px)",
        }}>
          <ConnectHero status={tunnel.status} enabled={!!selection || connected || connecting} onToggle={onHero} compact={false} />
          <div style={{ fontSize: 16, fontWeight: 600 }}>{statusLabel(tunnel.status)}</div>
          {selectedServer && (
            <div style={{ color: C.muted, fontSize: 13, display: "flex", alignItems: "center", gap: 6 }}>
              <span style={{ opacity: 0.7 }}>Сервер:</span> {selectedServer.remark}
            </div>
          )}
          {tunnel.status === "error" && tunnel.message && (
            <div style={{ color: C.coral, fontSize: 12, maxWidth: 340, textAlign: "center" }}>{tunnel.message}</div>
          )}
        </div>

        {/* Статистика — всегда видна (нули до подключения). */}
        <div style={{ display: "flex", gap: 12 }}>
          <Stat label="↓ Скачано" value={formatBytes(stats?.downBytes ?? 0)} sub={formatSpeed(stats?.downSpeed ?? 0)} />
          <Stat label="↑ Отправлено" value={formatBytes(stats?.upBytes ?? 0)} sub={formatSpeed(stats?.upSpeed ?? 0)} />
        </div>

        {s.error && <div style={{ color: C.coral, fontSize: 13 }}>{s.error}</div>}
      </div>

      {/* ПРАВАЯ КОЛОНКА: серверы. */}
      <div style={{ flex: "2 1 420px", minWidth: 340, display: "flex", flexDirection: "column", gap: 12 }}>
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
          <h2 style={{ fontSize: 18, fontWeight: 700, margin: 0 }}>Серверы</h2>
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <button onClick={pingAll} title="Обновить пинг"
              style={{ background: C.surface, border: `1px solid ${C.stroke}`, borderRadius: 10, padding: "7px 12px", color: C.accentBlue, cursor: "pointer", fontSize: 13, fontWeight: 600 }}>
              ⟳ Обновить пинг
            </button>
            <button onClick={onRefreshSubscriptions} disabled={refreshing}
              title="Обновить подписки"
              style={{
                width: 34, height: 34, display: "flex", alignItems: "center", justifyContent: "center",
                background: C.surface, border: `1px solid ${C.stroke}`, borderRadius: "50%",
                color: C.accentBlue, cursor: refreshing ? "default" : "pointer", fontSize: 16,
                padding: 0, lineHeight: 1,
              }}>
              <span style={{
                display: "inline-block",
                animation: refreshing ? "ic-spin 0.8s linear infinite" : "none",
              }}>↻</span>
            </button>
            <style>{"@keyframes ic-spin { to { transform: rotate(360deg); } }"}</style>
          </div>
        </div>
        {keyList.length === 0 && <div style={{ color: C.mutedDim, fontSize: 13 }}>Нет подписок</div>}
        {keyList.map((k, i) => {
          const servers = serversByKey[k.id] ?? [];
          const isSelectedKey = selection?.keyId === k.id;
          const fastest = fastestIndex(k.id, servers, pings);
          return (
            <div key={k.id} style={{ display: "flex", flexDirection: "column", gap: 8 }}>
              <KeyCard k={k} number={i + 1} selected={isSelectedKey}
                onClick={() => servers[0] && setSelection({ keyId: k.id, serverIndex: servers[0].index })} />
              {isSelectedKey && servers.map((srv) => (
                <div key={srv.index} style={{ paddingLeft: 12 }}>
                  <ServerRow server={srv}
                    selected={selection?.serverIndex === srv.index}
                    isFastest={srv.index === fastest}
                    ping={pings[pingKey(k.id, srv.index)]}
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

function ServerRow({ server, selected, isFastest, ping, onClick }: { server: SubscriptionServer; selected: boolean; isFastest: boolean; ping: number | undefined; onClick: () => void }) {
  // ping: undefined — ещё не мерян; <=0 — недоступен/ошибка («—»); иначе мс.
  const pingText = ping === undefined ? "…" : ping <= 0 ? "—" : `${ping} мс`;
  return (
    <div onClick={onClick}
      onMouseEnter={(e) => { e.currentTarget.style.borderColor = `${C.accentBlue}8C`; e.currentTarget.style.transform = "translateX(3px)"; }}
      onMouseLeave={(e) => { e.currentTarget.style.borderColor = selected ? `${C.accentBlue}8C` : C.stroke; e.currentTarget.style.transform = "translateX(0)"; }}
      style={{
        display: "flex", alignItems: "center", gap: 12, cursor: "pointer",
        background: selected ? C.surfaceHi : C.surface,
        border: `1px solid ${selected ? `${C.accentBlue}8C` : C.stroke}`,
        borderRadius: 16, padding: "12px 14px",
        transition: "border-color 160ms, transform 160ms",
      }}>
      <ServerFlag remark={server.remark} />
      <div style={{ flex: 1, display: "flex", flexDirection: "column", gap: 2 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
          <span style={{ fontWeight: 600 }}>{stripCountryCode(server.remark)}</span>
          {isFastest && (
            <span style={{ color: C.mint, background: `${C.mint}1F`, borderRadius: 6, padding: "2px 7px", fontSize: 11, fontWeight: 600 }}>
              ⚡ Быстрейший
            </span>
          )}
        </div>
        <span style={{ color: C.muted, fontSize: 12 }}>
          {protocolDisplay(server.protocol)}
        </span>
      </div>
      {/* Пинг-пилл: цвет по КАЧЕСТВУ (pingColor). */}
      <StatusPill text={pingText} color={pingColor(ping === undefined ? null : ping)} />
    </div>
  );
}

/** Кружок-бейдж с SVG-флагом страны сервера (или глобус, если страна не распознана). */
function ServerFlag({ remark }: { remark: string }) {
  const cc = countryCodeFromRemark(remark);
  if (cc && hasFlag(cc)) {
    return (
      <div style={{ width: 38, height: 38, borderRadius: "50%", background: C.spaceElevated, border: `1px solid ${C.stroke}`, display: "flex", alignItems: "center", justifyContent: "center", flexShrink: 0, overflow: "hidden" }}>
        <Flag code={cc} size={24} />
      </div>
    );
  }
  return <EmojiBadge emoji="🌐" size={38} />;
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

// ── хелперы ──

function statusLabel(s: string): string {
  return s === "connected" ? "Подключено" : s === "connecting" ? "Подключение…" : s === "error" ? "Ошибка" : "Отключено";
}
/** Индекс сервера с минимальным валидным пингом в подписке (для бейджа). */
function fastestIndex(keyId: number, servers: SubscriptionServer[], pings: Record<string, number>): number {
  let best = -1;
  let bestMs = Infinity;
  for (const srv of servers) {
    const ms = pings[pingKey(keyId, srv.index)];
    if (ms !== undefined && ms > 0 && ms < bestMs) {
      bestMs = ms;
      best = srv.index;
    }
  }
  return best;
}
function keyTitle(number: number, name?: string): string {
  // Убираем технический суффикс @bot.local (и подобные @…) из имени ключа.
  const label = (name ?? "").trim().replace(/@[\w.-]+$/, "").trim();
  const hasName = label && !label.startsWith("Ключ #");
  return hasName ? `Ключ ${number} (${label})` : `Ключ ${number}`;
}
/** Убирает код страны/эмодзи-флаг из начала remark (флаг уже показан слева):
 *  «RU LTE» → «LTE», «🇳🇱 NL Нидерланды» → «Нидерланды». */
function stripCountryCode(remark: string): string {
  // Сначала срезаем ведущий не-буквенный мусор (эмодзи-флаг, пробелы).
  let s = remark.replace(/^[^A-Za-zА-Яа-я]+/, "");
  // Затем 2-буквенный код + разделитель, если он был.
  const stripped = s.replace(/^[A-Za-z]{2}[\s|·:_-]+/, "").trim();
  return stripped.length > 0 ? stripped : s.trim() || remark;
}
function protocolLabel(p: string): string {
  return p.toUpperCase() === "HYSTERIA2" ? "Hysteria2" : "VLESS";
}
/** Подпись протокола сервера (вместо IP:порт — не раскрываем адреса). */
function protocolDisplay(p: string): string {
  const up = p.toUpperCase();
  if (up === "HYSTERIA2") return "Hysteria2 · QUIC";
  if (up === "VLESS") return "VLESS · Reality/TLS";
  return p;
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
