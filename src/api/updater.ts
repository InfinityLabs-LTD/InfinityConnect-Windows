/** Обёртка над tauri-plugin-updater: проверка/скачивание/установка обновления.
 *  Файлы раздаются с GitHub Releases (latest.json + подписанные артефакты). */
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export interface UpdateInfo {
  available: boolean;
  version?: string;
  notes?: string;
  date?: string;
}

/** Проверяет наличие обновления. Не качает — только сравнивает версии. */
export async function checkForUpdate(): Promise<{ info: UpdateInfo; handle: Update | null }> {
  const update = await check();
  if (update) {
    return {
      handle: update,
      info: {
        available: true,
        version: update.version,
        notes: update.body ?? undefined,
        date: update.date ?? undefined,
      },
    };
  }
  return { handle: null, info: { available: false } };
}

/** Скачивает и устанавливает обновление, затем перезапускает приложение.
 *  `onProgress` — колбэк прогресса (0..1), опционально. */
export async function downloadAndInstall(
  update: Update,
  onProgress?: (fraction: number) => void,
): Promise<void> {
  let total = 0;
  let downloaded = 0;
  await update.downloadAndInstall((event) => {
    switch (event.event) {
      case "Started":
        total = event.data.contentLength ?? 0;
        break;
      case "Progress":
        downloaded += event.data.chunkLength;
        if (total > 0 && onProgress) onProgress(Math.min(downloaded / total, 1));
        break;
      case "Finished":
        if (onProgress) onProgress(1);
        break;
    }
  });
  // Установка завершена — перезапускаем приложение в новой версии.
  await relaunch();
}
