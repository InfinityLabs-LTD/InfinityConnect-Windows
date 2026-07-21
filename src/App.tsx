import { useEffect, useState } from "react";
import { isAuthorized, onTunnelState } from "./api/commands";
import { useAppStore } from "./state/appStore";
import { InfinityColors as C } from "./theme/colors";
import AuthScreen from "./screens/AuthScreen";
import HomeScreen from "./screens/HomeScreen";

/**
 * Корень приложения (Фаза 1): восстановление сессии → роутинг Auth/Home.
 * Подписка на события состояния туннеля живёт здесь (зеркало VpnStateHolder).
 */
export default function App() {
  const { route, setRoute, setTunnel } = useAppStore();
  const [ready, setReady] = useState(false);

  useEffect(() => {
    const unlisten = onTunnelState(setTunnel);
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
      unlisten.then((u) => u());
    };
  }, [setRoute, setTunnel]);

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
