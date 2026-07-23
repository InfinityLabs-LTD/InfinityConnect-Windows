import { useEffect, useState } from "react";
import { userInfo, logout, type UserInfo } from "../api/commands";
import { useAppStore } from "../state/appStore";
import { InfinityColors as C } from "../theme/colors";
import { Scaffold } from "../components/Scaffold";
import { GlassCard } from "../components/ui";

/** Экран аккаунта (Фаза 4): данные пользователя, подписка, разлогин. */
export default function ProfileScreen() {
  const { setRoute } = useAppStore();
  const [info, setInfo] = useState<UserInfo | null>(null);
  const [err, setErr] = useState<string | null>(null);

  useEffect(() => {
    userInfo().then(setInfo).catch((e) => setErr(errMessage(e)));
  }, []);

  async function onLogout() {
    // Даже если серверный logout не удался (офлайн), локально выходим:
    // чистим состояние и возвращаемся на экран входа.
    try {
      await logout();
    } catch {
      /* игнорируем — токены всё равно чистятся на бэке, уходим на auth */
    }
    const s = useAppStore.getState();
    s.setKeys([]);
    s.setSelection(null);
    s.setError(null);
    setRoute("auth");
  }

  return (
    <Scaffold title="Аккаунт" onBack={() => setRoute("home")}>
      {err && <div style={{ color: C.coral, fontSize: 13 }}>{err}</div>}

      <GlassCard>
        <Row label="Пользователь" value={info?.username ?? "—"} />
        <Row label="Email" value={info?.email ?? "—"} />
      </GlassCard>

      <GlassCard>
        <Row label="Подписка" value={info?.is_subscription_active ? "Активна" : "Неактивна"}
          valueColor={info?.is_subscription_active ? C.mint : C.mutedDim} />
        {info?.plan_name && <Row label="План" value={info.plan_name} />}
        {info?.subscription_expires_at && <Row label="Действует до" value={info.subscription_expires_at.slice(0, 10)} />}
      </GlassCard>

      <button onClick={onLogout}
        style={{ marginTop: 8, padding: "12px 16px", borderRadius: 12, border: `1px solid ${C.coral}55`, background: `${C.coral}1A`, color: C.coral, fontWeight: 600, cursor: "pointer" }}>
        Выйти из аккаунта
      </button>
    </Scaffold>
  );
}

function Row({ label, value, valueColor }: { label: string; value: string; valueColor?: string }) {
  return (
    <div style={{ display: "flex", justifyContent: "space-between", padding: "6px 0" }}>
      <span style={{ color: C.muted, fontSize: 13 }}>{label}</span>
      <span style={{ color: valueColor ?? C.onSurface, fontSize: 13, fontWeight: 500 }}>{value}</span>
    </div>
  );
}

function errMessage(e: unknown): string {
  if (e && typeof e === "object" && "message" in e) return String((e as { message?: string }).message ?? "Ошибка");
  return String(e);
}
