import { useEffect, useState } from "react";
import {
  isAutostartEnabled, setAutostart,
  getPingSettings, setPingSettings,
  getRoutingSettings, setRoutingSettings, listInstalledApps,
  readCoreLogs, clearCoreLogs,
  type PingSettings, type PingMethod, type PingMode,
  type RoutingSettings, type SiteRoutingMode, type AppRoutingMode, type InstalledApp,
  type CoreLog,
} from "../api/commands";
import { useAppStore } from "../state/appStore";
import { InfinityColors as C } from "../theme/colors";
import { Scaffold } from "../components/Scaffold";
import { GlassCard, Eyebrow } from "../components/ui";
import { APP_PRESETS, type AppPreset } from "../data/appPresets";
import { checkForUpdate, downloadAndInstall } from "../api/updater";
import type { Update } from "@tauri-apps/plugin-updater";

/** Версии ядер — синхронно с fetch-binaries.ps1 / Android BuildFlags. */
const XRAY_VERSION = "26.3.27";
const HYSTERIA_VERSION = "2.10.0";
const SINGBOX_VERSION = "1.13.14";
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

/** Режимы прокси для приложений (паритет с Happ). */
const APP_MODES: { value: AppRoutingMode; label: string; hint: string }[] = [
  { value: "Off", label: "Системные настройки",
    hint: "Маршрутизация по приложениям отключена. Ко всем приложениям применяются общие настройки туннеля." },
  { value: "Disallow", label: "Прямое подключение для выбранных",
    hint: "Трафик выбранных приложений идёт в обход VPN-туннеля, напрямую в интернет. Остальные приложения по-прежнему идут через прокси." },
  { value: "Allow", label: "Прокси только для выбранных",
    hint: "Через VPN-туннель идёт только трафик выбранных приложений. Все остальные приложения подключаются напрямую." },
];

/** Экран маршрутизации: по сайтам (домены) + пометка про per-app (Фаза 7). */
export function RoutingScreen() {
  const { setRoute } = useAppStore();
  const [r, setR] = useState<RoutingSettings | null>(null);
  const [draft, setDraft] = useState("");
  const [appsDraft, setAppsDraft] = useState("");
  const [pickerOpen, setPickerOpen] = useState(false);
  const [installed, setInstalled] = useState<InstalledApp[] | null>(null);
  const [pickerLoading, setPickerLoading] = useState(false);

  function openPicker() {
    setPickerOpen(true);
    if (installed === null) {
      setPickerLoading(true);
      listInstalledApps()
        .then(setInstalled)
        .catch(() => setInstalled([]))
        .finally(() => setPickerLoading(false));
    }
  }

  /** Добавляет все exe приложения в список (Discord.exe + Update.exe и т.п.). */
  function addApp(app: InstalledApp) {
    mergeExes(app.exe_names);
  }

  /** Добавляет набор (пресет). Если приложения установлены — подтянет и их
   *  вспомогательные exe (напр. реальный Discord.exe из подпапки app-*). */
  function addPreset(preset: AppPreset) {
    let exes = [...preset.exes];
    if (installed) {
      // Автодополнение: для установленных приложений из пресета добавляем соседей.
      const presetLower = new Set(preset.exes.map((e) => e.toLowerCase()));
      for (const app of installed) {
        if (app.exe_names.some((e) => presetLower.has(e.toLowerCase()))) {
          exes.push(...app.exe_names);
        }
      }
    }
    mergeExes(exes);
  }

  /** Сливает имена exe в текущий список без дублей. */
  function mergeExes(names: string[]) {
    if (!r) return;
    const set = new Set(r.apps.map((a) => a.toLowerCase()));
    const merged = [...r.apps];
    for (const exe of names) {
      if (!set.has(exe.toLowerCase())) {
        merged.push(exe);
        set.add(exe.toLowerCase());
      }
    }
    setAppsDraft(merged.join("\n"));
    update({ apps: merged });
  }

  useEffect(() => {
    getRoutingSettings().then((rs) => {
      setR(rs);
      setDraft(rs.sites.join("\n"));
      setAppsDraft(rs.apps.join("\n"));
      // Фоновая загрузка установленных — чтобы пресеты сразу дополняли хелперами.
      if (rs.app_mode !== "Off") {
        listInstalledApps().then(setInstalled).catch(() => {});
      }
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

      <Eyebrow>Режим маршрутизации трафика</Eyebrow>
      {APP_MODES.map((m) => (
        <GlassCard key={m.value} highlighted={r.app_mode === m.value} onClick={() => update({ app_mode: m.value })}>
          <div style={{ display: "flex", alignItems: "flex-start", gap: 10 }}>
            <div style={{ marginTop: 2 }}><Radio on={r.app_mode === m.value} /></div>
            <div style={{ flex: 1 }}>
              <b>{m.label}</b>
              <div style={{ color: C.muted, fontSize: 12, marginTop: 2 }}>{m.hint}</div>
            </div>
          </div>
        </GlassCard>
      ))}

      {r.app_mode !== "Off" && (
        <>
          <Eyebrow>Готовые наборы</Eyebrow>
          <div style={{ display: "flex", flexWrap: "wrap", gap: 8 }}>
            {APP_PRESETS.map((p) => (
              <button key={p.id} onClick={() => addPreset(p)} title={p.hint}
                style={{ display: "flex", alignItems: "center", gap: 6, padding: "8px 12px", borderRadius: 999, border: `1px solid ${C.stroke}`, background: C.surface, color: C.onSurface, fontSize: 13, cursor: "pointer" }}>
                <span>{p.icon}</span>{p.name}
              </button>
            ))}
          </div>

          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginTop: 12 }}>
            <Eyebrow>Выбранные приложения</Eyebrow>
            <button onClick={openPicker}
              style={{ padding: "6px 12px", borderRadius: 10, border: `1px solid ${C.accentBlue}`, background: `${C.accentBlue}1F`, color: C.onSurface, fontSize: 13, cursor: "pointer" }}>
              + Из списка
            </button>
          </div>
          <textarea value={appsDraft} onChange={(e) => setAppsDraft(e.currentTarget.value)}
            onBlur={() => update({ apps: appsDraft.split("\n").map((s) => s.trim()).filter(Boolean) })}
            placeholder={"Discord.exe\nchrome.exe\nSpotify.exe"} rows={5}
            style={{ width: "100%", padding: "10px 12px", borderRadius: 12, border: `1px solid ${C.stroke}`, background: C.surface, color: C.onSurface, outline: "none", boxSizing: "border-box", fontFamily: "monospace", fontSize: 13, resize: "vertical" }} />
          <div style={{ color: C.mutedDim, fontSize: 12 }}>
            Достаточно имени файла (напр. «Discord.exe») — оно не меняется при
            обновлении. «Из списка» добавит и вспомогательные exe (напр. Update.exe).
          </div>
        </>
      )}

      {pickerOpen && (
        <AppPicker
          loading={pickerLoading}
          apps={installed ?? []}
          selected={new Set((r.apps ?? []).map((a) => a.toLowerCase()))}
          onPick={addApp}
          onClose={() => setPickerOpen(false)}
        />
      )}
    </Scaffold>
  );
}

/** Модалка выбора приложения из установленных. */
function AppPicker({ loading, apps, selected, onPick, onClose }: {
  loading: boolean; apps: InstalledApp[]; selected: Set<string>;
  onPick: (a: InstalledApp) => void; onClose: () => void;
}) {
  const [q, setQ] = useState("");
  const filtered = apps.filter((a) => a.name.toLowerCase().includes(q.toLowerCase()));
  return (
    <div onClick={onClose}
      style={{ position: "fixed", inset: 0, background: "rgba(0,0,0,0.6)", display: "flex", alignItems: "flex-end", justifyContent: "center", zIndex: 100 }}>
      <div onClick={(e) => e.stopPropagation()}
        style={{ width: "100%", maxHeight: "75vh", background: C.space, borderTopLeftRadius: 18, borderTopRightRadius: 18, border: `1px solid ${C.stroke}`, display: "flex", flexDirection: "column", overflow: "hidden" }}>
        <div style={{ padding: "14px 16px", display: "flex", alignItems: "center", gap: 10, borderBottom: `1px solid ${C.stroke}` }}>
          <b style={{ flex: 1 }}>Установленные приложения</b>
          <button onClick={onClose} style={{ background: "none", border: "none", color: C.muted, fontSize: 20, cursor: "pointer" }}>✕</button>
        </div>
        <input placeholder="Поиск…" value={q} onChange={(e) => setQ(e.currentTarget.value)} autoFocus
          style={{ margin: "10px 16px", padding: "10px 12px", borderRadius: 10, border: `1px solid ${C.stroke}`, background: C.surface, color: C.onSurface, outline: "none" }} />
        <div style={{ overflowY: "auto", padding: "0 12px 16px" }}>
          {loading && <div style={{ color: C.muted, padding: 16 }}>Сканирую…</div>}
          {!loading && filtered.length === 0 && <div style={{ color: C.muted, padding: 16 }}>Ничего не найдено</div>}
          {filtered.map((a) => {
            const isSel = a.exe_names.some((e) => selected.has(e.toLowerCase()));
            return (
              <div key={a.name + a.exe_names[0]} onClick={() => onPick(a)}
                style={{ display: "flex", alignItems: "center", gap: 10, padding: "10px 12px", borderRadius: 10, cursor: "pointer", background: isSel ? `${C.accentBlue}1F` : "transparent" }}>
                <div style={{ flex: 1 }}>
                  <div style={{ fontSize: 14 }}>{a.name}</div>
                  <div style={{ color: C.mutedDim, fontSize: 11, fontFamily: "monospace" }}>{a.exe_names.join(", ")}</div>
                </div>
                <span style={{ color: isSel ? C.accentBlue : C.mutedDim, fontSize: 18 }}>{isSel ? "✓" : "+"}</span>
              </div>
            );
          })}
        </div>
      </div>
    </div>
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

      <UpdateCard />

      <GlassCard>
        <Row label="Версия" value={APP_VERSION} />
        <Row label="Ядро Xray" value={XRAY_VERSION} />
        <Row label="Ядро Hysteria2" value={HYSTERIA_VERSION} />
        <Row label="Ядро sing-box" value={SINGBOX_VERSION} />
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

type UpdState =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "uptodate" }
  | { kind: "available"; version: string; notes?: string; handle: Update }
  | { kind: "downloading"; progress: number }
  | { kind: "error"; message: string };

/** Карточка обновления приложения: проверка → скачивание/установка → перезапуск. */
function UpdateCard() {
  const [st, setSt] = useState<UpdState>({ kind: "idle" });

  async function onCheck() {
    setSt({ kind: "checking" });
    try {
      const { info, handle } = await checkForUpdate();
      if (info.available && handle) {
        setSt({ kind: "available", version: info.version ?? "?", notes: info.notes, handle });
      } else {
        setSt({ kind: "uptodate" });
      }
    } catch (e) {
      setSt({ kind: "error", message: errText(e) });
    }
  }

  async function onInstall(handle: Update) {
    setSt({ kind: "downloading", progress: 0 });
    try {
      await downloadAndInstall(handle, (f) => setSt({ kind: "downloading", progress: f }));
      // relaunch произойдёт внутри — сюда обычно не доходим.
    } catch (e) {
      setSt({ kind: "error", message: errText(e) });
    }
  }

  return (
    <GlassCard>
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 12 }}>
        <div style={{ flex: 1 }}>
          <b>Обновление приложения</b>
          <div style={{ color: C.muted, fontSize: 12 }}>{updSubtitle(st)}</div>
        </div>
        {st.kind === "available" ? (
          <ActionBtn onClick={() => onInstall(st.handle)}>Установить {st.version}</ActionBtn>
        ) : st.kind === "downloading" ? (
          <span style={{ color: C.accentBlue, fontSize: 13, fontWeight: 600 }}>{Math.round(st.progress * 100)}%</span>
        ) : (
          <ActionBtn onClick={onCheck}>{st.kind === "checking" ? "Проверка…" : "Проверить"}</ActionBtn>
        )}
      </div>
      {st.kind === "available" && st.notes && (
        <pre style={{ margin: "10px 0 0", padding: 10, borderRadius: 10, background: C.space, border: `1px solid ${C.stroke}`, color: C.muted, fontSize: 12, whiteSpace: "pre-wrap", maxHeight: 120, overflow: "auto" }}>{st.notes}</pre>
      )}
      {st.kind === "downloading" && (
        <div style={{ marginTop: 10, height: 6, borderRadius: 3, background: C.stroke, overflow: "hidden" }}>
          <div style={{ height: "100%", width: `${st.progress * 100}%`, background: C.accentBlue, transition: "width .2s" }} />
        </div>
      )}
    </GlassCard>
  );
}

function updSubtitle(st: UpdState): string {
  switch (st.kind) {
    case "checking": return "Проверяем наличие новой версии…";
    case "uptodate": return "У вас последняя версия";
    case "available": return `Доступна версия ${st.version}`;
    case "downloading": return "Скачивание и установка…";
    case "error": return `Ошибка: ${st.message}`;
    default: return "Проверить наличие новой версии";
  }
}

function errText(e: unknown): string {
  if (e && typeof e === "object" && "message" in e) return String((e as { message?: string }).message ?? e);
  return String(e);
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div style={{ display: "flex", justifyContent: "space-between", padding: "6px 0" }}>
      <span style={{ color: C.muted, fontSize: 13 }}>{label}</span>
      <span style={{ fontSize: 13, fontWeight: 500 }}>{value}</span>
    </div>
  );
}

const CORE_TITLES: Record<string, string> = {
  singbox: "sing-box (TUN + split-tunnel)",
  xray: "Xray (VLESS / Reality / XHTTP)",
  hysteria: "Hysteria2 (QUIC)",
};

/** Экран логов ядер: просмотр stderr каждого ядра + копирование/очистка/обновление. */
export function LogsScreen() {
  const [logs, setLogs] = useState<CoreLog[] | null>(null);
  const [copied, setCopied] = useState<string | null>(null);

  function refresh() {
    readCoreLogs().then(setLogs).catch(() => setLogs([]));
  }
  useEffect(refresh, []);

  async function copy(core: string, content: string) {
    try {
      await navigator.clipboard.writeText(content);
      setCopied(core);
      setTimeout(() => setCopied(null), 1500);
    } catch { /* clipboard недоступен */ }
  }

  async function copyAll() {
    const all = (logs ?? []).map((l) => `===== ${l.core} =====\n${l.content}`).join("\n\n");
    try {
      await navigator.clipboard.writeText(all);
      setCopied("all");
      setTimeout(() => setCopied(null), 1500);
    } catch { /* */ }
  }

  return (
    <Scaffold title="Логи ядер" onBack={() => {}}>
      <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
        <ActionBtn onClick={refresh}>⟳ Обновить</ActionBtn>
        <ActionBtn onClick={copyAll}>{copied === "all" ? "✓ Скопировано" : "⧉ Копировать всё"}</ActionBtn>
        <ActionBtn onClick={() => clearCoreLogs().then(refresh)} danger>🗑 Очистить</ActionBtn>
      </div>

      {logs === null && <div style={{ color: C.muted }}>Загрузка…</div>}
      {logs?.map((l) => (
        <div key={l.core} style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
            <Eyebrow>{CORE_TITLES[l.core] ?? l.core}</Eyebrow>
            <button onClick={() => copy(l.core, l.content)} disabled={!l.content}
              style={{ background: "transparent", border: "none", color: l.content ? C.accentBlue : C.mutedDim, cursor: l.content ? "pointer" : "default", fontSize: 12, fontWeight: 600 }}>
              {copied === l.core ? "✓ Скопировано" : "⧉ Копировать"}
            </button>
          </div>
          <pre style={{
            margin: 0, padding: 14, borderRadius: 12, border: `1px solid ${C.stroke}`,
            background: C.space, color: l.content ? C.onSurface : C.mutedDim,
            fontSize: 12, fontFamily: "Consolas, monospace", lineHeight: 1.5,
            maxHeight: 260, overflow: "auto", whiteSpace: "pre-wrap", wordBreak: "break-word",
          }}>
            {l.content?.trim() || "— лог пуст —"}
          </pre>
        </div>
      ))}
    </Scaffold>
  );
}

function ActionBtn({ children, onClick, danger }: { children: React.ReactNode; onClick: () => void; danger?: boolean }) {
  const col = danger ? C.coral : C.accentBlue;
  return (
    <button onClick={onClick}
      style={{ padding: "8px 14px", borderRadius: 10, border: `1px solid ${col}55`, background: `${col}14`, color: col, fontSize: 13, fontWeight: 600, cursor: "pointer" }}>
      {children}
    </button>
  );
}
