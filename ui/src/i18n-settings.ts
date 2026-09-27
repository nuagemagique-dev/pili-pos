// Settings screen strings added with the Pili redesign. Merged into the
// dictionary by i18n.ts. Keys: snake_case; every entry has fr, en, zh.
import type { Messages } from "./i18n";

export const settingsMessages: Messages = {
  // Training mode panel (UX Guide §9)
  "settings_training_on": { fr: "Actif", en: "On", zh: "已开启" },
  "settings_training_off": { fr: "Inactif", en: "Off", zh: "未开启" },
  "settings_training_effect": {
    fr: "Fond violet, bandeau et filigrane sur tous les écrans. Les tickets sont marqués TRAINING.",
    en: "Purple background, banner and watermark on every screen. Tickets are marked TRAINING.",
    zh: "所有界面显示紫色背景、横幅和水印。票据标记为 TRAINING。",
  },

  // Language panel. Language names are endonyms: identical in every UI language.
  "settings_lang_name_fr": { fr: "Français", en: "Français", zh: "Français" },
  "settings_lang_name_zh": { fr: "中文", en: "中文", zh: "中文" },
  "settings_lang_name_en": { fr: "English", en: "English", zh: "English" },
  "settings_language_note": {
    fr: "Les noms des produits restent bilingues.",
    en: "Product names stay bilingual.",
    zh: "产品名称保持双语显示。",
  },

  // Remote support panel
  "settings_support_idle": { fr: "aucune session", en: "no session", zh: "无会话" },
  "settings_support_running": { fr: "session en cours", en: "session running", zh: "会话进行中" },
  "settings_support_unavailable": { fr: "indisponible", en: "unavailable", zh: "不可用" },
  "settings_support_banner_note": {
    fr: "Pendant une session, un bandeau rouge reste visible en bas de tous les écrans.",
    en: "During a session, a red banner stays visible at the bottom of every screen.",
    zh: "会话期间，所有界面底部都会显示红色横幅。",
  },

  // Employees panel
  "settings_pin_set": { fr: "PIN défini", en: "PIN set", zh: "已设PIN" },
  "settings_pin_missing": { fr: "sans PIN", en: "no PIN", zh: "未设PIN" },
  "settings_pin_button": { fr: "Code PIN", en: "PIN code", zh: "PIN码" },
  "settings_insz_hint": { fr: "INSZ · 11 chiffres", en: "INSZ · 11 digits", zh: "INSZ · 11位" },
  "settings_add_employee": { fr: "Ajouter l’employé", en: "Add employee", zh: "添加员工" },

  // Email tickets panel
  "settings_email_title": { fr: "Tickets par email", en: "Email tickets", zh: "电子邮件票据" },

  // HubRise panel
  "settings_hubrise_pending": { fr: "{count} en attente", en: "{count} pending", zh: "{count} 待处理" },
};
