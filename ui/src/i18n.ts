import { computed } from "vue";
import { usePosStore } from "./stores/pos";
import type { AppLanguage } from "./stores/pos";

const messages: Record<string, Record<AppLanguage, string>> = {
  // OrderView
  "tables": { fr: "Tables", en: "Tables", zh: "桌位" },
  "reports": { fr: "Rapports", en: "Reports", zh: "报表" },
  "config": { fr: "Config", en: "Config", zh: "设置" },
  "online_orders": { fr: "En ligne", en: "Online", zh: "在线" },
  "logout": { fr: "Sortir", en: "Logout", zh: "退出" },

  // WebshopOrdersView — filter tabs. One tab per working status;
  // the terminal statuses (completed / rejected / cancelled) share
  // a single "Terminées" bucket since they all mean "card is dead"
  // from the staff's live-triage perspective.
  "webshop_filter_all": { fr: "Toutes", en: "All", zh: "全部" },
  "webshop_filter_new": { fr: "Nouvelles", en: "New", zh: "新订单" },
  "webshop_filter_accepted": { fr: "Acceptées", en: "Accepted", zh: "已接受" },
  "webshop_filter_in_preparation": { fr: "En prépa", en: "In prep", zh: "准备中" },
  "webshop_filter_awaiting_collection": { fr: "Prêtes", en: "Ready", zh: "待取" },
  "webshop_filter_in_delivery": { fr: "En livraison", en: "In delivery", zh: "配送中" },
  "webshop_filter_done": { fr: "Terminées", en: "Done", zh: "已完成" },

  // WebshopOrdersView — status chips
  "webshop_status_new": { fr: "Nouvelle", en: "New", zh: "新" },
  "webshop_status_received": { fr: "Reçue", en: "Received", zh: "已收到" },
  "webshop_status_accepted": { fr: "Acceptée", en: "Accepted", zh: "已接受" },
  "webshop_status_in_preparation": { fr: "En prépa", en: "In prep", zh: "准备中" },
  "webshop_status_ready": { fr: "Prête", en: "Ready", zh: "待取" },
  "webshop_status_in_delivery": { fr: "En livraison", en: "In delivery", zh: "配送中" },
  "webshop_status_completed": { fr: "Terminée", en: "Done", zh: "已完成" },
  "webshop_status_rejected": { fr: "Refusée", en: "Rejected", zh: "已拒绝" },
  "webshop_status_cancelled": { fr: "Annulée", en: "Cancelled", zh: "已取消" },

  // WebshopOrdersView — service type rendering
  "webshop_service_delivery": { fr: "Livraison", en: "Delivery", zh: "外卖" },
  "webshop_service_collection": { fr: "À emporter", en: "Pickup", zh: "自取" },
  "webshop_service_eat_in": { fr: "Sur place", en: "Eat in", zh: "堂食" },
  "webshop_service_unknown": { fr: "—", en: "—", zh: "—" },

  // WebshopOrdersView — primary + reject actions
  "webshop_action_accept": { fr: "Accepter", en: "Accept", zh: "接受" },
  "webshop_action_prepare": { fr: "Préparer", en: "Prepare", zh: "开始准备" },
  "webshop_action_ready": { fr: "Prête", en: "Ready", zh: "完成" },
  "webshop_action_out_for_delivery": {
    fr: "En livraison",
    en: "Out for delivery",
    zh: "出发配送",
  },
  "webshop_action_complete": { fr: "Terminer", en: "Complete", zh: "结束" },
  "webshop_action_reject": { fr: "Refuser", en: "Reject", zh: "拒绝" },

  // WebshopOrdersView — row labels
  "webshop_label_type": { fr: "Type", en: "Type", zh: "类型" },
  "webshop_label_time": { fr: "Heure", en: "Time", zh: "时间" },
  "webshop_label_customer": { fr: "Client", en: "Customer", zh: "顾客" },
  "webshop_label_ref": { fr: "Réf.", en: "Ref.", zh: "编号" },
  "webshop_label_total": { fr: "Total", en: "Total", zh: "合计" },

  // WebshopOrdersView — state messages
  "webshop_loading": { fr: "Chargement…", en: "Loading…", zh: "加载中…" },
  "webshop_empty": {
    fr: "Aucune commande à afficher.",
    en: "No orders to show.",
    zh: "暂无订单",
  },

  // WebshopOrdersView — delay chips + print buttons + toasts
  "webshop_label_ready_at": { fr: "Prêt à", en: "Ready at", zh: "预计" },
  "webshop_label_delivery_address": {
    fr: "Livraison",
    en: "Delivery",
    zh: "送货地址",
  },
  "webshop_action_print_kitchen": { fr: "Cuisine", en: "Kitchen", zh: "厨房" },
  "webshop_action_print_customer": { fr: "Client", en: "Customer", zh: "客单" },
  "webshop_toast_delay_ok": {
    fr: "Délai appliqué",
    en: "Delay applied",
    zh: "已延迟",
  },
  "webshop_toast_print_ok": {
    fr: "Ticket envoyé à l'imprimante",
    en: "Sent to printer",
    zh: "已送至打印机",
  },
  "webshop_toast_printer_missing": {
    fr: "Imprimante non configurée",
    en: "Printer not configured",
    zh: "未配置打印机",
  },
  "table": { fr: "Table", en: "Table", zh: "桌" },

  // Cart
  "order": { fr: "Commande", en: "Order", zh: "订单" },
  "no_items": { fr: "Aucun article", en: "No items", zh: "暂无商品" },
  "total": { fr: "TOTAL", en: "TOTAL", zh: "合计" },
  "pay": { fr: "PAYER", en: "PAY", zh: "结账" },

  // Checkout
  "cash": { fr: "CASH", en: "CASH", zh: "现金" },
  "card": { fr: "CARTE", en: "CARD", zh: "刷卡" },
  "exact": { fr: "Exact", en: "Exact", zh: "刚好" },
  "other": { fr: "Autre", en: "Other", zh: "其他" },
  "clear": { fr: "Effacer", en: "Clear", zh: "清除" },
  "validate": { fr: "Valider", en: "Confirm", zh: "确认" },
  "card_wait": { fr: "Encaisser sur le terminal", en: "Process on terminal", zh: "请在终端上刷卡" },
  "back": { fr: "Retour", en: "Back", zh: "返回" },
  "to_pay": { fr: "À payer", en: "To pay", zh: "应付" },
  "received": { fr: "Reçu", en: "Received", zh: "收到" },
  "change": { fr: "Rendu", en: "Change", zh: "找零" },
  "processing_fdm": { fr: "Communication FDM...", en: "FDM communication...", zh: "FDM通信中..." },
  "reprint": { fr: "Réimprimer", en: "Reprint", zh: "重印" },
  "finish": { fr: "Terminer", en: "Done", zh: "完成" },
  "fdm_no_sig": { fr: "FDM non connecté — ticket sans signature", en: "FDM offline — ticket unsigned", zh: "FDM未连接 — 票据无签名" },
  "before_rounding": { fr: "avant arrondi", en: "before rounding", zh: "四舍五入前" },

  // Login / PIN
  "login": { fr: "Connexion", en: "Login", zh: "登录" },
  "pin_required": { fr: "PIN requis", en: "PIN required", zh: "需要PIN" },
  "pin_wrong": { fr: "PIN incorrect", en: "Wrong PIN", zh: "PIN错误" },
  "pin_not_set_warning": {
    fr: "PIN non configuré — allez dans Config",
    en: "PIN not set — go to Settings",
    zh: "未设置PIN — 请前往设置",
  },
  "pin_enter": { fr: "Saisir le PIN", en: "Enter PIN", zh: "输入PIN" },
  "pin_change": { fr: "Changer le PIN", en: "Change PIN", zh: "修改PIN" },
  "pin_changed": { fr: "PIN modifié", en: "PIN changed", zh: "PIN已修改" },
  "pin_confirm": { fr: "Confirmer le PIN", en: "Confirm PIN", zh: "确认PIN" },
  "pin_mismatch": { fr: "Les PIN ne correspondent pas", en: "PINs don't match", zh: "PIN不匹配" },
  "other_insz": { fr: "Autre (INSZ)", en: "Other (INSZ)", zh: "其他 (INSZ)" },
  "digits_required": { fr: "11 chiffres requis", en: "11 digits required", zh: "需要11位数字" },

  // Tables
  "free": { fr: "Libre", en: "Free", zh: "空桌" },
  "items_short": { fr: "art.", en: "items", zh: "件" },
  "takeaway_new": { fr: "À emporter", en: "Takeaway", zh: "外带" },
  "takeaway_badge": { fr: "À emporter", en: "Takeaway", zh: "外带" },

  // Reports
  "report_x": { fr: "Rapport X", en: "X Report", zh: "X报表" },
  "report_z": { fr: "Rapport Z", en: "Z Report", zh: "Z报表" },
  "report_x_desc": { fr: "Résumé en cours de journée", en: "Mid-day summary", zh: "日中汇总" },
  "report_user_x": { fr: "Rapport utilisateurs X", en: "User X Report", zh: "员工X报表" },
  "report_user_x_desc": { fr: "Ventes et pointages par employé", en: "Per-employee sales & presence", zh: "按员工统计销售和打卡" },
  "report_z_desc": { fr: "Fin de journée (remise à zéro)", en: "End of day (reset)", zh: "日结（清零）" },
  "generate": { fr: "Générer", en: "Generate", zh: "生成" },
  "report_generated": { fr: "Rapport généré", en: "Report generated", zh: "报表已生成" },
  "report_history_title": { fr: "Historique Z", en: "Z History", zh: "Z报表记录" },
  "report_reprint": { fr: "Réimprimer", en: "Reprint", zh: "重印" },
  "report_reprint_ok": { fr: "Copie envoyée", en: "Copy sent", zh: "已发送副本" },
  "report_no_history": { fr: "Aucun rapport Z", en: "No Z reports", zh: "暂无Z报表" },
  "z_report_warning": {
    fr: "Rapport Z non effectué aujourd'hui",
    en: "Z-report not done today",
    zh: "今日未生成Z报表",
  },
  "printer_paper_out": { fr: "Papier épuisé", en: "Paper out", zh: "缺纸" },
  "printer_cover_open": { fr: "Capot ouvert", en: "Cover open", zh: "盖板打开" },
  "printer_check": { fr: "Vérifiez l'imprimante", en: "Check printer", zh: "检查打印机" },

  // Header status indicators (§8.1) — functional labels, not technical
  "status_register": { fr: "Caisse", en: "Register", zh: "收银" },
  "status_printer": { fr: "Ticket", en: "Printer", zh: "打印" },
  "status_orders": { fr: "Commandes", en: "Orders", zh: "订单" },

  // Settings
  "settings": { fr: "Paramètres", en: "Settings", zh: "设置" },
  "training_mode": { fr: "Mode Formation", en: "Training Mode", zh: "培训模式" },
  "training_desc": { fr: "Événements marqués T (non fiscaux)", en: "Events marked T (non-fiscal)", zh: "标记为T的事件（非财务）" },
  "active": { fr: "ACTIF", en: "ACTIVE", zh: "已开启" },
  "inactive": { fr: "INACTIF", en: "INACTIVE", zh: "未开启" },
  "language": { fr: "Langue", en: "Language", zh: "语言" },
  "language_desc": { fr: "Noms des produits et catégories", en: "Product and category names", zh: "产品和分类名称" },
  "employees": { fr: "Employés", en: "Employees", zh: "员工" },
  "add": { fr: "Ajouter", en: "Add", zh: "添加" },
  "name": { fr: "Nom", en: "Name", zh: "姓名" },
  "info": { fr: "Informations", en: "Information", zh: "信息" },
  "printer_none": { fr: "Non configurée", en: "Not configured", zh: "未配置" },

  // SettingsView — worker management + info rows
  "settings_name_placeholder": { fr: "Nom", en: "Name", zh: "姓名" },
  "settings_insz_placeholder": { fr: "INSZ (11 chiffres)", en: "INSZ (11 digits)", zh: "INSZ (11位)" },
  "settings_err_name_insz_required": { fr: "Nom et INSZ requis", en: "Name and INSZ required", zh: "姓名和INSZ必填" },
  "settings_err_11_digits_required": { fr: "11 chiffres requis", en: "11 digits required", zh: "必须为11位" },
  "settings_pos_id_label": { fr: "POS ID", en: "POS ID", zh: "POS ID" },
  "settings_terminal_label": { fr: "Terminal", en: "Terminal", zh: "终端" },
  "settings_restaurant_label": { fr: "Restaurant", en: "Restaurant", zh: "餐厅" },
  "settings_fdm_label": { fr: "FDM", en: "FDM", zh: "黑盒" },
  "settings_printer_label": { fr: "Imprimante", en: "Printer", zh: "打印机" },

  // SettingsView — HubRise section
  "hubrise_section_help": {
    fr: "Toutes les commandes (sur place, à emporter, en ligne) transitent par HubRise pour les rapports centralisés.",
    en: "All orders (dine-in, takeaway, online) go through HubRise for centralized reporting.",
    zh: "所有订单（堂食、外带、在线）均通过HubRise进行集中报表。",
  },
  "hubrise_status_label": { fr: "Statut", en: "Status", zh: "状态" },
  "hubrise_connected": { fr: "Connecté", en: "Connected", zh: "已连接" },
  "hubrise_disconnected": { fr: "Déconnecté", en: "Disconnected", zh: "未连接" },
  "hubrise_account_label": { fr: "Compte", en: "Account", zh: "账户" },
  "hubrise_location_label": { fr: "Établissement", en: "Location", zh: "店铺" },
  "hubrise_catalog_label": { fr: "Catalogue", en: "Catalog", zh: "菜单" },
  "hubrise_disconnect_action": { fr: "Déconnecter", en: "Disconnect", zh: "断开" },
  "hubrise_open_authorize_url": { fr: "Ouvrir l'URL d'autorisation HubRise", en: "Open HubRise authorize URL", zh: "打开HubRise授权链接" },
  "hubrise_paste_code_title": { fr: "Coller le code d'autorisation", en: "Paste authorization code", zh: "粘贴授权码" },
  "hubrise_code_placeholder": { fr: "Code d'autorisation", en: "Authorization code", zh: "授权码" },
  "hubrise_complete_connection": { fr: "Valider la connexion", en: "Complete connection", zh: "完成连接" },
  "hubrise_err_code_required": { fr: "Code requis", en: "Code required", zh: "授权码必填" },

  // Refund
  "refund": { fr: "Remboursement", en: "Refund", zh: "退款" },
  "refund_select_ticket": { fr: "Sélectionner le ticket", en: "Select ticket", zh: "选择票据" },
  "refund_select_items": { fr: "Articles à rembourser", en: "Items to refund", zh: "退款商品" },
  "refund_confirm": { fr: "Rembourser", en: "Refund", zh: "退款" },
  "refund_success": { fr: "Remboursement enregistré", en: "Refund recorded", zh: "退款已记录" },
  "refund_none": { fr: "Aucun ticket récent", en: "No recent tickets", zh: "无近期票据" },

  // Cash management (Event F)
  "cash_section": { fr: "Gestion caisse", en: "Cash management", zh: "现金管理" },
  "cash_in": { fr: "Entrée de caisse", en: "Cash in", zh: "现金入账" },
  "cash_out": { fr: "Sortie de caisse", en: "Cash out", zh: "现金出账" },
  "cash_open_drawer": { fr: "Ouvrir tiroir", en: "Open drawer", zh: "打开钱箱" },
  "cash_amount_placeholder": { fr: "Montant (€)", en: "Amount (€)", zh: "金额 (€)" },
  "cash_reason_placeholder": { fr: "Motif", en: "Reason", zh: "原因" },
  "cash_success": { fr: "Mouvement enregistré", en: "Movement recorded", zh: "已记录" },
  "cash_drawer_opened": { fr: "Tiroir ouvert", en: "Drawer opened", zh: "钱箱已打开" },
  "cash_err_amount": { fr: "Montant invalide", en: "Invalid amount", zh: "金额无效" },

  // Training banner
  "training_banner": { fr: "MODE FORMATION — Les transactions ne sont PAS enregistrées", en: "TRAINING MODE — Transactions are NOT recorded", zh: "培训模式 — 交易不会被记录" },
};

export function useI18n() {
  const pos = usePosStore();

  function t(key: string): string {
    const entry = messages[key];
    if (!entry) return key;
    return entry[pos.language] ?? entry.fr ?? key;
  }

  return { t };
}
