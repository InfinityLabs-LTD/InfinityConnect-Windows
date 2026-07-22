/**
 * Переиспользуемые виджеты в стиле Happ/Infinity (перенос эстетики Android
 * components/Common.kt, Design.kt). Стеклянные карточки, статус-пиллы, бейджи.
 */
import type { CSSProperties, ReactNode } from "react";
import { InfinityColors as C } from "../theme/colors";

/** Стеклянная карточка с тонкой обводкой; highlighted — выделенная. */
export function GlassCard({
  children,
  highlighted = false,
  onClick,
  style,
}: {
  children: ReactNode;
  highlighted?: boolean;
  onClick?: () => void;
  style?: CSSProperties;
}) {
  return (
    <div
      onClick={onClick}
      onMouseEnter={(e) => {
        if (!onClick) return;
        e.currentTarget.style.borderColor = `${C.accentBlue}8C`;
        e.currentTarget.style.transform = "translateY(-2px)";
        e.currentTarget.style.boxShadow = `0 10px 30px ${C.space}CC`;
      }}
      onMouseLeave={(e) => {
        if (!onClick) return;
        e.currentTarget.style.borderColor = highlighted ? `${C.accentBlue}8C` : C.stroke;
        e.currentTarget.style.transform = "translateY(0)";
        e.currentTarget.style.boxShadow = "none";
      }}
      style={{
        background: highlighted ? C.surfaceHi : C.surface,
        border: `1px solid ${highlighted ? `${C.accentBlue}8C` : C.stroke}`,
        borderRadius: 18,
        padding: 14,
        cursor: onClick ? "pointer" : "default",
        transition: "background 160ms, border-color 160ms, transform 160ms, box-shadow 160ms",
        ...style,
      }}
    >
      {children}
    </div>
  );
}

/** Цветной пилл (пинг/статус). Цвет — фон 12% + текст полный. */
export function StatusPill({ text, color }: { text: string; color: string }) {
  return (
    <span
      style={{
        color,
        background: `${color}1F`,
        borderRadius: 999,
        padding: "3px 10px",
        fontSize: 12,
        fontWeight: 600,
        minWidth: 34,
        textAlign: "center",
        whiteSpace: "nowrap",
      }}
    >
      {text}
    </span>
  );
}

/** Эмодзи-бейдж в кружке (флаг сервера / иконка ключа). */
export function EmojiBadge({ emoji, size = 40 }: { emoji: string; size?: number }) {
  return (
    <div
      style={{
        width: size,
        height: size,
        borderRadius: "50%",
        background: C.spaceElevated,
        border: `1px solid ${C.stroke}`,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        fontSize: size * 0.5,
        flexShrink: 0,
      }}
    >
      {emoji}
    </div>
  );
}

/** Eyebrow-метка (SERVERS, MODE) — капс + трекинг. */
export function Eyebrow({ children }: { children: ReactNode }) {
  return (
    <div
      style={{
        color: C.muted,
        fontSize: 12,
        fontWeight: 600,
        letterSpacing: 1.4,
        textTransform: "uppercase",
      }}
    >
      {children}
    </div>
  );
}

/** Маленький чип (протокол VLESS/Hysteria2). */
export function Chip({ text, color = C.accentBlue }: { text: string; color?: string }) {
  return (
    <span
      style={{
        color,
        background: `${color}1F`,
        borderRadius: 6,
        padding: "2px 7px",
        fontSize: 11,
        fontWeight: 600,
      }}
    >
      {text}
    </span>
  );
}
