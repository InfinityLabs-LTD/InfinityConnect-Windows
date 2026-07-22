/**
 * Палитра InfinityConnect — точный перенос значений из Android `InfinityColors`
 * (ui/theme/Theme.kt). Тёмная premium: фиолетово-космический фон, индиго-фиолет
 * акцент, мятный «подключено», коралловый «ошибка». Семантические цвета не трогать.
 *
 * Пинг-пилл красится по КАЧЕСТВУ (не по методу) — см. pingColor().
 */
export const InfinityColors = {
  // Фоны.
  space: "#0B0716", // самый тёмный фон
  spaceElevated: "#150E28", // приподнятый фон (за картами)
  surface: "#1C1338", // карточка
  surfaceHi: "#261A4C", // карточка выделенная/ховер
  stroke: "#352455", // тонкая обводка

  // Акценты (фиолетовые).
  accentBlue: "#9D5CFF", // основной акцент
  accentIndigo: "#6C3CFF", // глубокий индиго-фиолет
  accentCyan: "#C77DFF", // светлый пурпурно-розовый
  accentMagenta: "#E85CD8", // розово-маджента (тёплый край градиента)

  // Семантические (НЕ трогать при рестайле).
  mint: "#22E1A1", // подключено / успех
  coral: "#FF5A6E", // ошибка / отключить
  amber: "#FFB020", // средний пинг / предупреждение

  // Текст.
  onSurface: "#EDE9F7", // основной
  muted: "#9A8FB6", // вторичный
  mutedDim: "#685C83", // самый приглушённый
} as const;

/** Градиенты и брэнд-кисти (CSS linear-gradient строки). */
export const InfinityGradients = {
  /** Основной акцент (кнопки, hero idle). */
  accent: `linear-gradient(135deg, ${InfinityColors.accentIndigo}, ${InfinityColors.accentBlue}, ${InfinityColors.accentMagenta})`,
  /** Hero подключено. */
  connected: `linear-gradient(135deg, ${InfinityColors.mint}, ${InfinityColors.accentCyan})`,
  /** Фон экрана (вертикальный). */
  screen: `linear-gradient(180deg, #160D2B, ${InfinityColors.space})`,
} as const;

/**
 * Цвет пинг-пилла по КАЧЕСТВУ задержки (мс), а не по методу измерения.
 * <100 мс — мятный, <250 — янтарный, иначе коралловый; недоступно — приглушённый.
 */
export function pingColor(ms: number | null): string {
  if (ms == null || ms <= 0) return InfinityColors.mutedDim;
  if (ms < 100) return InfinityColors.mint;
  if (ms < 250) return InfinityColors.amber;
  return InfinityColors.coral;
}
