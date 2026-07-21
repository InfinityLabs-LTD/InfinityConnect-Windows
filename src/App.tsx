import { useEffect, useState } from "react";
import { isAuthorized, onTunnelState, onTunnelStats } from "./api/commands";
import { useAppStore } from "./state/appStore";
import { InfinityColors as C } from "./theme/colors";
import AuthScreen from "./screens/AuthScreen";
import HomeScreen from "./screens/HomeScreen";

/**
 * Корень приложения: восстановление сессии → роутинг Auth/Home.
 * Подписки на события туннеля/статистики живут здесь (зеркало VpnStateHolder).
 */
export default function App() {
  const { route, setRoute, setTunnel, setStats } = useAppStore();
  const [ready, setReady] = useState(false);

  useEffect(() => {
    const unlistenState = onTunnelState(setTunnel);
    const unlistenStats = onTunnelStats(setStats);
    (async () => {
      try {
        if (await isAuthorized()) setRoute("home");
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
      <div style={splash}>
        <span style={{ color: C.primaryLight }}>Infinity Connect…</span>
      </div>
    );
  }

  return route === "home" ? <HomeScreen /> : <AuthScreen />;
}

const splash: React.CSSProperties = {
  minHeight: "100vh", background: C.background,
  display: "flex", alignItems: "center", justifyContent: "center",
  fontFamily: "Segoe UI, system-ui, sans-serif",
};
