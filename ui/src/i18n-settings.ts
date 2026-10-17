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

  // Email tickets panel
  "settings_email_title": { fr: "Tickets par email", en: "Email tickets", zh: "电子邮件票据" },

  // HubRise panel

  // Courses panel (v1.2): "Service en plusieurs temps"
  "courses_title": { fr: "Service en plusieurs temps", en: "Courses", zh: "分道出菜" },
  "courses_state_on": { fr: "Actif", en: "On", zh: "已开启" },
  "courses_state_off": { fr: "Inactif", en: "Off", zh: "未开启" },
  "courses_note": {
    fr: "Le bon de cuisine imprime le premier temps tout de suite et garde les suivants jusqu'à « Envoyer la suite ». Rien de cela n'entre dans les données fiscales.",
    en: "The kitchen slip prints the first course at once and holds the next ones until \"Send next course\". None of this enters the fiscal data.",
    zh: "厨房单立即打印第一道，其余等待“发出下一道”。这些都不会进入税务数据。",
  },
  "courses_categories": { fr: "Temps de chaque catégorie", en: "Course of each category", zh: "各分类的出菜顺序" },
  "courses_no_categories": { fr: "Aucune catégorie", en: "No category", zh: "没有分类" },
  "courses_save": { fr: "Enregistrer", en: "Save", zh: "保存" },
  "courses_saved": { fr: "Réglage enregistré", en: "Setting saved", zh: "已保存" },

  // Owner alerts panel (v1.2): Telegram messages to the owner's phone
  "alerts_title": { fr: "Alertes sur téléphone", en: "Phone alerts", zh: "手机警报" },
  "alerts_state_on": { fr: "Actives", en: "On", zh: "已开启" },
  "alerts_state_off": { fr: "Inactives", en: "Off", zh: "未开启" },
  "alerts_token_ok": { fr: "Robot Telegram configuré", en: "Telegram bot configured", zh: "Telegram 机器人已配置" },
  "alerts_token_missing_pill": { fr: "Robot Telegram absent", en: "Telegram bot missing", zh: "缺少 Telegram 机器人" },
  "alerts_token_missing_note": {
    fr: "Le jeton du robot n'est pas configuré sur cette caisse. Il se règle à l'installation (pos.json), jamais ici.",
    en: "The bot token is not configured on this till. It is set at installation (pos.json), never here.",
    zh: "此收银机未配置机器人令牌。令牌在安装时设置（pos.json），不在此处设置。",
  },
  "alerts_chat_id": { fr: "Conversation Telegram", en: "Telegram chat", zh: "Telegram 会话" },
  "alerts_chat_id_unset": { fr: "Non renseignée", en: "Not set", zh: "未设置" },
  "alerts_label": { fr: "Nom dans les messages", en: "Name in the messages", zh: "消息中的名称" },
  "alerts_label_placeholder": { fr: "Vide : identifiant de la caisse", en: "Empty: the till identifier", zh: "留空：使用收银机标识" },
  "alerts_language": { fr: "Langue des messages", en: "Message language", zh: "消息语言" },
  "alerts_kinds": { fr: "Prévenir quand", en: "Warn when", zh: "在以下情况提醒" },
  "alerts_kind_fdm": { fr: "La boîte noire ne répond plus", en: "The FDM stops answering", zh: "黑匣子无响应" },
  "alerts_kind_printer": { fr: "Une imprimante est injoignable", en: "A printer is unreachable", zh: "打印机无法连接" },
  "alerts_kind_terminal": { fr: "Le terminal de paiement est injoignable", en: "The payment terminal is unreachable", zh: "支付终端无法连接" },
  "alerts_kind_z_missing": { fr: "Le Z du jour n'est pas fait", en: "The day's Z is not done", zh: "当日 Z 报表未完成" },
  "alerts_kind_online_waiting": { fr: "Une commande en ligne attend", en: "An online order is waiting", zh: "有网上订单在等待" },
  "alerts_z_deadline": { fr: "Z attendu avant", en: "Z expected before", zh: "Z 报表截止时间" },
  "alerts_online_wait": { fr: "Commande en ligne en attente depuis", en: "Online order waiting for", zh: "网上订单等待时间" },
  "alerts_reminder": { fr: "Rappel toutes les", en: "Remind every", zh: "提醒间隔" },
  "alerts_value_minutes": { fr: "{n} min", en: "{n} min", zh: "{n} 分钟" },
  "alerts_value_hour": { fr: "{n} h", en: "{n} h", zh: "{n} 点" },
  "alerts_value_never": { fr: "Jamais", en: "Never", zh: "从不" },
  "alerts_save": { fr: "Enregistrer", en: "Save", zh: "保存" },
  "alerts_saved": { fr: "Enregistré", en: "Saved", zh: "已保存" },
  "alerts_test": { fr: "Envoyer un message test", en: "Send a test message", zh: "发送测试消息" },
  "alerts_test_sent": { fr: "Message test envoyé", en: "Test message sent", zh: "测试消息已发送" },
  "alerts_note": {
    fr: "Les messages ne contiennent ni nom de client, ni adresse, ni montant, ni numéro de ticket. Ils ne bloquent jamais la caisse.",
    en: "Messages never contain a customer name, an address, an amount or a ticket number. They never block the till.",
    zh: "消息不含客户姓名、地址、金额或小票号码，也不会阻止收银机运行。",
  },
  "alerts_log_title": { fr: "Derniers messages", en: "Latest messages", zh: "最近消息" },
  "alerts_log_empty": { fr: "Aucun message pour l'instant", en: "No message yet", zh: "暂无消息" },
  "alerts_transition_raised": { fr: "Alerte", en: "Alert", zh: "警报" },
  "alerts_transition_reminder": { fr: "Rappel", en: "Reminder", zh: "提醒" },
  "alerts_transition_resolved": { fr: "Résolu", en: "Resolved", zh: "已恢复" },
  "alerts_transition_test": { fr: "Test", en: "Test", zh: "测试" },
  "alerts_outcome_sent": { fr: "Envoyé", en: "Sent", zh: "已发送" },
  "alerts_outcome_failed": { fr: "Échec", en: "Failed", zh: "失败" },
  "alerts_outcome_suppressed": { fr: "Retenu (limite horaire)", en: "Held back (hourly limit)", zh: "已拦截（每小时上限）" },
  "alerts_minus": { fr: "Groupe (signe -)", en: "Group (minus sign)", zh: "群组（负号）" },
};
