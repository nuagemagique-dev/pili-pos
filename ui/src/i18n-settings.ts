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

  // Table plan panel (managers): the table numbers the Tables screen shows
  "settings_tables": { fr: "Plan de salle", en: "Table plan", zh: "桌位" },
  "settings_tables_current": { fr: "{count} tables : {plan}", en: "{count} tables: {plan}", zh: "{count}张桌：{plan}" },
  "settings_tables_placeholder": { fr: "1-12, 20-25", en: "1-12, 20-25", zh: "1-12, 20-25" },
  "settings_tables_save": { fr: "Enregistrer", en: "Save", zh: "保存" },
  "settings_tables_saved": { fr: "Plan de salle enregistré", en: "Table plan saved", zh: "桌位已保存" },
  "settings_tables_note": {
    fr: "Numéros et plages séparés par des virgules. Une table qui a encore une commande ouverte reste affichée.",
    en: "Numbers and ranges separated by commas. A table that still has an open order stays on screen.",
    zh: "用逗号分隔号码和范围。仍有未结订单的桌位会继续显示。",
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
  // Presence registration by the manager (lane 1-v1-fixes)
  "settings_presence_in": { fr: "Arrivée", en: "Clock in", zh: "上班打卡" },
  "settings_presence_out": { fr: "Départ", en: "Clock out", zh: "下班打卡" },
  "settings_on_shift_since": { fr: "En service depuis {time}", en: "On shift since {time}", zh: "{time} 起上班中" },
  "settings_off_shift": { fr: "Hors service", en: "Off shift", zh: "未上班" },
  "settings_presence_done_in": { fr: "Arrivée enregistrée · {name}", en: "Clock-in registered · {name}", zh: "已登记上班 · {name}" },
  "settings_presence_done_out": { fr: "Départ enregistré · {name}", en: "Clock-out registered · {name}", zh: "已登记下班 · {name}" },
  "settings_presence_fdm_down": {
    fr: "FDM indisponible : pointage impossible",
    en: "FDM unavailable: clocking in or out is not possible",
    zh: "FDM 不可用，无法打卡",
  },
  "settings_presence_note": {
    fr: "Personnel sans accès à la caisse (cuisine) : le Gérant enregistre son arrivée et son départ ici.",
    en: "Staff without till access (kitchen): the manager registers their clock-in and clock-out here.",
    zh: "不使用收银机的员工（厨房）：由经理在此登记上下班。",
  },


  // Badge reader panel (manager): NFC or USB wedge reader for presence and login
  "settings_badges": { fr: "Lecteur de badges", en: "Badge reader", zh: "工牌读卡器" },
  "badge_reader_off": { fr: "Désactivé", en: "Off", zh: "关闭" },
  "badge_reader_usb": { fr: "Lecteur USB", en: "USB reader", zh: "USB 读卡器" },
  "settings_badge_login": { fr: "Connexion par badge", en: "Badge login", zh: "刷工牌登录" },
  "settings_badge_login_note": {
    fr: "Les Gérants saisissent toujours leur PIN après le badge.",
    en: "Managers still enter their PIN after the badge.",
    zh: "经理刷工牌后仍需输入 PIN。",
  },
  "settings_badge_usb_timing": { fr: "Réglages du lecteur USB", en: "USB reader timing", zh: "USB 读卡器参数" },
  "settings_badge_gap": { fr: "Écart maximal entre caractères (ms)", en: "Maximum gap between characters (ms)", zh: "字符最大间隔（毫秒）" },
  "settings_badge_min_len": { fr: "Longueur minimale du code", en: "Minimum code length", zh: "卡号最短长度" },
  "settings_badge_max_len": { fr: "Longueur maximale du code", en: "Maximum code length", zh: "卡号最长长度" },
  "settings_badge_nfc_unavailable": {
    fr: "Pas de NFC sur cet appareil : choisissez un lecteur USB",
    en: "No NFC on this device: choose a USB reader",
    zh: "本设备没有 NFC，请选择 USB 读卡器",
  },
  "settings_badge_relink": {
    fr: "La clé des badges a été renouvelée : liez de nouveau les badges",
    en: "The badge key was renewed: link the badges again",
    zh: "工牌密钥已更新，请重新绑定工牌",
  },
  "settings_badge_privacy": {
    fr: "Seule une empreinte chiffrée du badge est conservée, jamais son numéro. Usage : présence et connexion uniquement.",
    en: "Only a keyed hash of the badge is kept, never its number. Used for presence and login only.",
    zh: "仅保存工牌的加密指纹，从不保存卡号，仅用于考勤和登录。",
  },
  "settings_badge_log": { fr: "Derniers événements", en: "Latest events", zh: "最近记录" },
  "settings_badge_log_empty": { fr: "Aucun événement", en: "No event yet", zh: "暂无记录" },

  // Email tickets panel
  "settings_email_title": { fr: "Tickets par email", en: "Email tickets", zh: "电子邮件票据" },

  // HubRise panel
};
