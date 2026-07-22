import { C } from "../theme";
import { Btn, Check, H2, Sub } from "../ui";

/** Экран 3: успех + «Запустить сейчас» + «Завершить». */
export function Done({ version, launch, setLaunch, onFinish }: {
  version: string;
  launch: boolean;
  setLaunch: (v: boolean) => void;
  onFinish: () => void;
}) {
  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", alignItems: "flex-start", animation: "ic-fade-up .3s ease" }}>
      <div style={{
        width: 66, height: 66, borderRadius: "50%", margin: "16px 0 4px",
        background: "radial-gradient(circle at 50% 40%, #2b5f4d, #143327)",
        border: "1px solid #2f8f6a", display: "grid", placeItems: "center",
        color: C.mint, fontSize: 32, boxShadow: "0 0 40px -6px rgba(34,225,161,0.5)",
      }}>✓</div>
      <H2>Готово!</H2>
      <Sub>Infinity Connect установлен и готов к работе.</Sub>

      <div style={{ marginTop: 18 }}>
        <Check checked={launch} onToggle={() => setLaunch(!launch)} label="Запустить Infinity Connect сейчас" />
      </div>

      <div style={{ marginTop: "auto", paddingTop: 22, width: "100%", display: "flex", alignItems: "center", justifyContent: "space-between" }}>
        <span style={{ fontSize: 11.5, color: C.mutedDim }}>Издатель: Infinity Labs · v{version}</span>
        <Btn onClick={onFinish}>Завершить</Btn>
      </div>
    </div>
  );
}
