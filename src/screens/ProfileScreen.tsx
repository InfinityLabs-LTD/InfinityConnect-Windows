import { useEffect, useState } from "react";
import {
  userInfo, subscriptionInfo, supportUrl, openUrl, logout, keys as fetchKeys,
  type UserInfo, type SubscriptionInfo, type Key,
} from "../api/commands";
import { useAppStore } from "../state/appStore";
import { InfinityColors as C, InfinityGradients as G } from "../theme/colors";
import { Scaffold } from "../components/Scaffold";

/**
 * Экран профиля — зеркало Android ProfileScreen: hero (аватар-инициалы, логин,
 * бейдж подписки), карточка подписки с цветной гранью и метриками
 * (ключи/месяцы/потрачено), карточка «Аккаунт», поддержка и выход.
 */
export default function ProfileScreen() {
  const { setRoute, keys: cachedKeys } = useAppStore();
  const [user, setUser] = useState<UserInfo | null>(null);
  const [sub, setSub] = useState<SubscriptionInfo | null>(null);
  const [support, setSupport] = useState<string | null>(null);
  const [err, setErr] = useState<string | null>(null);

  useEffect(() => {
    userInfo().then(setUser).catch((e) => setErr(errMessage(e)));
    subscriptionInfo().then(setSub).catch(() => {/* метрики опциональны */});
    supportUrl().then(setSupport).catch(() => {});
    // Ключи для списка сроков: если стор пуст (профиль открыт до Home) — дозагружаем.
    if (useAppStore.getState().keys.length === 0) {
      fetchKeys().then((ks) => useAppStore.getState().setKeys(ks)).catch(() => {});
    }
  }, []);

  async function onLogout() {
    // Даже если серверный logout не удался (офлайн), локально выходим:
    // чистим состояние и возвращаемся на экран входа.
    try {
      await logout();
    } catch {
      /* токены чистятся на бэке в любом случае */
    }
    const s = useAppStore.getState();
    s.setKeys([]);
    s.setSelection(null);
    s.setError(null);
    setRoute("auth");
  }

  const active = user?.is_subscription_active === true;
  const plan = planLabel(cachedKeys, user);

  return (
    <Scaffold title="Профиль" onBack={() => setRoute("home")}>
      <div style={{ maxWidth: 480, width: "100%", display: "flex", flexDirection: "column", gap: 16 }}>
      {err && <div style={{ color: C.coral, fontSize: 13 }}>{err}</div>}

      {/* ── Hero: аватар-инициалы, логин, бейдж подписки ── */}
      <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 10, padding: "8px 0 4px" }}>
        <div style={{
          width: 88, height: 88, borderRadius: "50%", background: G.accent,
          border: `1px solid ${C.stroke}`, display: "flex", alignItems: "center", justifyContent: "center",
          fontSize: 28, fontWeight: 700, color: "#fff",
        }}>
          {initials(user?.username)}
        </div>
        <b style={{ fontSize: 20 }}>{user?.username ?? "—"}</b>
        <span style={{
          fontSize: 12, fontWeight: 600, padding: "4px 12px", borderRadius: 999,
          color: active ? C.mint : C.muted,
          border: `1px solid ${active ? C.mint : C.muted}55`,
          background: `${active ? C.mint : C.muted}14`,
        }}>
          {active ? "Подписка активна" : "Подписка неактивна"}
        </span>
      </div>

      {/* ── Подписка: цветная левая грань + срок + сроки по ключам + метрики ── */}
      <SubscriptionCard active={active} sub={sub} keys={cachedKeys} />

      {/* ── Аккаунт ── */}
      <div style={{ background: C.surface, border: `1px solid ${C.stroke}`, borderRadius: 20, padding: 18 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 10 }}>
          <span style={{ fontSize: 14 }}>@</span>
          <span style={eyebrow}>АККАУНТ</span>
        </div>
        <InfoRow icon="👑" label="Логин" value={user?.username ?? "—"} />
        {user?.email && looksLikeEmail(user.email) && (
          <>
            <Hr />
            <InfoRow icon="@" label="E-mail" value={user.email} />
          </>
        )}
        {plan && (
          <>
            <Hr />
            <InfoRow icon="🔑" label="Тариф" value={plan} />
          </>
        )}
      </div>

      {/* ── Кнопки ── */}
      {support && (
        <button onClick={() => openUrl(support).catch(() => {})} style={outlineBtn(C.accentBlue)}>
          🎧 Написать в поддержку
        </button>
      )}
      <button onClick={onLogout} style={outlineBtn(C.coral)}>
        ⎋ Выйти
      </button>
      </div>
    </Scaffold>
  );
}

/** Карточка подписки: статусная грань слева, «Осталось N дней», срок по
 *  каждому ключу (🌐/👑 + имя + дни/дата), плитки метрик. */
function SubscriptionCard({ active, sub, keys }: { active: boolean; sub: SubscriptionInfo | null; keys: Key[] }) {
  const accent = active ? C.mint : C.muted;
  const expiresAt = sub?.earliest_expiry;
  const remaining = daysUntil(expiresAt);

  const title = !active
    ? "Неактивна"
    : remaining != null && remaining >= 0
      ? `Осталось ${remaining} ${plural(remaining, "день", "дня", "дней")}`
      : "Активна";

  const activeKeys = keys.filter((k) => k.is_active && k.expires_at);

  return (
    <div style={{ display: "flex", background: C.surface, border: `1px solid ${C.stroke}`, borderRadius: 20, overflow: "hidden" }}>
      {/* Цветная левая грань — маркер статуса. */}
      <div style={{ width: 4, flexShrink: 0, background: accent }} />
      <div style={{ flex: 1, padding: 18 }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <span style={eyebrow}>ПОДПИСКА</span>
          <span style={{ color: accent, fontSize: 16 }}>📅</span>
        </div>
        <div style={{ fontSize: 20, fontWeight: 700, marginTop: 10 }}>{title}</div>
        {expiresAt && (
          <div style={{ color: C.muted, fontSize: 13, marginTop: 2 }}>до {formatDate(expiresAt)}</div>
        )}

        {/* Срок по каждому активному ключу. */}
        {activeKeys.length > 0 && (
          <>
            <div style={{ height: 1, background: C.stroke, margin: "14px 0 4px" }} />
            {activeKeys.map((k) => (
              <KeyExpiryRow key={k.id} k={k} />
            ))}
          </>
        )}

        <div style={{ height: 1, background: C.stroke, margin: "10px 0 12px" }} />
        <div style={{ display: "flex", gap: 12 }}>
          {sub != null && <MetricTile label="Ключей" value={String(sub.keys_count)} />}
          {sub?.total_months != null && sub.total_months > 0 && (
            <MetricTile label="Месяцев" value={String(sub.total_months)} />
          )}
          {sub?.total_spent != null && sub.total_spent > 0 && (
            <MetricTile label="Потрачено" value={formatMoney(sub.total_spent)} />
          )}
        </div>
      </div>
    </div>
  );
}

/** Строка ключа: 🌐/👑 + имя слева, «N дней / до даты» справа. */
function KeyExpiryRow({ k }: { k: Key }) {
  const days = daysUntil(k.expires_at);
  const name = keyDisplayName(k.name);
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 10, padding: "8px 0" }}>
      <span style={{ fontSize: 15 }}>{k.is_premium ? "👑" : "🌐"}</span>
      <span style={{ flex: 1, fontSize: 13.5, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
        {name}
      </span>
      <div style={{ display: "flex", flexDirection: "column", alignItems: "flex-end", gap: 1 }}>
        <b style={{ fontSize: 13.5 }}>
          {days != null ? `${days} ${plural(days, "день", "дня", "дней")}` : "—"}
        </b>
        {k.expires_at && (
          <span style={{ color: C.muted, fontSize: 11.5 }}>до {formatDate(k.expires_at)}</span>
        )}
      </div>
    </div>
  );
}

/** Имя ключа без технического суффикса @bot.local (как на Home). */
function keyDisplayName(name?: string): string {
  const n = (name ?? "").trim().replace(/@[\w.-]+$/, "").trim();
  return n || "Ключ";
}

function MetricTile({ label, value }: { label: string; value: string }) {
  return (
    <div style={{ flex: 1, background: C.spaceElevated, borderRadius: 14, padding: "12px 8px", display: "flex", flexDirection: "column", alignItems: "center", gap: 2 }}>
      <b style={{ fontSize: 15 }}>{value}</b>
      <span style={{ color: C.muted, fontSize: 11 }}>{label}</span>
    </div>
  );
}

function InfoRow({ icon, label, value }: { icon: string; label: string; value: string }) {
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 12, padding: "9px 0" }}>
      <span style={{ fontSize: 14, width: 18, textAlign: "center", opacity: 0.7 }}>{icon}</span>
      <span style={{ color: C.muted, fontSize: 14, flex: 1 }}>{label}</span>
      <span style={{ fontSize: 14, fontWeight: 500 }}>{value}</span>
    </div>
  );
}

function Hr() {
  return <div style={{ height: 1, background: C.stroke }} />;
}

const eyebrow: React.CSSProperties = {
  fontSize: 11, letterSpacing: "0.1em", color: C.muted, fontWeight: 600,
};

function outlineBtn(color: string): React.CSSProperties {
  return {
    width: "100%", padding: "14px 16px", borderRadius: 16,
    border: `1px solid ${color}55`, background: "transparent",
    color, fontWeight: 600, fontSize: 14, cursor: "pointer",
  };
}

// ── Хелперы (зеркало Android) ──

/** Инициалы: два слова → первые буквы; одно → первые 2 символа. */
function initials(name?: string): string {
  const n = (name ?? "").trim();
  if (!n) return "?";
  const parts = n.split(/[\s\-_]+/).filter(Boolean);
  if (parts.length >= 2) return (parts[0][0] + parts[1][0]).toUpperCase();
  return n.slice(0, 2).toUpperCase();
}

/** Тариф по составу активных ключей; фолбэк — plan_name сервера. */
function planLabel(keys: Key[], user: UserInfo | null): string | null {
  const active = keys.filter((k) => k.is_active);
  const hasBase = active.some((k) => !k.is_premium);
  const hasPremium = active.some((k) => k.is_premium);
  if (hasBase && hasPremium) return "Базовый + Премиум";
  if (hasPremium) return "Премиум";
  if (hasBase) return "Базовый";
  return user?.plan_name ?? null;
}

function looksLikeEmail(s: string): boolean {
  const t = s.trim();
  const at = t.indexOf("@");
  return at > 0 && t.indexOf(".", at) > at + 1;
}

/** Дней до даты ISO (UTC-полночь); null — нет даты/не парсится. */
function daysUntil(iso?: string): number | null {
  if (!iso) return null;
  const target = new Date(iso);
  if (isNaN(target.getTime())) return null;
  const ms = target.getTime() - Date.now();
  return Math.max(0, Math.ceil(ms / 86_400_000));
}

/** ISO → «ДД.ММ.ГГГГ». */
function formatDate(iso: string): string {
  const d = new Date(iso);
  if (isNaN(d.getTime())) return iso.slice(0, 10);
  const p = (n: number) => String(n).padStart(2, "0");
  return `${p(d.getDate())}.${p(d.getMonth() + 1)}.${d.getFullYear()}`;
}

function plural(n: number, one: string, few: string, many: string): string {
  const m10 = n % 10;
  const m100 = n % 100;
  if (m10 === 1 && m100 !== 11) return one;
  if (m10 >= 2 && m10 <= 4 && (m100 < 12 || m100 > 14)) return few;
  return many;
}

function formatMoney(v: number): string {
  const s = v % 1 === 0 ? String(v) : v.toFixed(2).replace(".", ",");
  return `${s} ₽`;
}

function errMessage(e: unknown): string {
  if (e && typeof e === "object" && "message" in e) return String((e as { message?: string }).message ?? "Ошибка");
  return String(e);
}
