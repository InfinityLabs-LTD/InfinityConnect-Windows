/**
 * Статус ключа и доступность подключения — зеркало Android `VpnKey.status()`.
 * Не-ACTIVE ключи (истёк/отключён/лимит) блокируются: их серверы нельзя
 * выбрать и подключить, пинг по ним не гоняем.
 */
import type { Key } from "../api/commands";

export type KeyStatus = "ACTIVE" | "EXPIRED" | "DISABLED" | "LIMITED";

/** Достигнут лимит устройств (данные есть и исчерпаны). */
export function devicesExhausted(k: Key): boolean {
  return (k.device_limit ?? 0) > 0 && (k.devices_used ?? 0) >= (k.device_limit ?? 0);
}

/** Истёк ли срок ключа (expires_at в прошлом). */
export function isExpired(k: Key): boolean {
  if (!k.expires_at) return false;
  const t = new Date(k.expires_at).getTime();
  return !isNaN(t) && t < Date.now();
}

/**
 * Статус ключа: приоритет — статус сервера (Remnawave); без него выводим на
 * клиенте: истёк → EXPIRED; неактивен → DISABLED; лимит устройств → LIMITED.
 */
export function keyStatus(k: Key): KeyStatus {
  const raw = (k.status ?? "").trim().toUpperCase();
  if (raw === "ACTIVE" || raw === "EXPIRED" || raw === "DISABLED" || raw === "LIMITED") {
    return raw as KeyStatus;
  }
  if (isExpired(k)) return "EXPIRED";
  if (!k.is_active) return "DISABLED";
  if (devicesExhausted(k)) return "LIMITED";
  return "ACTIVE";
}

/** Ключ заблокирован для подключения (любой статус, кроме ACTIVE). */
export function isKeyBlocked(k: Key): boolean {
  return keyStatus(k) !== "ACTIVE";
}

/** Причина недоступности — текст для пользователя. */
export function blockedReason(k: Key): string {
  switch (keyStatus(k)) {
    case "EXPIRED": return "Срок подписки истёк";
    case "DISABLED": return "Подписка отключена";
    case "LIMITED":
      return devicesExhausted(k)
        ? "Достигнут лимит устройств этой подписки"
        : "Достигнут лимит этой подписки";
    default: return "Подписка недоступна";
  }
}
