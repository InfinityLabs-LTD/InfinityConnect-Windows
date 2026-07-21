/** Каркас вложенного экрана: топбар с кнопкой «назад» + заголовок + контент. */
import type { ReactNode } from "react";
import { InfinityColors as C, InfinityGradients as G } from "../theme/colors";

export function Scaffold({ title, onBack, children }: { title: string; onBack: () => void; children: ReactNode }) {
  return (
    <div style={{ minHeight: "100vh", background: G.screen, color: C.onSurface, fontFamily: "Segoe UI, system-ui, sans-serif", padding: 16, display: "flex", flexDirection: "column", gap: 14 }}>
      <header style={{ display: "flex", alignItems: "center", gap: 12 }}>
        <button onClick={onBack} title="Назад"
          style={{ background: C.surface, border: `1px solid ${C.stroke}`, borderRadius: 10, width: 38, height: 38, fontSize: 18, cursor: "pointer", color: C.onSurface }}>
          ‹
        </button>
        <b style={{ fontSize: 18 }}>{title}</b>
      </header>
      {children}
    </div>
  );
}
