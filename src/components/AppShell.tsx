/** Широкий десктоп-лейаут: сайдбар слева + анимированная область контента.
 *  Заменяет вертикальный стек полноэкранных экранов. Навигация — по главным
 *  разделам; настройки развёрнуты в плоский список (лучше для широкого окна). */
import { useEffect, useRef, useState, type ReactNode } from "react";
import { useAppStore } from "../state/appStore";
import { InfinityColors as C, InfinityGradients as G } from "../theme/colors";

type NavKey = "home" | "settings/routing" | "settings/ping" | "settings/logs" | "profile" | "settings/about";

const NAV: { key: NavKey; icon: string; label: string }[] = [
  { key: "home", icon: "⚡", label: "Подключение" },
  { key: "settings/routing", icon: "🧭", label: "Маршрутизация" },
  { key: "settings/ping", icon: "📶", label: "Пинг" },
  { key: "settings/logs", icon: "📄", label: "Логи" },
  { key: "profile", icon: "👤", label: "Профиль" },
  { key: "settings/about", icon: "ℹ️", label: "О приложении" },
];

export function AppShell({ children }: { children: ReactNode }) {
  const { route, setRoute, tunnel } = useAppStore();
  const active: NavKey = (route === "settings" ? "settings/routing" : route) as NavKey;

  // Плавный кроссфейд контента при смене раздела: короткий fade-out старого →
  // подмена → мягкий eased fade-in+slide нового. Длительности подобраны так,
  // чтобы переход ощущался плавным, а не «мигающим».
  const [display, setDisplay] = useState<ReactNode>(children);
  const [phase, setPhase] = useState<"in" | "out">("in");
  const prevRoute = useRef(route);
  useEffect(() => {
    if (prevRoute.current !== route) {
      setPhase("out");
      const t = setTimeout(() => {
        setDisplay(children);
        prevRoute.current = route;
        // Двойной rAF: даём браузеру отрисовать out-состояние нового контента
        // ДО включения in — иначе анимация fade-in проскакивает.
        requestAnimationFrame(() => requestAnimationFrame(() => setPhase("in")));
      }, 130);
      return () => clearTimeout(t);
    }
    setDisplay(children);
  }, [route, children]);

  const connected = tunnel.status === "connected";

  return (
    <div style={{ position: "relative", zIndex: 1, minHeight: "100vh", display: "flex", color: C.onSurface, fontFamily: "Segoe UI, system-ui, sans-serif" }}>
      {/* Сайдбар. */}
      <aside style={{ width: 232, flexShrink: 0, display: "flex", flexDirection: "column", padding: "22px 14px", gap: 6, borderRight: `1px solid ${C.stroke}`, background: "rgba(11,7,22,0.55)", backdropFilter: "blur(14px)" }}>
        {/* Лого. */}
        <div style={{ display: "flex", alignItems: "center", gap: 12, padding: "4px 10px 20px" }}>
          <div style={{ width: 40, height: 40, borderRadius: 12, background: G.accent, display: "flex", alignItems: "center", justifyContent: "center", fontWeight: 800, fontSize: 20, color: "#fff", boxShadow: `0 6px 24px ${C.accentBlue}55` }}>I</div>
          <div>
            <div style={{ fontWeight: 700, fontSize: 15, lineHeight: 1.1 }}>Infinity</div>
            <div style={{ color: C.muted, fontSize: 12 }}>Connect</div>
          </div>
        </div>

        {NAV.map((n) => (
          <NavItem key={n.key} icon={n.icon} label={n.label} active={active === n.key} onClick={() => setRoute(n.key)} />
        ))}

        <div style={{ flex: 1 }} />

        {/* Статус-индикатор внизу сайдбара. */}
        <div style={{ display: "flex", alignItems: "center", gap: 10, padding: "12px 12px", borderRadius: 12, background: C.surface, border: `1px solid ${C.stroke}` }}>
          <span style={{ width: 9, height: 9, borderRadius: "50%", background: connected ? C.mint : C.mutedDim, boxShadow: connected ? `0 0 10px ${C.mint}` : "none", transition: "all .3s" }} />
          <span style={{ fontSize: 13, color: connected ? C.mint : C.muted }}>{connected ? "Подключено" : "Отключено"}</span>
        </div>
      </aside>

      {/* Контент с анимацией перехода. */}
      <main style={{ flex: 1, minWidth: 0, overflowY: "auto", maxHeight: "100vh" }}>
        <div key={prevRoute.current} style={{
          opacity: phase === "in" ? 1 : 0,
          transform: phase === "in" ? "translateY(0) scale(1)" : "translateY(14px) scale(0.99)",
          transition: "opacity .34s cubic-bezier(.22,.61,.36,1), transform .34s cubic-bezier(.22,.61,.36,1)",
          willChange: "opacity, transform",
          padding: "32px 40px 40px",
          maxWidth: 1180,
          margin: "0 auto",
        }}>
          {display}
        </div>
      </main>
    </div>
  );
}

function NavItem({ icon, label, active, onClick }: { icon: string; label: string; active: boolean; onClick: () => void }) {
  return (
    <button onClick={onClick}
      style={{
        display: "flex", alignItems: "center", gap: 12, padding: "11px 14px", borderRadius: 12,
        border: "none", cursor: "pointer", textAlign: "left", width: "100%",
        background: active ? G.accent : "transparent",
        color: active ? "#fff" : C.muted,
        fontSize: 14, fontWeight: active ? 600 : 500,
        boxShadow: active ? `0 6px 20px ${C.accentIndigo}44` : "none",
        transition: "background .2s, color .2s",
      }}
      onMouseEnter={(e) => { if (!active) e.currentTarget.style.background = C.surface; }}
      onMouseLeave={(e) => { if (!active) e.currentTarget.style.background = "transparent"; }}>
      <span style={{ fontSize: 17 }}>{icon}</span>
      {label}
    </button>
  );
}
