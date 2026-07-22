import type { CSSProperties, ReactNode } from "react";
import { C, accentGradient } from "./theme";

/** Кнопка: primary (градиент) или ghost (обводка). */
export function Btn({ children, onClick, variant = "primary", disabled }: {
  children: ReactNode; onClick?: () => void; variant?: "primary" | "ghost"; disabled?: boolean;
}) {
  const base: CSSProperties = {
    fontSize: 13.5, fontWeight: 600, borderRadius: 10, padding: "10px 22px",
    cursor: disabled ? "default" : "pointer", border: "1px solid transparent",
    transition: "filter .15s, transform .05s", opacity: disabled ? 0.45 : 1,
  };
  const variStyle: CSSProperties = variant === "primary"
    ? { background: accentGradient, color: "#fff", boxShadow: "0 8px 22px -8px rgba(108,60,255,0.8)" }
    : { background: C.surface, borderColor: C.stroke, color: C.muted };
  return (
    <button
      onClick={disabled ? undefined : onClick}
      disabled={disabled}
      style={{ ...base, ...variStyle }}
      onMouseEnter={(e) => { if (!disabled && variant === "primary") e.currentTarget.style.filter = "brightness(1.08)"; if (!disabled && variant === "ghost") { e.currentTarget.style.color = C.text; e.currentTarget.style.borderColor = C.blue; } }}
      onMouseLeave={(e) => { e.currentTarget.style.filter = "none"; if (variant === "ghost") { e.currentTarget.style.color = C.muted; e.currentTarget.style.borderColor = C.stroke; } }}
    >
      {children}
    </button>
  );
}

/** Чекбокс-строка (фирменная галочка). */
export function Check({ checked, onToggle, label }: { checked: boolean; onToggle: () => void; label: string }) {
  return (
    <div onClick={onToggle} style={{ display: "flex", alignItems: "center", gap: 10, fontSize: 13, color: C.text, cursor: "pointer" }}>
      <span style={{
        width: 18, height: 18, borderRadius: 6, flexShrink: 0, display: "grid", placeItems: "center",
        fontSize: 12, color: checked ? "#fff" : "transparent",
        background: checked ? `linear-gradient(135deg, ${C.indigo}, ${C.blue})` : C.surface,
        border: checked ? "none" : `1px solid ${C.stroke}`,
      }}>✓</span>
      {label}
    </div>
  );
}

/** Заголовок экрана. */
export function H2({ children }: { children: ReactNode }) {
  return <h2 style={{ fontSize: 20, fontWeight: 700, letterSpacing: "-0.01em" }}>{children}</h2>;
}

/** Подзаголовок/описание. */
export function Sub({ children }: { children: ReactNode }) {
  return <div style={{ color: C.muted, fontSize: 13.5, lineHeight: 1.5, marginTop: 8 }}>{children}</div>;
}
