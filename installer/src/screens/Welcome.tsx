import { C } from "../theme";
import { Btn, Check, H2, Sub } from "../ui";

export interface InstallOptions {
  dir: string;
  desktopShortcut: boolean;
  autostart: boolean;
}

/** Экран 1: приветствие, папка установки, опции, «Установить». */
export function Welcome({ opts, setOpts, onBrowse, onInstall }: {
  opts: InstallOptions;
  setOpts: (o: InstallOptions) => void;
  onBrowse: () => void;
  onInstall: () => void;
}) {
  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", animation: "ic-fade-up .3s ease" }}>
      <H2>Добро пожаловать</H2>
      <Sub>Сейчас Infinity Connect будет установлен на ваш компьютер. Это займёт меньше минуты.</Sub>

      <div style={{ marginTop: 22, display: "flex", flexDirection: "column", gap: 7 }}>
        <label style={{ fontSize: 11, color: C.mutedDim, letterSpacing: "0.08em", textTransform: "uppercase" }}>
          Папка установки
        </label>
        <div style={{ display: "flex", alignItems: "center", gap: 10, background: C.surface, border: `1px solid ${C.stroke}`, borderRadius: 10, padding: "10px 12px", fontSize: 13, color: C.muted }}>
          <span style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{opts.dir}</span>
          <span onClick={onBrowse} style={{ fontSize: 12, color: C.blue, fontWeight: 600, border: `1px solid ${C.stroke}`, borderRadius: 7, padding: "5px 10px", cursor: "pointer" }}>
            Обзор…
          </span>
        </div>
      </div>

      <div style={{ marginTop: 16, display: "flex", flexDirection: "column", gap: 11 }}>
        <Check checked={opts.desktopShortcut} onToggle={() => setOpts({ ...opts, desktopShortcut: !opts.desktopShortcut })} label="Создать ярлык на рабочем столе" />
        <Check checked={opts.autostart} onToggle={() => setOpts({ ...opts, autostart: !opts.autostart })} label="Запускать при входе в Windows" />
      </div>

      <div style={{ marginTop: "auto", paddingTop: 22, display: "flex", alignItems: "center", justifyContent: "space-between" }}>
        <span style={{ fontSize: 11.5, color: C.mutedDim }}>Издатель: Infinity Labs</span>
        <Btn onClick={onInstall}>Установить</Btn>
      </div>
    </div>
  );
}
