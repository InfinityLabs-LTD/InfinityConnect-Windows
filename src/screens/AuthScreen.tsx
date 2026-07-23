import { useEffect, useState } from "react";
import { discover, listenAuthResult, login, openUrl, siteAuthUrl } from "../api/commands";
import { useAppStore } from "../state/appStore";
import { InfinityColors as C, InfinityGradients as G } from "../theme/colors";
import logo from "../assets/logo.png";

/** Домен сервера зашит в приложение — пользователь его не вводит. */
const SERVER_DOMAIN = "bot.infinityconnect.ru:8443";

/** Экран входа (фирменный стиль): discovery по зашитому домену → логин → Home. */
export default function AuthScreen() {
  const { setRoute, setError, error } = useAppStore();
  const [loginValue, setLogin] = useState("");
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  /** Ждём возврата из браузера (deep-link) после «Войти через сайт». */
  const [waitingSite, setWaitingSite] = useState(false);

  // Результат входа через сайт: Rust ловит infinityconnect://auth?code=…,
  // меняет код на токены и эмитит auth://result.
  useEffect(() => {
    const un = listenAuthResult((e) => {
      setWaitingSite(false);
      if (e.ok) {
        setError(null);
        setRoute("home");
      } else {
        setError(e.error ?? "Не удалось войти через сайт");
      }
    });
    return () => {
      un.then((f) => f());
    };
  }, [setError, setRoute]);

  /** Открывает страницу входа сайта; дальше ждём deep-link-возврата. */
  async function onSiteLogin() {
    setBusy(true);
    setError(null);
    try {
      await discover(SERVER_DOMAIN);
      const url = await siteAuthUrl();
      if (!url) throw new Error("Вход через сайт недоступен");
      await openUrl(url);
      setWaitingSite(true);
    } catch (e) {
      setError(errMessage(e));
    } finally {
      setBusy(false);
    }
  }

  async function onSubmit() {
    setBusy(true);
    setError(null);
    try {
      await discover(SERVER_DOMAIN);
      await login(loginValue.trim(), password);
      setRoute("home");
    } catch (e) {
      setError(errMessage(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    // position+zIndex: экран должен быть ПОВЕРХ MeshBackground (fixed, z=0) —
    // без этого фон перекрывал форму входа и экран выглядел пустым.
    <div style={{ position: "relative", zIndex: 1, minHeight: "100vh", color: C.onSurface, fontFamily: FONT, display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", gap: 14, padding: 24 }}>
      {/* Логотип */}
      <img src={logo} alt="Infinity Connect" width={72} height={72} style={{ borderRadius: 20, boxShadow: `0 10px 40px ${C.accentBlue}55` }} />
      <h1 style={{ margin: 0, fontSize: 24, letterSpacing: -0.5 }}>Infinity Connect</h1>
      <p style={{ color: C.muted, margin: 0, marginBottom: 8 }}>Вход в аккаунт</p>

      <Field placeholder="Логин" value={loginValue} onChange={setLogin} />
      <Field placeholder="Пароль" value={password} onChange={setPassword} type="password" />

      <button onClick={onSubmit} disabled={busy}
        style={{ width: 320, padding: "12px 16px", borderRadius: 12, border: "none", background: G.accent, color: "#fff", fontWeight: 700, fontSize: 15, cursor: busy ? "default" : "pointer", opacity: busy ? 0.6 : 1, marginTop: 4 }}>
        {busy ? "Вход…" : "Войти"}
      </button>

      {/* Вход через сайт: браузер → deep-link infinityconnect://auth?code=… */}
      <button onClick={onSiteLogin} disabled={busy || waitingSite}
        style={{ width: 320, padding: "12px 16px", borderRadius: 12, border: `1px solid ${C.stroke}`, background: C.surface, color: C.onSurface, fontWeight: 600, fontSize: 14, cursor: busy || waitingSite ? "default" : "pointer", opacity: busy ? 0.6 : 1 }}>
        {waitingSite ? "Ожидание входа на сайте…" : "Войти через сайт"}
      </button>
      {waitingSite && (
        <p style={{ color: C.muted, margin: 0, maxWidth: 320, textAlign: "center", fontSize: 13 }}>
          Завершите вход в браузере — приложение продолжит автоматически.
        </p>
      )}

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
