/**
 * Стор приложения (Zustand) — зеркало Android VpnStateHolder.
 * Отражает авторизацию, ключи/серверы и состояние туннеля.
 */
import { create } from "zustand";
import type { Key, SubscriptionServer, TunnelStateEvent } from "../api/commands";

type Route = "auth" | "home";

interface AppState {
  route: Route;
  tunnel: TunnelStateEvent;
  keys: Key[];
  serversByKey: Record<number, SubscriptionServer[]>;
  error: string | null;

  setRoute: (r: Route) => void;
  setTunnel: (s: TunnelStateEvent) => void;
  setKeys: (k: Key[]) => void;
  setServers: (keyId: number, servers: SubscriptionServer[]) => void;
  setError: (e: string | null) => void;
}

export const useAppStore = create<AppState>((set) => ({
  route: "auth",
  tunnel: { status: "disconnected" },
  keys: [],
  serversByKey: {},
  error: null,

  setRoute: (route) => set({ route }),
  setTunnel: (tunnel) => set({ tunnel }),
  setKeys: (keys) => set({ keys }),
  setServers: (keyId, servers) =>
    set((s) => ({ serversByKey: { ...s.serversByKey, [keyId]: servers } })),
  setError: (error) => set({ error }),
}));
