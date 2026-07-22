// Тонкий слой над Tauri-бэком установщика: реальные invoke()/listen().
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { Progress } from "./screens/Progress";
import type { InstallOptions } from "./screens/Welcome";

/** Каталог установки по умолчанию (Program Files\Infinity Connect). */
export function defaultInstallDir(): Promise<string> {
  return invoke<string>("default_install_dir");
}

/** Диалог выбора папки; возвращает путь или null. */
export function browseDir(current: string): Promise<string | null> {
  return invoke<string | null>("browse_dir", { current });
}

/** Запускает установку, шлёт прогресс через onProgress, резолвится по завершении. */
export async function install(opts: InstallOptions, onProgress: (p: Progress) => void): Promise<void> {
  const un = await listen<Progress>("install://progress", (e) => onProgress(e.payload));
  try {
    await invoke("install", { opts });
  } finally {
    un();
  }
}

/** Закрыть окно установщика. */
export async function closeWindow(): Promise<void> {
  await getCurrentWindow().close();
}

/** Запустить установленное приложение из папки установки. */
export async function launchApp(dir: string): Promise<void> {
  await invoke("launch_app", { dir });
}
