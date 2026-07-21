import { useState } from "react";
import { discover, login } from "../api/commands";
import { useAppStore } from "../state/appStore";
import { InfinityColors as C, InfinityGradients as G } from "../theme/colors";

/** Экран входа (фирменный стиль): discovery по домену → логин → Home. */
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
    <div style={{ minHeight: "100vh", background: G.screen, color: C.onSurface, fontFamily: FONT, display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", gap: 14, padding: 24 }}>
      {/* Логотип-глиф */}
      <div style={{ width: 72, height: 72, borderRadius: 20, background: G.accent, display: "flex", alignItems: "center", justifyContent: "center", fontSize: 36, fontWeight: 800, color: "#fff", boxShadow: `0 10px 40px ${C.accentBlue}55` }}>
        I
      </div>
      <h1 style={{ margin: 0, fontSize: 24, letterSpacing: -0.5 }}>Infinity Connect</h1>
      <p style={{ color: C.muted, margin: 0, marginBottom: 8 }}>Вход в аккаунт</p>

      <Field placeholder="Домен (например vpn.example.com)" value={domain} onChange={setDomain} />
      <Field placeholder="Логин" value={loginValue} onChange={setLogin} />
      <Field placeholder="Пароль" value={password} onChange={setPassword} type="password" />

      <button onClick={onSubmit} disabled={busy}
        style={{ width: 320, padding: "12px 16px", borderRadius: 12, border: "none", background: G.accent, color: "#fff", fontWeight: 700, fontSize: 15, cursor: busy ? "default" : "pointer", opacity: busy ? 0.6 : 1, marginTop: 4 }}>
        {busy ? "Вход…" : "Войти"}
      </button>

      {error && <p style={{ color: C.coral, margin: 0, maxWidth: 320, textAlign: "center" }}>{error}</p>}
    </div>
  );
}

function Field({ placeholder, value, onChange, type = "text" }: { placeholder: string; value: string; onChange: (v: string) => void; type?: string }) {
  return (
    <input placeholder={placeholder} value={value} type={type}
      onChange={(e) => onChange(e.currentTarget.value)}
      style={{ width: 320, padding: "12px 14px", borderRadius: 12, border: `1px solid ${C.stroke}`, background: C.surface, color: C.onSurface, outline: "none", fontSize: 14 }} />
  );
}

const FONT = "Segoe UI, system-ui, sans-serif";
function errMessage(e: unknown): string {
  if (e && typeof e === "object" && "message" in e) return String((e as { message?: string }).message ?? "Ошибка");
  return String(e);
}
