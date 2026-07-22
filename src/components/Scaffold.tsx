/** Каркас раздела внутри широкого лейаута: заголовок + контент.
 *  Навигацию ведёт сайдбар (AppShell), поэтому кнопки «назад» больше нет. */
import type { ReactNode } from "react";

export function Scaffold({ title, children }: { title: string; onBack?: () => void; children: ReactNode }) {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 16, maxWidth: 860 }}>
      <h1 style={{ fontSize: 26, fontWeight: 700, margin: 0, letterSpacing: -0.5 }}>{title}</h1>
      {children}
    </div>
  );
}
