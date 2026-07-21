/**
 * Типы и вызовы Tauri-команд (мост фронт↔бэк).
 *
 * Единственная точка, где фронт общается с Rust-бэкендом: через invoke()
 * (запрос→ответ) и listen() (поток событий состояния/статистики). Никакой
 * логики во фронте, кроме отображения — зеркало VpnStateHolder из Android.
 */
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/** Событие состояния туннеля (эмитится Rust `state.rs`). */
export interface TunnelStateEvent {
  status: "disconnected" | "connecting" | "connected" | "error";
  message?: string;
}

/** Событие статистики трафика (байты суммарно + скорость байт/с). */
export interface TunnelStatsEvent {
  upBytes: number;
  downBytes: number;
  upSpeed: number;
  downSpeed: number;
}

/** Ошибка бэкенда (AppError): {kind, message}. */
export interface AppError {
  kind: "Network" | "Unauthorized" | "Parse" | "Storage" | "Other";
  message?: string;
}

/** Ключ (подписка) пользователя. */
export interface Key {
  id: number;
  name?: string;
  location?: string;
  country_flag?: string;
  is_active: boolean;
  expires_at?: string;
  protocol?: string;
  is_premium: boolean;
  status?: string;
  device_limit?: number;
  devices_used?: number;
}

/** Сервер подписки для UI (стиль Happ). */
export interface SubscriptionServer {
  index: number;
  remark: string;
  address: string;
  port: number;
  protocol: string;
}

export interface DiscoveryInfo {
  api_base_url: string;
  project_name?: string;
  register_url?: string;
  support_url?: string;
}

export interface UserInfo {
  username?: string;
  email?: string;
  is_subscription_active: boolean;
  subscription_expires_at?: string;
  plan_name?: string;
}

// ── Команды (Фаза 0/1) ──

export const ping = (name: string) => invoke<string>("ping", { name });

export const discover = (domain: string) =>
  invoke<DiscoveryInfo>("discover", { domain });

export const login = (login: string, password: string) =>
  invoke<void>("login", { login, password });

export const logout = () => invoke<void>("logout");

export const isAuthorized = () => invoke<boolean>("is_authorized");

export const userInfo = () => invoke<UserInfo>("user_info");

export const keys = () => invoke<Key[]>("keys");

export const keyServers = (keyId: number) =>
  invoke<SubscriptionServer[]>("key_servers", { keyId });

export const connect = (keyId: number, serverIndex: number) =>
  invoke<void>("connect", { keyId, serverIndex });

export const disconnect = () => invoke<void>("disconnect");

export const tunnelStatus = () => invoke<boolean>("tunnel_status");

export const isAutostartEnabled = () => invoke<boolean>("is_autostart_enabled");

export const setAutostart = (enabled: boolean) =>
  invoke<void>("set_autostart", { enabled });

/** Подписка на события состояния туннеля от бэкенда. */
export async function onTunnelState(
  handler: (state: TunnelStateEvent) => void,
): Promise<UnlistenFn> {
  return listen<TunnelStateEvent>("tunnel://state", (e) => handler(e.payload));
}

/** Подписка на события статистики трафика. */
export async function onTunnelStats(
  handler: (stats: TunnelStatsEvent) => void,
): Promise<UnlistenFn> {
  return listen<TunnelStatsEvent>("tunnel://stats", (e) => handler(e.payload));
}
