// Палитра установщика — та же, что в приложении (src/theme/colors.ts).
export const C = {
  space: "#0B0716",
  spaceHi: "#150E28",
  surface: "#1C1338",
  surfaceHi: "#261A4C",
  stroke: "#352455",
  indigo: "#6C3CFF",
  blue: "#9D5CFF",
  cyan: "#C77DFF",
  magenta: "#E85CD8",
  mint: "#22E1A1",
  coral: "#FF5A6E",
  text: "#F2EEFA",
  muted: "#B9A9E0",
  mutedDim: "#7C6BA6",
} as const;

export const accentGradient = `linear-gradient(120deg, ${C.indigo}, ${C.magenta})`;
