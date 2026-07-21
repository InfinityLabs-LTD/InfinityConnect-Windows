/**
 * Стор приложения (Zustand) — зеркало Android VpnStateHolder.
 * Отражает состояние туннеля/статистику, приходящие Tauri-событиями.
 * На Фазе 0 хранит лишь статус и результат тестового ping.
 */
import { create } from "zustand";
import type { TunnelStateEvent } from "../api/commands";

interface AppState {
  tunnel: TunnelStateEvent;
  lastPingReply: string | null;
  setTunnel: (s: TunnelStateEvent) => void;
  setPingReply: (reply: string) => void;
}

export const useAppStore = create<AppState>((set) => ({
  tunnel: { status: "disconnected" },
  lastPingReply: null,
  setTunnel: (tunnel) => set({ tunnel }),
  setPingReply: (lastPingReply) => set({ lastPingReply }),
}));
