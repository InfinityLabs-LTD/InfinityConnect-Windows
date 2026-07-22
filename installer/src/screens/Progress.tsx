import { C } from "../theme";
import { H2, Sub } from "../ui";

export interface Progress {
  fraction: number; // 0..1
  step: string;
  log: string[];
}

/** Экран 2: живой прогресс установки + лог компонентов. */
export function ProgressScreen({ progress }: { progress: Progress }) {
  const pct = Math.round(progress.fraction * 100);
  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", animation: "ic-fade-up .3s ease" }}>
      <H2>Устанавливаем…</H2>
      <Sub>Распаковываем ядра и настраиваем компоненты. Не закрывайте окно.</Sub>

      <div style={{ marginTop: 26, display: "flex", flexDirection: "column", gap: 12 }}>
        <div style={{ height: 10, borderRadius: 6, background: C.surfaceHi, overflow: "hidden" }}>
          <div style={{ height: "100%", width: `${pct}%`, borderRadius: 6, position: "relative", transition: "width .3s ease", background: `linear-gradient(90deg, ${C.indigo}, ${C.blue}, ${C.magenta})` }}>
            <div style={{ position: "absolute", inset: 0, background: "linear-gradient(90deg, transparent, rgba(255,255,255,0.35), transparent)", animation: "ic-sheen 1.4s linear infinite" }} />
          </div>
        </div>
        <div style={{ display: "flex", justifyContent: "space-between", fontSize: 12.5 }}>
          <span style={{ color: C.muted }}>{progress.step}</span>
          <span style={{ color: C.cyan, fontWeight: 700, fontVariantNumeric: "tabular-nums" }}>{pct}%</span>
        </div>
      </div>

      <div style={{ marginTop: 18, background: "rgba(11,7,22,0.6)", border: `1px solid ${C.stroke}`, borderRadius: 10, padding: "12px 14px", fontSize: 12, color: C.mutedDim, lineHeight: 1.7, fontFamily: "'Cascadia Code', Consolas, monospace", flex: 1, overflow: "hidden" }}>
        {progress.log.map((line, i) => (
          <div key={i}>
            <span style={{ color: C.mint, fontWeight: 600 }}>✓</span> {line}
          </div>
        ))}
      </div>

      <div style={{ marginTop: 14, fontSize: 11.5, color: C.mutedDim }}>Издатель: Infinity Labs</div>
    </div>
  );
}
