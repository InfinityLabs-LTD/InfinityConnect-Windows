import { useEffect, useState } from "react";
import { C } from "./theme";
import { BrandRail } from "./BrandRail";
import { Welcome, type InstallOptions } from "./screens/Welcome";
import { ProgressScreen, type Progress } from "./screens/Progress";
import { Done } from "./screens/Done";
import { browseDir, closeWindow, defaultInstallDir, install, launchApp } from "./api";

const VERSION = "1.0.0";

type Stage = "welcome" | "progress" | "done";

export function App() {
  const [stage, setStage] = useState<Stage>("welcome");
  const [opts, setOpts] = useState<InstallOptions>({
    dir: "C:\\Program Files\\Infinity Connect",
    desktopShortcut: true,
    autostart: true,
  });
  const [progress, setProgress] = useState<Progress>({ fraction: 0, step: "", log: [] });
  const [launch, setLaunch] = useState(true);

  useEffect(() => {
    defaultInstallDir().then((dir) => setOpts((o) => ({ ...o, dir }))).catch(() => {});
  }, []);

  async function onInstall() {
    setStage("progress");
    try {
      await install(opts, setProgress);
      setStage("done");
    } catch {
      // На этапе 2 добавим экран ошибки; пока просто остаёмся на прогрессе.
    }
  }

  async function onBrowse() {
    const picked = await browseDir(opts.dir);
    if (picked) setOpts((o) => ({ ...o, dir: picked }));
  }

  async function onFinish() {
    if (launch) await launchApp(opts.dir).catch(() => {});
    await closeWindow();
  }

  return (
    <div style={{
      height: "100vh", width: "100vw", overflow: "hidden",
      background: C.space, display: "flex", flexDirection: "column",
    }}>
      <TitleBar closable={stage !== "progress"} />
      <div style={{ display: "flex", flex: 1, minHeight: 0 }}>
        <BrandRail version={VERSION} />
        <div style={{ flex: 1, padding: "30px 30px 26px", minWidth: 0 }}>
          {stage === "welcome" && <Welcome opts={opts} setOpts={setOpts} onBrowse={onBrowse} onInstall={onInstall} />}
          {stage === "progress" && <ProgressScreen progress={progress} />}
          {stage === "done" && <Done version={VERSION} launch={launch} setLaunch={setLaunch} onFinish={onFinish} />}
        </div>
      </div>
    </div>
  );
}

/** Кастомный титлбар: перетаскивание окна + закрытие (скрыто во время установки). */
function TitleBar({ closable }: { closable: boolean }) {
  return (
    <div
      data-tauri-drag-region
      style={{
        height: 38, display: "flex", alignItems: "center", justifyContent: "space-between",
        padding: "0 6px 0 14px", flexShrink: 0,
        background: "linear-gradient(180deg, rgba(38,26,76,0.6), transparent)",
      }}
    >
      <span data-tauri-drag-region style={{ fontSize: 12, color: C.muted, letterSpacing: "0.02em" }}>
        Установка Infinity Connect
      </span>
      {closable && (
        <div
          onClick={() => closeWindow()}
          style={{ width: 34, height: 30, display: "grid", placeItems: "center", color: C.mutedDim, fontSize: 13, borderRadius: 6, cursor: "pointer" }}
          onMouseEnter={(e) => { e.currentTarget.style.background = "#E85C6E33"; e.currentTarget.style.color = "#ff9aa6"; }}
          onMouseLeave={(e) => { e.currentTarget.style.background = "transparent"; e.currentTarget.style.color = C.mutedDim; }}
        >
          ✕
        </div>
      )}
    </div>
  );
}
