import { computed } from "vue";
import { webshopMessages } from "./i18n-webshop";
import { checkoutMessages } from "./i18n-checkout";
import { reportsMessages } from "./i18n-reports";
import { settingsMessages } from "./i18n-settings";

export type Messages = Record<string, Record<AppLanguage, string>>;

/* Synthetic fixture: the shapes of the real dictionary, a few entries each. */
const coreMessages: Messages = {
  // Plain entry with a bare key below it.
  "hello": { fr: "Bonjour", en: "Hello", zh: "你好" },
  bare_key: { fr: "Nu", en: "Bare", zh: "裸" },
  "apos": { fr: 'L\'été', en: "It's", zh: "它" },
  "dquote": { fr: "Dis \"oui\"", en: `Say "yes"`, zh: "说" },
  "backslash": { fr: "a\\b", en: "a\\b" },
  "markup": { fr: "A & B < C > D", en: "x" },
  "lead_at": { fr: "@home", en: "?what", zh: "a" },
  "newline": { fr: "ligne1\nligne2", en: "l1\nl2" },
  "double_space": { fr: "a  b", en: "a b" },
  "edge_space": { fr: " lead", en: "trail " },
  "percent": { fr: "100 % sûr", en: "100%" },
  "placeholder": { fr: "Part {n}", en: "Part {n}", zh: "第{n}份" },
  "concat": { fr: "long " + "text", en: "a" + 'b' },
  "unicode": { fr: "été", en: "ok" },
  "dup": { fr: "core", en: "core", zh: "core" },
  "err_alpha": { fr: "Alpha", en: "Alpha", zh: "甲" },
  "err_beta": {
    fr: "Beta",
    en: "Beta",
    zh: "乙",
  },
};

const screenMessages: Messages = {
  ...coreMessages,
  ...checkoutMessages,
  ...reportsMessages,
  ...settingsMessages,
};
const messages: Messages = { ...screenMessages, ...webshopMessages };

export function useI18n() {
  function t(key: string, vars?: Record<string, string | number>): string {
    let value = messages[key]?.fr ?? key;
    for (const [name, v] of Object.entries(vars ?? {})) {
      value = value.replaceAll(`{${name}}`, String(v));
    }
    return value;
  }
  return { t };
}
