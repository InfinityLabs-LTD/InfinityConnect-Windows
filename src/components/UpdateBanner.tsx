/** Баннер авто-обновления: при запуске тихо проверяет наличие новой версии и,
 *  если есть, предлагает обновиться с описанием изменений (release notes).
 *  Не мешает работе — можно отложить («Позже»). */
import { useEffect, useState } from "react";
import { checkForUpdate, downloadAndInstall } from "../api/updater";
import type { Update } from "@tauri-apps/plugin-updater";
import { InfinityColors as C, InfinityGradients as G } from "../theme/colors";

type State =
  | { kind: "hidden" }
  | { kind: "available"; version: string; notes?: string; handle: Update }
  | { kind: "downloading"; progress: number; version: string }
  | { kind: "error"; message: string };

export function UpdateBanner() {
  const [st, setSt] = useState<State>({ kind: "hidden" });

  // Тихая авто-проверка через ~4с после старта (не блокируем загрузку UI).
  useEffect(() => {
    const t = setTimeout(async () => {
      try {
        const { info, handle } = await checkForUpdate();
        if (info.available && handle) {
          setSt({ kind: "available", version: info.version ?? "?", notes: info.notes, handle });
        }
      } catch {
        /* нет обновлений / endpoint не настроен — молчим, это фон */
      }
    }, 4000);
    return () => clearTimeout(t);
  }, []);

  if (st.kind === "hidden") return null;

  async function onUpdate(handle: Update, version: string) {
    setSt({ kind: "downloading", progress: 0, version });
    try {
      await downloadAndInstall(handle, (f) => setSt({ kind: "downloading", progress: f, version }));
    } catch (e) {
      setSt({ kind: "error", message: errText(e) });
    }
  }

  return (
    <div style={{
      position: "fixed", right: 24, bottom: 24, width: 380, maxWidth: "calc(100vw - 48px)",
      zIndex: 200, borderRadius: 18, overflow: "hidden",
      border: `1px solid ${C.accentBlue}55`, background: "rgba(28,19,56,0.96)",
      backdropFilter: "blur(16px)", boxShadow: `0 16px 50px rgba(0,0,0,0.5)`,
      animation: "upd-slide-in .35s cubic-bezier(.22,.61,.36,1)",
    }}>
      {/* Акцентная полоса сверху. */}
      <div style={{ height: 4, background: G.accent }} />

      <div style={{ padding: 18 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 8 }}>
          <span style={{ fontSize: 22 }}>🚀</span>
          <div style={{ flex: 1 }}>
            <b style={{ fontSize: 15 }}>Доступно обновление</b>
            <div style={{ color: C.muted, fontSize: 12 }}>Версия {st.kind === "downloading" ? st.version : st.kind === "available" ? st.version : ""}</div>
          </div>
          {st.kind === "available" && (
            <button onClick={() => setSt({ kind: "hidden" })} title="Скрыть"
              style={{ background: "none", border: "none", color: C.mutedDim, fontSize: 18, cursor: "pointer" }}>✕</button>
          )}
        </div>

        {/* Что изменилось (release notes). */}
        {st.kind === "available" && (
          <>
            {st.notes ? (
              <div style={{ margin: "6px 0 14px", padding: 12, borderRadius: 12, background: C.space, border: `1px solid ${C.stroke}`, maxHeight: 160, overflow: "auto" }}>
                <div style={{ color: C.mutedDim, fontSize: 11, fontWeight: 600, textTransform: "uppercase", letterSpacing: 1, marginBottom: 6 }}>Что нового</div>
                <div style={{ color: C.onSurface, fontSize: 13, lineHeight: 1.5, whiteSpace: "pre-wrap" }}>{st.notes}</div>
              </div>
            ) : (
              <div style={{ color: C.muted, fontSize: 13, margin: "6px 0 14px" }}>Готово к установке.</div>
            )}
            <div style={{ display: "flex", gap: 10 }}>
              <button onClick={() => onUpdate(st.handle, st.version)}
                style={{ flex: 1, padding: "11px", borderRadius: 12, border: "none", background: G.accent, color: "#fff", fontWeight: 700, fontSize: 14, cursor: "pointer" }}>
                Обновить сейчас
              </button>
              <button onClick={() => setSt({ kind: "hidden" })}
                style={{ padding: "11px 16px", borderRadius: 12, border: `1px solid ${C.stroke}`, background: "transparent", color: C.muted, fontSize: 14, cursor: "pointer" }}>
                Позже
              </button>
            </div>
          </>
        )}

        {/* Прогресс скачивания. */}
        {st.kind === "downloading" && (
          <>
            <div style={{ color: C.muted, fontSize: 13, margin: "6px 0 10px" }}>Скачивание и установка… {Math.round(st.progress * 100)}%</div>
            <div style={{ height: 6, borderRadius: 3, background: C.stroke, overflow: "hidden" }}>
              <div style={{ height: "100%", width: `${st.progress * 100}%`, background: G.accent, transition: "width .2s" }} />
            </div>
            <div style={{ color: C.mutedDim, fontSize: 11, marginTop: 8 }}>Приложение перезапустится автоматически.</div>
          </>
        )}

        {st.kind === "error" && (
          <div style={{ color: C.coral, fontSize: 13 }}>Ошибка обновления: {st.message}</div>
        )}
      </div>

      <style>{"@keyframes upd-slide-in { from { opacity:0; transform: translateY(20px);} to {opacity:1; transform:translateY(0);} }"}</style>
    </div>
  );
}

function errText(e: unknown): string {
  if (e && typeof e === "object" && "message" in e) return String((e as { message?: string }).message ?? e);
  return String(e);
}
