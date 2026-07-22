/** Фоновая анимация: «сетевая» mesh-сетка + плывущие glow-узлы. Тех/VPN-вайб.
 *  Чистый CSS/SVG (без канваса и внешних либ) — лёгкий по CPU, работает под CSP. */
import { InfinityColors as C } from "../theme/colors";

export function MeshBackground() {
  return (
    <div aria-hidden style={{ position: "fixed", inset: 0, overflow: "hidden", zIndex: 0, pointerEvents: "none", background: `radial-gradient(1200px 800px at 20% -10%, #1B1140 0%, ${C.space} 60%)` }}>
      {/* Плывущие размытые glow-пятна (aurora под сеткой). */}
      <div style={blob(C.accentIndigo, "-10%", "-5%", 520, "mesh-float-a")} />
      <div style={blob(C.accentMagenta, "70%", "10%", 420, "mesh-float-b")} />
      <div style={blob(C.accentBlue, "35%", "60%", 480, "mesh-float-c")} />

      {/* Сетка линий (SVG pattern) с медленным дрейфом. */}
      <svg width="100%" height="100%" style={{ position: "absolute", inset: 0, opacity: 0.8 }}>
        <defs>
          <pattern id="grid" width="52" height="52" patternUnits="userSpaceOnUse">
            <path d="M 52 0 L 0 0 0 52" fill="none" stroke={C.accentIndigo} strokeOpacity="0.35" strokeWidth="1" />
          </pattern>
          <radialGradient id="gridFade" cx="50%" cy="35%" r="85%">
            <stop offset="0%" stopColor="#fff" stopOpacity="1" />
            <stop offset="70%" stopColor="#fff" stopOpacity="0.4" />
            <stop offset="100%" stopColor="#fff" stopOpacity="0" />
          </radialGradient>
          <mask id="gridMask">
            <rect width="100%" height="100%" fill="url(#gridFade)" />
          </mask>
        </defs>
        <g mask="url(#gridMask)" className="mesh-grid-drift">
          <rect x="-52" y="-52" width="200%" height="200%" fill="url(#grid)" />
        </g>
      </svg>

      {/* Пульсирующие glow-узлы сети. */}
      <svg width="100%" height="100%" style={{ position: "absolute", inset: 0, opacity: 0.8 }}>
        {NODES.map((n, i) => (
          <circle key={i} cx={n.x} cy={n.y} r="3" fill={n.color}>
            <animate attributeName="opacity" values="0.2;1;0.2" dur={`${n.dur}s`} begin={`${n.delay}s`} repeatCount="indefinite" />
            <animate attributeName="r" values="2;4;2" dur={`${n.dur}s`} begin={`${n.delay}s`} repeatCount="indefinite" />
          </circle>
        ))}
      </svg>

      <style>{CSS}</style>
    </div>
  );
}

function blob(color: string, left: string, top: string, size: number, anim: string): React.CSSProperties {
  return {
    position: "absolute", left, top, width: size, height: size, borderRadius: "50%",
    background: color, filter: "blur(120px)", opacity: 0.28,
    animation: `${anim} 26s ease-in-out infinite`,
  };
}

const NODES = [
  { x: "12%", y: "18%", color: C.accentCyan, dur: 6, delay: 0 },
  { x: "82%", y: "24%", color: C.accentBlue, dur: 8, delay: 1.5 },
  { x: "45%", y: "68%", color: C.mint, dur: 7, delay: 3 },
  { x: "68%", y: "82%", color: C.accentMagenta, dur: 9, delay: 2 },
  { x: "28%", y: "48%", color: C.accentBlue, dur: 6.5, delay: 4 },
  { x: "92%", y: "60%", color: C.accentCyan, dur: 7.5, delay: 1 },
  { x: "58%", y: "12%", color: C.accentMagenta, dur: 8.5, delay: 2.5 },
  { x: "8%", y: "78%", color: C.accentBlue, dur: 6.8, delay: 3.5 },
  { x: "38%", y: "90%", color: C.accentCyan, dur: 9.2, delay: 0.5 },
];

const CSS = `
@keyframes mesh-float-a { 0%,100% { transform: translate(0,0) scale(1); } 50% { transform: translate(60px,40px) scale(1.1); } }
@keyframes mesh-float-b { 0%,100% { transform: translate(0,0) scale(1); } 50% { transform: translate(-50px,50px) scale(1.15); } }
@keyframes mesh-float-c { 0%,100% { transform: translate(0,0) scale(1); } 50% { transform: translate(40px,-40px) scale(1.05); } }
@keyframes mesh-grid-drift { 0% { transform: translate(0,0); } 100% { transform: translate(46px,46px); } }
.mesh-grid-drift { animation: mesh-grid-drift 12s linear infinite; }
@media (prefers-reduced-motion: reduce) {
  .mesh-grid-drift, [class^="mesh-float"] { animation: none !important; }
}
`;
