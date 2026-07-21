import { useEffect, useState } from "react";
import {
  isAutostartEnabled, setAutostart,
  getPingSettings, setPingSettings,
  getRoutingSettings, setRoutingSettings,
  type PingSettings, type PingMethod, type PingMode,
  type RoutingSettings, type SiteRoutingMode,
} from "../api/commands";
import { useAppStore } from "../state/appStore";
import { InfinityColors as C } from "../theme/colors";
import { Scaffold } from "../components/Scaffold";
import { GlassCard, Eyebrow } from "../components/ui";

/** Версии ядер — синхронно с fetch-binaries.ps1 / Android BuildFlags. */
const XRAY_VERSION = "26.3.27";
const HYSTERIA_VERSION = "2.10.0";
const APP_VERSION = "0.1.0";

/** Хаб настроек: 3 пункта → Маршрутизация / Пинг / О приложении. */
export function SettingsHub() {
  const { setRoute } = useAppStore();
  return (
    <Scaffold title="Настройки" onBack={() => setRoute("home")}>
      <HubItem icon="🧭" title="Маршрутизация" subtitle="По приложениям и сайтам"
        onClick={() => setRoute("settings/routing")} />
      <HubItem icon="📶" title="Настройки пинга" subtitle="Метод, режим, таймаут"
        onClick={() => setRoute("settings/ping")} />
      <HubItem icon="ℹ️" title="О приложении" subtitle="Версия, ядра, разработчик"
        onClick={() => setRoute("settings/about")} />
    </Scaffold>
  );
}

function HubItem({ icon, title, subtitle, onClick }: { icon: string; title: string; subtitle: string; onClick: () => void }) {
  return (
    <GlassCard onClick={onClick}>
      <div style={{ display: "flex", alignItems: "center", gap: 14 }}>
        <span style={{ fontSize: 22 }}>{icon}</span>
        <div style={{ flex: 1 }}>
          <b>{title}</b>
          <div style={{ color: C.muted, fontSize: 12 }}>{subtitle}</div>
        </div>
        <span style={{ color: C.mutedDim, fontSize: 20 }}>›</span>
      </div>
    </GlassCard>
  );
}

const SITE_MODES: { value: SiteRoutingMode; label: string; hint: string }[] = [
  { value: "Off", label: "Выключено", hint: "Список доменов не используется" },
  { value: "Proxy", label: "Через VPN", hint: "Указанные домены — через VPN" },
  { value: "Direct", label: "Напрямую", hint: "Указанные домены — мимо VPN" },
];

/** Экран маршрутизации: по сайтам (домены) + пометка про per-app (Фаза 7). */
export function RoutingScreen() {
  const { setRoute } = useAppStore();
  const [r, setR] = useState<RoutingSettings | null>(null);
  const [draft, setDraft] = useState("");
  const [appsDraft, setAppsDraft] = useState("");

  useEffect(() => {
    getRoutingSettings().then((rs) => {
      setR(rs);
      setDraft(rs.sites.join("\n"));
      setAppsDraft(rs.apps.join("\n"));
    }).catch(() => {});
  }, []);

  function update(patch: Partial<RoutingSettings>) {
    if (!r) return;
    const next = { ...r, ...patch };
    setR(next);
    setRoutingSettings(next).catch(() => {});
  }

  function commitDomains() {
    const sites = draft.split("\n").map((s) => s.trim()).filter(Boolean);
    update({ sites });
  }

  if (!r) {
    return (
      <Scaffold title="Маршрутизация" onBack={() => setRoute("settings")}>
        <div style={{ color: C.muted }}>Загрузка…</div>
      </Scaffold>
    );
  }

  return (
    <Scaffold title="Маршрутизация" onBack={() => setRoute("settings")}>
      <Eyebrow>По сайтам</Eyebrow>
      {SITE_MODES.map((m) => (
        <GlassCard key={m.value} highlighted={r.site_mode === m.value} onClick={() => update({ site_mode: m.value })}>
          <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
            <Radio on={r.site_mode === m.value} />
            <div style={{ flex: 1 }}>
              <b>{m.label}</b>
              <div style={{ color: C.muted, fontSize: 12 }}>{m.hint}</div>
            </div>
          </div>
        </GlassCard>
      ))}

      {r.site_mode !== "Off" && (
        <>
          <Eyebrow>Домены (по одному в строке)</Eyebrow>
          <textarea value={draft} onChange={(e) => setDraft(e.currentTarget.value)} onBlur={commitDomains}
            placeholder={"youtube.com\nnetflix.com"} rows={5}
            style={{ width: "100%", padding: "10px 12px", borderRadius: 12, border: `1px solid ${C.stroke}`, background: C.surface, color: C.onSurface, outline: "none", boxSizing: "border-box", fontFamily: "monospace", fontSize: 13, resize: "vertical" }} />
          <div style={{ color: C.mutedDim, fontSize: 12 }}>
            «domain» матчит и поддомены. Применяется только к VLESS-серверам.
          </div>
        </>
      )}

      <Eyebrow>Kill-switch</Eyebrow>
      <GlassCard onClick={() => update({ kill_switch: !r.kill_switch })}>
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
          <div style={{ flex: 1 }}>
            <b>Блокировать трафик мимо VPN</b>
            <div style={{ color: C.muted, fontSize: 12 }}>
              При обрыве ядра не пускать трафик в обход туннеля
            </div>
          </div>
          <Toggle on={r.kill_switch} />
        </div>
      </GlassCard>

      <Eyebrow>По приложениям</Eyebrow>
      <GlassCard>
        <div style={{ color: C.muted, fontSize: 13, marginBottom: r.app_mode === "Disallow" ? 10 : 0 }}>
          Режим «кроме выбранных»: указанным приложениям блокируется трафик мимо
          VPN (по пути .exe). Режим «только выбранные» требует драйвера — позже.
        </div>
        <div style={{ display: "flex", gap: 8, marginTop: 8 }}>
          {(["Off", "Disallow"] as const).map((m) => (
            <button key={m} onClick={() => update({ app_mode: m })}
              style={{ flex: 1, padding: "8px", borderRadius: 10, cursor: "pointer",
                border: `1px solid ${r.app_mode === m ? C.accentBlue : C.stroke}`,
                background: r.app_mode === m ? `${C.accentBlue}1F` : "transparent",
                color: C.onSurface, fontSize: 13 }}>
              {m === "Off" ? "Выключено" : "Кроме выбранных"}
            </button>
          ))}
        </div>
        {r.app_mode === "Disallow" && (
          <textarea value={appsDraft} onChange={(e) => setAppsDraft(e.currentTarget.value)}
            onBlur={() => update({ apps: appsDraft.split("\n").map((s) => s.trim()).filter(Boolean) })}
            placeholder={"C:\\Program Files\\App\\app.exe"} rows={3}
            style={{ width: "100%", marginTop: 10, padding: "10px 12px", borderRadius: 12, border: `1px solid ${C.stroke}`, background: C.spaceElevated, color: C.onSurface, outline: "none", boxSizing: "border-box", fontFamily: "monospace", fontSize: 12, resize: "vertical" }} />
        )}
      </GlassCard>
    </Scaffold>
  );
}

const METHODS: { value: PingMethod; label: string; hint: string }[] = [
  { value: "ProxyGet", label: "Прокси GET", hint: "HTTP GET через ядро (end-to-end)" },
  { value: "ProxyHead", label: "Прокси HEAD", hint: "HTTP HEAD через ядро (легче)" },
  { value: "Tcp", label: "TCP", hint: "Хендшейк до host:port" },
  { value: "Icmp", label: "ICMP", hint: "Echo (сетевой RTT)" },
];
const MODES: { value: PingMode; label: string }[] = [
  { value: "Default", label: "Default (лучший из N)" },
  { value: "Double", label: "Double (второй замер)" },
  { value: "Keepalive", label: "Keepalive (по TLS)" },
];

/** Экран настроек пинга: метод, режим, URL, таймаут. */
export function PingScreen() {
  const { setRoute } = useAppStore();
  const [s, setS] = useState<PingSettings | null>(null);

  useEffect(() => {
    getPingSettings().then(setS).catch(() => {});
  }, []);

  function update(patch: Partial<PingSettings>) {
    if (!s) return;
    const next = { ...s, ...patch };
    setS(next);
    setPingSettings(next).catch(() => {});
  }

  if (!s) {
    return (
      <Scaffold title="Настройки пинга" onBack={() => setRoute("settings")}>
        <div style={{ color: C.muted }}>Загрузка…</div>
      </Scaffold>
    );
  }

  const isProxy = s.method === "ProxyGet" || s.method === "ProxyHead";

  return (
    <Scaffold title="Настройки пинга" onBack={() => setRoute("settings")}>
      <Eyebrow>Метод</Eyebrow>
      {METHODS.map((m) => (
        <GlassCard key={m.value} highlighted={s.method === m.value} onClick={() => update({ method: m.value })}>
          <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
            <Radio on={s.method === m.value} />
            <div style={{ flex: 1 }}>
              <b>{m.label}</b>
              <div style={{ color: C.muted, fontSize: 12 }}>{m.hint}</div>
            </div>
          </div>
        </GlassCard>
      ))}

      {/* Режим — только для прокси-методов. */}
      {isProxy && (
        <>
          <Eyebrow>Режим</Eyebrow>
          {MODES.map((m) => (
            <GlassCard key={m.value} highlighted={s.mode === m.value} onClick={() => update({ mode: m.value })}>
              <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
                <Radio on={s.mode === m.value} />
                <b>{m.label}</b>
              </div>
            </GlassCard>
          ))}
        </>
      )}

      <Eyebrow>Тест-URL</Eyebrow>
      <input value={s.test_url} onChange={(e) => update({ test_url: e.currentTarget.value })}
        style={{ width: "100%", padding: "10px 12px", borderRadius: 12, border: `1px solid ${C.stroke}`, background: C.surface, color: C.onSurface, outline: "none", boxSizing: "border-box" }} />

      <Eyebrow>Таймаут: {s.timeout_sec} с</Eyebrow>
      <input type="range" min={5} max={15} value={s.timeout_sec}
        onChange={(e) => update({ timeout_sec: Number(e.currentTarget.value) })}
        style={{ width: "100%", accentColor: C.accentBlue }} />
    </Scaffold>
  );
}

function Radio({ on }: { on: boolean }) {
  return (
    <div style={{ width: 18, height: 18, borderRadius: "50%", border: `2px solid ${on ? C.accentBlue : C.stroke}`, display: "flex", alignItems: "center", justifyContent: "center", flexShrink: 0 }}>
      {on && <div style={{ width: 8, height: 8, borderRadius: "50%", background: C.accentBlue }} />}
    </div>
  );
}

/** О приложении: автозапуск, версия, ядра, разработчик. */
export function AboutScreen() {
  const { setRoute } = useAppStore();
  const [autostart, setAuto] = useState(false);

  useEffect(() => {
    isAutostartEnabled().then(setAuto).catch(() => {});
  }, []);

  async function toggleAutostart() {
    const next = !autostart;
    try {
      await setAutostart(next);
      setAuto(next);
    } catch {
      /* оставляем прежнее значение */
    }
  }

  return (
    <Scaffold title="О приложении" onBack={() => setRoute("settings")}>
      <GlassCard onClick={toggleAutostart}>
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
          <div>
            <b>Запуск с Windows</b>
            <div style={{ color: C.muted, fontSize: 12 }}>Автозапуск в трее при входе</div>
          </div>
          <Toggle on={autostart} />
        </div>
      </GlassCard>

      <GlassCard>
        <Row label="Версия" value={APP_VERSION} />
        <Row label="Ядро Xray" value={XRAY_VERSION} />
        <Row label="Ядро Hysteria2" value={HYSTERIA_VERSION} />
        <Row label="Платформа" value="Windows · Rust + Tauri" />
      </GlassCard>
      <div style={{ color: C.mutedDim, fontSize: 12, textAlign: "center", marginTop: 8 }}>
        Infinity Labs
      </div>
    </Scaffold>
  );
}

function Toggle({ on }: { on: boolean }) {
  return (
    <div style={{ width: 44, height: 26, borderRadius: 999, background: on ? C.mint : C.stroke, position: "relative", transition: "background 160ms" }}>
      <div style={{ position: "absolute", top: 3, left: on ? 21 : 3, width: 20, height: 20, borderRadius: "50%", background: "#fff", transition: "left 160ms" }} />
    </div>
  );
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div style={{ display: "flex", justifyContent: "space-between", padding: "6px 0" }}>
      <span style={{ color: C.muted, fontSize: 13 }}>{label}</span>
      <span style={{ fontSize: 13, fontWeight: 500 }}>{value}</span>
    </div>
  );
}
