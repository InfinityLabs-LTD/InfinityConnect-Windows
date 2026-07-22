/** Эмодзи-флаг страны из remark сервера. Ремарки вида «RU LTE | …»,
 *  «NL Нидерланды 1», «DE Premium 2» — берём код страны из начала строки
 *  или по названию, конвертируем в regional-indicator эмодзи. */

/** ISO 3166-1 alpha-2 → эмодзи-флаг (два regional indicator символа). */
function codeToFlag(cc: string): string {
  const up = cc.toUpperCase();
  if (up.length !== 2 || !/^[A-Z]{2}$/.test(up)) return "";
  const A = 0x1f1e6; // 🇦
  return String.fromCodePoint(A + (up.charCodeAt(0) - 65), A + (up.charCodeAt(1) - 65));
}

/** Русские/английские названия стран → ISO-код (частые для VPN-локаций). */
const NAME_TO_CC: Record<string, string> = {
  "нидерланды": "NL", "netherlands": "NL", "голландия": "NL",
  "россия": "RU", "russia": "RU",
  "германия": "DE", "germany": "DE",
  "финляндия": "FI", "finland": "FI",
  "швеция": "SE", "sweden": "SE",
  "сша": "US", "usa": "US", "америка": "US", "united states": "US",
  "франция": "FR", "france": "FR",
  "великобритания": "GB", "англия": "GB", "uk": "GB",
  "польша": "PL", "poland": "PL",
  "турция": "TR", "turkey": "TR",
  "япония": "JP", "japan": "JP",
  "сингапур": "SG", "singapore": "SG",
  "казахстан": "KZ", "литва": "LT", "латвия": "LV", "эстония": "EE",
  "испания": "ES", "италия": "IT", "швейцария": "CH", "австрия": "AT",
  "канада": "CA", "гонконг": "HK", "оаэ": "AE", "эмираты": "AE",
};

/**
 * Выводит ISO-код страны из remark сервера («RU LTE» → "RU", «Нидерланды» → "NL").
 * "" если не распознано. Используется компонентом <Flag> для рендера SVG-флага
 * (эмодзи-флаги на Windows не рендерятся). НЕ трогает подписку — только UI.
 */
export function countryCodeFromRemark(remark: string): string {
  // 1. ГЛАВНОЕ: код из эмодзи-флага в начале (🇷🇺 LTE, 🇩🇪 Premium 2). Панель
  //    ставит regional-indicator флаг — из него достаём ISO-код надёжнее всего.
  const cc = codeFromEmojiFlag(remark);
  if (cc) return cc;
  // 2. Код страны в начале текстом: «RU …», «NL …», «DE-Premium».
  const s = remark.replace(/^[^A-Za-zА-Яа-я]+/, "").trim();
  const m = s.match(/^([A-Za-z]{2})(?=[^A-Za-z]|$)/);
  if (m) return m[1].toUpperCase();
  // 3. По названию страны в тексте.
  const low = s.toLowerCase();
  for (const [name, code] of Object.entries(NAME_TO_CC)) {
    if (low.includes(name)) return code;
  }
  return "";
}

/** Извлекает ISO-код из ведущего эмодзи-флага (два regional indicator символа
 *  U+1F1E6..U+1F1FF → буквы A..Z). "" если флага нет. */
function codeFromEmojiFlag(str: string): string {
  const cps = Array.from(str.trimStart());
  if (cps.length < 2) return "";
  const A = 0x1f1e6;
  const c0 = cps[0].codePointAt(0) ?? 0;
  const c1 = cps[1].codePointAt(0) ?? 0;
  if (c0 >= A && c0 <= A + 25 && c1 >= A && c1 <= A + 25) {
    return String.fromCharCode(65 + (c0 - A), 65 + (c1 - A));
  }
  return "";
}

/** Эмодзи-флаг из remark (оставлено для возможного использования вне Windows). */
export function flagFromRemark(remark: string): string {
  const cc = countryCodeFromRemark(remark);
  return cc ? codeToFlag(cc) : "";
}
