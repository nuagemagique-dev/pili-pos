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
    fr: "Les noms des produits suivent la langue choisie.",
    en: "Product names follow the chosen language.",
    zh: "产品名称跟随所选语言显示。",
  },

  // Product label panel: what leads a product button and a cart line
  "settings_product_label": { fr: "Affichage des produits", en: "Product display", zh: "商品显示" },
  "settings_product_label_name": { fr: "Nom", en: "Name", zh: "名称" },
  "settings_product_label_code": { fr: "Code", en: "Code", zh: "编号" },
  "settings_product_label_note": {
    fr: "Avec « Code », le code du menu (A2, B14) remplace le nom, affiché en dessous. Un produit sans code garde son nom. Les tickets ne changent pas.",
    en: "With “Code”, the menu code (A2, B14) replaces the name, shown underneath. A product without a code keeps its name. Tickets do not change.",
    zh: "选择“编号”后，菜单编号（A2、B14）代替名称显示，名称显示在下方。没有编号的商品仍显示名称。票据不变。",
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
};
