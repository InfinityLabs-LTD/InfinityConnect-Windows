import { useEffect, useState } from "react";
import { isAutostartEnabled, setAutostart } from "../api/commands";
import { useAppStore } from "../state/appStore";
import { InfinityColors as C } from "../theme/colors";
import { Scaffold } from "../components/Scaffold";
import { GlassCard } from "../components/ui";

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

/** Экран маршрутизации (наполнение — Фаза 6). */
export function RoutingScreen() {
  const { setRoute } = useAppStore();
  return (
    <Scaffold title="Маршрутизация" onBack={() => setRoute("settings")}>
      <GlassCard>
        <div style={{ color: C.muted, fontSize: 13 }}>
          Маршрутизация по сайтам (домены → Xray routing.rules) и по приложениям
          (split-tunnel через WFP) появится на Фазе 6.
        </div>
      </GlassCard>
    </Scaffold>
  );
}

/** Экран настроек пинга (наполнение — Фаза 5). */
export function PingScreen() {
  const { setRoute } = useAppStore();
  return (
    <Scaffold title="Настройки пинга" onBack={() => setRoute("settings")}>
      <GlassCard>
        <div style={{ color: C.muted, fontSize: 13 }}>
          4 метода (Прокси GET/HEAD через ядро, TCP, ICMP) + режимы
          Default/Double/Keepalive + таймаут появятся на Фазе 5.
        </div>
      </GlassCard>
    </Scaffold>
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
