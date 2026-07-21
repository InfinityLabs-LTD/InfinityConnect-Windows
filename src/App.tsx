import { useEffect, useState } from "react";
import { isAuthorized, onTunnelState, onTunnelStats, tunnelStatus } from "./api/commands";
import { useAppStore } from "./state/appStore";
import { InfinityColors as C, InfinityGradients as G } from "./theme/colors";
import AuthScreen from "./screens/AuthScreen";
import HomeScreen from "./screens/HomeScreen";
import ProfileScreen from "./screens/ProfileScreen";
import { SettingsHub, RoutingScreen, PingScreen, AboutScreen } from "./screens/SettingsScreens";

/**
 * Корень приложения: восстановление сессии → роутинг. Подписки на события
 * туннеля/статистики живут здесь (зеркало VpnStateHolder).
 */
export default function App() {
  const { route, setRoute, setTunnel, setStats } = useAppStore();
  const [ready, setReady] = useState(false);

  useEffect(() => {
    const unlistenState = onTunnelState(setTunnel);
    const unlistenStats = onTunnelStats(setStats);
    (async () => {
      try {
        if (await isAuthorized()) {
          setRoute("home");
          // Восстанавливаем состояние туннеля (мог остаться подключённым).
          if (await tunnelStatus()) setTunnel({ status: "connected" });
        }
      } catch {
        /* нет сессии — остаёмся на auth */
      } finally {
        setReady(true);
      }
    })();
    return () => {
      unlistenState.then((u) => u());
      unlistenStats.then((u) => u());
    };
  }, [setRoute, setTunnel, setStats]);

  if (!ready) {
    return (
      <div style={{ minHeight: "100vh", background: G.screen, display: "flex", alignItems: "center", justifyContent: "center", fontFamily: "Segoe UI, system-ui, sans-serif" }}>
        <span style={{ color: C.accentBlue }}>Infinity Connect…</span>
      </div>
    );
  }

  switch (route) {
    case "home": return <HomeScreen />;
    case "profile": return <ProfileScreen />;
    case "settings": return <SettingsHub />;
    case "settings/routing": return <RoutingScreen />;
    case "settings/ping": return <PingScreen />;
    case "settings/about": return <AboutScreen />;
    default: return <AuthScreen />;
  }
}
