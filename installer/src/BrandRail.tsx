import { C } from "./theme";

/** Левая фирменная панель: логотип «I», название, версия — как в макете. */
export function BrandRail({ version }: { version: string }) {
  return (
    <div style={{
      width: 190, flexShrink: 0, position: "relative", overflow: "hidden",
      padding: "26px 20px", display: "flex", flexDirection: "column", alignItems: "center", gap: 16,
      background: `radial-gradient(200px 160px at 30% 8%, #241653 0%, transparent 70%),
                   linear-gradient(180deg, ${C.spaceHi}, ${C.space})`,
      borderRight: `1px solid ${C.stroke}`,
    }}>
      {/* сетка как MeshBackground */}
      <div aria-hidden style={{
        position: "absolute", inset: 0, pointerEvents: "none", opacity: 0.1,
        backgroundImage: `linear-gradient(${C.indigo} 1px, transparent 1px), linear-gradient(90deg, ${C.indigo} 1px, transparent 1px)`,
        backgroundSize: "26px 26px",
      }} />
      <div style={{
        width: 72, height: 72, borderRadius: 20, marginTop: 8, zIndex: 1,
        background: `linear-gradient(160deg, ${C.indigo}, ${C.magenta})`,
        display: "grid", placeItems: "center", color: "#fff",
        fontSize: 42, fontWeight: 800, fontFamily: "Georgia, 'Times New Roman', serif",
        boxShadow: "0 12px 30px -6px rgba(108,60,255,0.6)",
      }}>I</div>
      <div style={{ textAlign: "center", zIndex: 1 }}>
        <b style={{ fontSize: 16, display: "block" }}>Infinity</b>
        <span style={{ color: C.cyan, fontSize: 16, fontWeight: 700 }}>Connect</span>
        <div style={{ height: 2, width: 90, margin: "10px auto 8px", borderRadius: 2, background: `linear-gradient(90deg, ${C.indigo}, ${C.magenta})` }} />
        <small style={{ color: C.mutedDim, fontSize: 11, letterSpacing: "0.14em" }}>VPN</small>
      </div>
      <div style={{ marginTop: "auto", color: C.mutedDim, fontSize: 11, zIndex: 1 }}>Версия {version}</div>
    </div>
  );
}
