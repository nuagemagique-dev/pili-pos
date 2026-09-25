// Webshop (HubRise) UI strings. Merged into the dictionary by i18n.ts
// only when HUBRISE_ENABLED, so the certification build drops them.
import type { Messages } from "./i18n";

export const webshopMessages: Messages = {
  "online_orders": { fr: "En ligne", en: "Online", zh: "在线" },
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
  "status_orders": { fr: "Commandes", en: "Orders", zh: "订单" },
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
};
