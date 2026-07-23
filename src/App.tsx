import { useEffect, useState } from "react";
import { isAuthorized, onTunnelState, onTunnelStats, tunnelStatus } from "./api/commands";
import { useAppStore } from "./state/appStore";
import { InfinityColors as C } from "./theme/colors";
import { MeshBackground } from "./components/MeshBackground";
import { AppShell } from "./components/AppShell";
import { UpdateBanner } from "./components/UpdateBanner";
import { ErrorBoundary } from "./components/ErrorBoundary";
import AuthScreen from "./screens/AuthScreen";
import HomeScreen from "./screens/HomeScreen";
import ProfileScreen from "./screens/ProfileScreen";
import { RoutingScreen, PingScreen, AboutScreen, LogsScreen } from "./screens/SettingsScreens";

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
      <>
        <MeshBackground />
        <div style={{ position: "relative", zIndex: 1, minHeight: "100vh", display: "flex", alignItems: "center", justifyContent: "center", fontFamily: "Segoe UI, system-ui, sans-serif" }}>
          <span style={{ color: C.accentBlue }}>Infinity Connect…</span>
        </div>
      </>
    );
  }

  // До логина — полноэкранный экран входа (без сайдбара).
  if (route === "auth") {
    return (
      <>
        <MeshBackground />
        <ErrorBoundary>
          <AuthScreen />
        </ErrorBoundary>
      </>
    );
  }

  // После логина — широкий лейаут: сайдбар + контент + баннер обновления.
  return (
    <>
      <MeshBackground />
      <AppShell>
        <ErrorBoundary>{renderContent(route)}</ErrorBoundary>
      </AppShell>
      <UpdateBanner />
    </>
  );
}

function renderContent(route: string) {
  switch (route) {
    case "profile": return <ProfileScreen />;
    case "settings":
    case "settings/routing": return <RoutingScreen />;
    case "settings/ping": return <PingScreen />;
    case "settings/logs": return <LogsScreen />;
    case "settings/about": return <AboutScreen />;
    default: return <HomeScreen />;
  }
}
