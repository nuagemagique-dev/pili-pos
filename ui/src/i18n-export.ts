// Nightly export panel strings (Settings). Kept out of i18n.ts on
// purpose (decision X1D11): scripts/android/vue_to_android.mjs parses
// i18n.ts and the Android generated strings must stay unchanged.
// Keys: snake_case; every entry has fr, en, zh.
import type { Messages } from "./i18n";
import { usePosStore } from "./stores/pos";

export const exportMessages: Messages = {
  "title": { fr: "Export nocturne", en: "Nightly export", zh: "夜间导出" },
  "state_disabled": { fr: "Non configuré", en: "Not configured", zh: "未配置" },
  "state_invalid": { fr: "Configuration invalide", en: "Invalid configuration", zh: "配置无效" },
  "state_ok": { fr: "À jour", en: "Up to date", zh: "已是最新" },
  "state_pending": {
    fr: "{n} exports en attente",
    en: "{n} exports waiting",
    zh: "{n} 个导出待上传",
  },
  "endpoint": { fr: "Destination", en: "Endpoint", zh: "目标地址" },
  "last_export": { fr: "Dernier export", en: "Last export", zh: "最近导出" },
  "last_upload": { fr: "Dernier envoi", en: "Last upload", zh: "最近上传" },
  "next_attempt": { fr: "Prochain essai", en: "Next attempt", zh: "下次尝试" },
  "last_error": { fr: "Dernière erreur", en: "Last error", zh: "最近错误" },
  "kept": {
    fr: "Conservés : {n} sur {days} jours",
    en: "Kept: {n} of {days} days",
    zh: "已保留：{n} 个（{days} 天内）",
  },
  "dropped": { fr: "Abandonnés", en: "Dropped", zh: "已丢弃" },
  "retry_now": { fr: "Réessayer maintenant", en: "Retry now", zh: "立即重试" },
  "retrying": { fr: "Nouvel essai…", en: "Retrying…", zh: "正在重试…" },
  "never": { fr: "Jamais", en: "Never", zh: "从未" },
};

/** The panel's `t`: current UI language like i18n.ts, French when a
 * language or a key is missing, the key itself when unknown. `params`
 * fills `{name}` placeholders. */
export function useExportText() {
  const pos = usePosStore();

  function t(key: string, params?: Record<string, string | number>): string {
    const entry = exportMessages[key];
    let value = entry ? (entry[pos.language] ?? entry.fr ?? key) : key;
    if (params) {
      for (const [name, v] of Object.entries(params)) {
        value = value.replaceAll(`{${name}}`, String(v));
      }
    }
    return value;
  }

  return { t };
}
