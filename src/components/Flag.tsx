/** Инлайновые SVG-флаги стран. Windows/WebView2 НЕ рендерит эмодзи-флаги
 *  (regional indicators показываются как буквы), поэтому рисуем SVG сами.
 *  Флаги простые (полосы/блоки) — узнаваемы и компактны. Код страны ISO alpha-2. */
import { InfinityColors as C } from "../theme/colors";

/** Горизонтальные полосы (сверху вниз). */
function stripes(colors: string[]) {
  const h = 100 / colors.length;
  return colors.map((col, i) => <rect key={i} x="0" y={i * h} width="150" height={h} fill={col} />);
}
/** Вертикальные полосы (слева направо). */
function vstripes(colors: string[]) {
  const w = 150 / colors.length;
  return colors.map((col, i) => <rect key={i} x={i * w} y="0" width={w} height="100" fill={col} />);
}

/** Карта ISO-код → набор прямоугольников SVG (viewBox 150×100). */
const FLAGS: Record<string, () => React.ReactNode> = {
  RU: () => <>{stripes(["#fff", "#0039A6", "#D52B1E"])}</>,
  NL: () => <>{stripes(["#AE1C28", "#fff", "#21468B"])}</>,
  DE: () => <>{stripes(["#000", "#DD0000", "#FFCE00"])}</>,
  FR: () => <>{vstripes(["#0055A4", "#fff", "#EF4135"])}</>,
  IT: () => <>{vstripes(["#009246", "#fff", "#CE2B37"])}</>,
  PL: () => <>{stripes(["#fff", "#DC143C"])}</>,
  FI: () => <><rect width="150" height="100" fill="#fff" /><rect x="40" width="20" height="100" fill="#003580" /><rect y="40" width="150" height="20" fill="#003580" /></>,
  SE: () => <><rect width="150" height="100" fill="#006AA7" /><rect x="40" width="16" height="100" fill="#FECC00" /><rect y="42" width="150" height="16" fill="#FECC00" /></>,
  GB: () => <><rect width="150" height="100" fill="#012169" /><path d="M0,0 150,100 M150,0 0,100" stroke="#fff" strokeWidth="20" /><path d="M0,0 150,100 M150,0 0,100" stroke="#C8102E" strokeWidth="8" /><rect x="60" width="30" height="100" fill="#fff" /><rect y="35" width="150" height="30" fill="#fff" /><rect x="65" width="20" height="100" fill="#C8102E" /><rect y="40" width="150" height="20" fill="#C8102E" /></>,
  US: () => <><rect width="150" height="100" fill="#fff" />{[0, 2, 4, 6, 8, 10, 12].map((i) => <rect key={i} y={i * (100 / 13)} width="150" height={100 / 13} fill="#B22234" />)}<rect width="60" height={100 / 13 * 7} fill="#3C3B6E" /></>,
  TR: () => <><rect width="150" height="100" fill="#E30A17" /><circle cx="60" cy="50" r="24" fill="#fff" /><circle cx="68" cy="50" r="19" fill="#E30A17" /><path d="M88 50 l-14 5 9-12 0 14 -9-12z" fill="#fff" /></>,
  JP: () => <><rect width="150" height="100" fill="#fff" /><circle cx="75" cy="50" r="30" fill="#BC002D" /></>,
  SG: () => <><rect width="150" height="50" fill="#EF3340" /><rect y="50" width="150" height="50" fill="#fff" /><circle cx="35" cy="28" r="18" fill="#fff" /><circle cx="42" cy="28" r="16" fill="#EF3340" /></>,
  KZ: () => <><rect width="150" height="100" fill="#00AFCA" /><circle cx="75" cy="45" r="18" fill="#FEC50C" /></>,
  ES: () => <><rect width="150" height="100" fill="#AA151B" /><rect y="25" width="150" height="50" fill="#F1BF00" /></>,
  CH: () => <><rect width="150" height="100" fill="#D52B1E" /><rect x="62" y="30" width="26" height="40" fill="#fff" /><rect x="55" y="40" width="40" height="20" fill="#fff" /></>,
  CA: () => <><rect width="150" height="100" fill="#fff" /><rect width="37" height="100" fill="#FF0000" /><rect x="113" width="37" height="100" fill="#FF0000" /><path d="M75 30 l6 14 14-4 -8 12 8 12 -14-4 -6 14 -6-14 -14 4 8-12 -8-12 14 4z" fill="#FF0000" /></>,
  AE: () => <><rect width="150" height="100" fill="#00732F" /><rect y="33" width="150" height="34" fill="#fff" /><rect y="66" width="150" height="34" fill="#000" /><rect width="40" height="100" fill="#FF0000" /></>,
  HK: () => <><rect width="150" height="100" fill="#DE2910" /><circle cx="75" cy="50" r="20" fill="#fff" /></>,
};

/** Отображает SVG-флаг страны или null, если код неизвестен. */
export function Flag({ code, size = 26 }: { code: string; size?: number }) {
  const draw = FLAGS[code.toUpperCase()];
  if (!draw) return null;
  return (
    <svg width={size} height={size * (2 / 3)} viewBox="0 0 150 100"
      style={{ borderRadius: 3, border: `1px solid ${C.stroke}`, display: "block" }}>
      {draw()}
    </svg>
  );
}

/** Есть ли SVG-флаг для этого кода. */
export function hasFlag(code: string): boolean {
  return !!FLAGS[code.toUpperCase()];
}
