/**
 * Типы и вызовы Tauri-команд (мост фронт↔бэк).
 *
 * Единственная точка, где фронт общается с Rust-бэкендом: через invoke()
 * (запрос→ответ) и listen() (поток событий состояния/статистики). Никакой
 * логики во фронте, кроме отображения — это зеркало VpnStateHolder из Android.
 */
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/** Событие состояния туннеля (эмитится Rust `state.rs`). Заглушка Фазы 0. */
export interface TunnelStateEvent {
  status: "disconnected" | "connecting" | "connected" | "error";
  message?: string;
}

/** Тестовая команда Фазы 0 — проверка моста invoke end-to-end. */
export async function ping(name: string): Promise<string> {
  return invoke<string>("ping", { name });
}

/** Подписка на события состояния туннеля от бэкенда. */
export async function onTunnelState(
  handler: (state: TunnelStateEvent) => void,
): Promise<UnlistenFn> {
  return listen<TunnelStateEvent>("tunnel://state", (e) => handler(e.payload));
}
