/**
 * Палитра InfinityConnect (перенос значений InfinityColors из Android).
 * Тема фиолетовая; семантические цвета (успех/ошибка/предупреждение) отдельно.
 * Пинг-пилл красится по КАЧЕСТВУ, не по методу (см. quality()).
 *
 * TODO(Фаза 4): сверить точные hex с Android InfinityColors и дополнить.
 */
export const InfinityColors = {
  // Фиолетовый бренд.
  primary: "#7C4DFF",
  primaryDark: "#5E35D9",
  primaryLight: "#9E7BFF",

  // Фоны.
  background: "#0E0B1A",
  surface: "#1A1530",
  surfaceElevated: "#241C42",

  // Текст.
  textPrimary: "#F2EEFF",
  textSecondary: "#B4A9D6",
  textMuted: "#7A6F9E",

  // Семантические (НЕ трогать при рестайле).
  success: "#3DDC84",
  warning: "#FFB74D",
  error: "#FF5370",
} as const;

/** Цвет пинг-пилла по качеству задержки (мс), а не по методу измерения. */
export function pingColor(ms: number): string {
  if (ms < 0) return InfinityColors.textMuted; // недоступно
  if (ms < 100) return InfinityColors.success;
  if (ms < 250) return InfinityColors.warning;
  return InfinityColors.error;
}
