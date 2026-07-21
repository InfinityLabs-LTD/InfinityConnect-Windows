/**
 * Стор приложения (Zustand) — зеркало Android VpnStateHolder.
 * Отражает авторизацию, ключи/серверы и состояние туннеля.
 */
import { create } from "zustand";
import type {
  Key,
  SubscriptionServer,
  TunnelStateEvent,
  TunnelStatsEvent,
} from "../api/commands";

type Route =
  | "auth"
  | "home"
  | "profile"
  | "settings"
  | "settings/routing"
  | "settings/ping"
  | "settings/about";

/** Выбранный сервер (ключ + индекс в подписке). */
export interface Selection {
  keyId: number;
  serverIndex: number;
}

interface AppState {
  route: Route;
  tunnel: TunnelStateEvent;
  stats: TunnelStatsEvent | null;
  keys: Key[];
  serversByKey: Record<number, SubscriptionServer[]>;
  selection: Selection | null;
  error: string | null;

  setRoute: (r: Route) => void;
  setTunnel: (s: TunnelStateEvent) => void;
  setStats: (s: TunnelStatsEvent) => void;
  setKeys: (k: Key[]) => void;
  setServers: (keyId: number, servers: SubscriptionServer[]) => void;
  setSelection: (s: Selection) => void;
  setError: (e: string | null) => void;
}

export const useAppStore = create<AppState>((set) => ({
  route: "auth",
  tunnel: { status: "disconnected" },
  stats: null,
  keys: [],
  serversByKey: {},
  selection: null,
  error: null,

  setRoute: (route) => set({ route }),
  setTunnel: (tunnel) =>
    set((s) => ({
      tunnel,
      // При отключении сбрасываем статистику.
      stats: tunnel.status === "connected" ? s.stats : null,
    })),
  setStats: (stats) => set({ stats }),
  setKeys: (keys) => set({ keys }),
  setServers: (keyId, servers) =>
    set((s) => ({ serversByKey: { ...s.serversByKey, [keyId]: servers } })),
  setSelection: (selection) => set({ selection }),
  setError: (error) => set({ error }),
}));
