/**
 * Центральная круглая кнопка подключения (перенос Android ConnectHero).
 * Радиальное свечение меняет цвет по состоянию: idle → фиолет, connecting →
 * пульсирующий индиго + вращающееся кольцо, connected → мятный (полное кольцо).
 */
import { InfinityColors as C } from "../theme/colors";
import type { TunnelStateEvent } from "../api/commands";

export function ConnectHero({
  status,
  enabled,
  onToggle,
  compact = false,
}: {
  status: TunnelStateEvent["status"];
  enabled: boolean;
  onToggle: () => void;
  compact?: boolean;
}) {
  const connected = status === "connected";
  const connecting = status === "connecting";
  const error = status === "error";

  const heroSize = compact ? 176 : 240;
  const discSize = compact ? 112 : 150;

  const accent = connected ? C.mint : error ? C.coral : C.accentBlue;
  const discGradient = connected
    ? `linear-gradient(135deg, ${C.mint}, ${C.accentCyan})`
    : `linear-gradient(135deg, ${C.accentIndigo}, ${C.accentMagenta})`;

  const ringR = heroSize * 0.31;
  const circ = 2 * Math.PI * ringR;

  return (
    <div
      style={{
        width: heroSize,
        height: heroSize,
        position: "relative",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
      }}
    >
      {/* Радиальное свечение + кольцо (SVG). */}
      <svg
        width={heroSize}
        height={heroSize}
        style={{
          position: "absolute",
          inset: 0,
          animation: connecting || connected ? "hero-pulse 2.2s ease-in-out infinite" : undefined,
        }}
      >
        <defs>
          <radialGradient id="glow" cx="50%" cy="50%" r="50%">
            <stop offset="0%" stopColor={accent} stopOpacity={0.45} />
            <stop offset="100%" stopColor={accent} stopOpacity={0} />
          </radialGradient>
        </defs>
        <circle cx={heroSize / 2} cy={heroSize / 2} r={heroSize / 2} fill="url(#glow)" />
        {/* Базовое кольцо. */}
        <circle
          cx={heroSize / 2}
          cy={heroSize / 2}
          r={ringR}
          fill="none"
          stroke={C.stroke}
          strokeWidth={3}
        />
        {/* Активная дуга: полное кольцо (connected) / вращающийся сегмент (connecting). */}
        {connected && (
          <circle
            cx={heroSize / 2}
            cy={heroSize / 2}
            r={ringR}
            fill="none"
            stroke={accent}
            strokeWidth={4}
          />
        )}
        {connecting && (
          <circle
            cx={heroSize / 2}
            cy={heroSize / 2}
            r={ringR}
            fill="none"
            stroke={accent}
            strokeWidth={4}
            strokeLinecap="round"
            strokeDasharray={`${circ * 0.25} ${circ}`}
            style={{
              transformOrigin: "center",
              animation: "hero-spin 1.2s linear infinite",
            }}
          />
        )}
      </svg>

      {/* Внутренний диск-кнопка. */}
      <button
        onClick={onToggle}
        disabled={!enabled}
        aria-label={connected ? "Отключить" : "Подключить"}
        style={{
          width: discSize,
          height: discSize,
          borderRadius: "50%",
          border: "none",
          background: discGradient,
          cursor: enabled ? "pointer" : "default",
          opacity: enabled ? 1 : 0.5,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          boxShadow: `0 8px 40px ${accent}66`,
          position: "relative",
          zIndex: 1,
        }}
      >
        {/* Иконка питания. */}
        <svg width={compact ? 40 : 52} height={compact ? 40 : 52} viewBox="0 0 24 24" fill="none">
          <path
            d="M12 3v9"
            stroke="#fff"
            strokeWidth={2.2}
            strokeLinecap="round"
          />
          <path
            d="M6.3 6.3a8 8 0 1 0 11.4 0"
            stroke="#fff"
            strokeWidth={2.2}
            strokeLinecap="round"
            fill="none"
          />
        </svg>
      </button>

      {/* CSS-анимации (инлайн-стайл через <style>). */}
      <style>{`
        @keyframes hero-pulse { 0%,100% { transform: scale(0.92); } 50% { transform: scale(1.06); } }
        @keyframes hero-spin { to { transform: rotate(360deg); } }
      `}</style>
    </div>
  );
}
