import { useState } from "react";
import { discover, login } from "../api/commands";
import { useAppStore } from "../state/appStore";
import { InfinityColors as C } from "../theme/colors";

/**
 * Экран входа (Фаза 1). Discovery по домену → логин → переход на Home.
 * Домен по умолчанию можно поменять; сохранять его в настройках — Фаза 4.
 */
export default function AuthScreen() {
  const { setRoute, setError, error } = useAppStore();
  const [domain, setDomain] = useState("");
  const [loginValue, setLogin] = useState("");
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);

  async function onSubmit() {
    setBusy(true);
    setError(null);
    try {
      if (domain.trim()) await discover(domain.trim());
      await login(loginValue.trim(), password);
      setRoute("home");
    } catch (e) {
      setError(errMessage(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div style={wrap}>
      <h1 style={{ color: C.primaryLight, margin: 0 }}>Infinity Connect</h1>
      <p style={{ color: C.textSecondary, marginTop: 4 }}>Вход в аккаунт</p>

      <input style={input} placeholder="Домен (например vpn.example.com)"
        value={domain} onChange={(e) => setDomain(e.currentTarget.value)} />
      <input style={input} placeholder="Логин"
        value={loginValue} onChange={(e) => setLogin(e.currentTarget.value)} />
      <input style={input} type="password" placeholder="Пароль"
        value={password} onChange={(e) => setPassword(e.currentTarget.value)} />

      <button style={{ ...button, opacity: busy ? 0.6 : 1 }} disabled={busy} onClick={onSubmit}>
        {busy ? "Вход…" : "Войти"}
      </button>

      {error && <p style={{ color: C.error, margin: 0 }}>{error}</p>}
    </div>
  );
}

function errMessage(e: unknown): string {
  if (e && typeof e === "object" && "message" in e) {
    return String((e as { message?: string }).message ?? "Ошибка");
  }
  return String(e);
}

const wrap: React.CSSProperties = {
  minHeight: "100vh", background: C.background, color: C.textPrimary,
  fontFamily: "Segoe UI, system-ui, sans-serif", display: "flex",
  flexDirection: "column", alignItems: "center", justifyContent: "center",
  gap: 12, padding: 24,
};
const input: React.CSSProperties = {
  width: 300, padding: "10px 12px", borderRadius: 8,
  border: `1px solid ${C.surfaceElevated}`, background: C.surface,
  color: C.textPrimary, outline: "none",
};
const button: React.CSSProperties = {
  width: 300, padding: "10px 16px", borderRadius: 8, border: "none",
  background: C.primary, color: "#fff", cursor: "pointer", fontWeight: 600, marginTop: 4,
};
