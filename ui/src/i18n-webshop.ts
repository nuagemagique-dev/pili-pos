// Webshop (HubRise) UI strings. Merged into the dictionary by i18n.ts
// only when __HUBRISE_ENABLED__, so the certification build drops them.
import type { Messages } from "./i18n";

export const webshopMessages: Messages = {
  // Settings, HubRise panel.
  "settings_hubrise_pending": { fr: "{count} en attente", en: "{count} pending", zh: "{count} 待处理" },
  "online_orders": { fr: "En ligne", en: "Online", zh: "在线" },
  // WebshopOrdersView - one column per stage. The terminal statuses
  // (completed / rejected / cancelled) share the "Terminées" view
  // behind a button in the "Prêtes" column head.
  "webshop_filter_done": { fr: "Terminées", en: "Done", zh: "已结束" },
  "webshop_col_new": { fr: "Nouvelles", en: "New", zh: "新订单" },
  "webshop_col_kitchen": { fr: "En cuisine", en: "In the kitchen", zh: "制作中" },
  "webshop_col_ready": { fr: "Prêtes", en: "Ready", zh: "待取" },
  // Key time at the top right of a card
  "webshop_time_ago": { fr: "il y a {n} min", en: "{n} min ago", zh: "{n} 分钟前" },
  "webshop_time_ready_at": { fr: "prêt à {time}", en: "ready at {time}", zh: "预计 {time}" },
  "webshop_time_ready": { fr: "prête {time}", en: "ready {time}", zh: "{time} 已备好" },
  // The restaurant's own webshop, in place of a channel logo
  "webshop_own_label": { fr: "Webshop", en: "Webshop", zh: "网店" },
  "webshop_delay_label": { fr: "Retarder", en: "Delay", zh: "延迟" },
  "webshop_action_reprint": { fr: "Réimprimer", en: "Reprint", zh: "重新打印" },

  // WebshopOrdersView - status shown on in-delivery and finished cards
  "webshop_status_in_delivery": { fr: "En livraison", en: "In delivery", zh: "配送中" },
  "webshop_status_completed": { fr: "Terminée", en: "Done", zh: "已完成" },
  "webshop_status_rejected": { fr: "Refusée", en: "Rejected", zh: "已拒绝" },
  "webshop_status_cancelled": { fr: "Annulée", en: "Cancelled", zh: "已取消" },

  // WebshopOrdersView - service type rendering
  "webshop_service_delivery": { fr: "Livraison", en: "Delivery", zh: "外卖" },
  "webshop_service_collection": { fr: "À emporter", en: "Pickup", zh: "自取" },
  "webshop_service_eat_in": { fr: "Sur place", en: "Eat in", zh: "堂食" },
  "webshop_service_unknown": { fr: "-", en: "-", zh: "-" },

  // WebshopOrdersView - primary + reject actions
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

  // WebshopOrdersView - row labels

  // WebshopOrdersView - state messages
  "webshop_loading": { fr: "Chargement…", en: "Loading…", zh: "加载中…" },
  "webshop_empty": {
    fr: "Aucune commande à afficher.",
    en: "No orders to show.",
    zh: "暂无订单",
  },

  // WebshopOrdersView - delay chips + print buttons + toasts
  "webshop_action_print_kitchen": { fr: "Ticket cuisine", en: "Kitchen ticket", zh: "厨房单" },
  "webshop_action_print_customer": { fr: "Ticket client", en: "Customer ticket", zh: "客单" },
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
  // SettingsView - HubRise section
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
  // Backend error codes of the HubRise integration (commands/errors.rs,
  // module `hubrise`).
  "err_hubrise_not_connected": { fr: "HubRise n'est pas connecté.", en: "HubRise is not connected.", zh: "HubRise未连接。" },
  "err_hubrise_order_unknown": { fr: "Commande en ligne inconnue.", en: "Unknown online order.", zh: "未知的在线订单。" },
  "err_hubrise_status_not_allowed": { fr: "Ce statut ne peut pas être choisi ici.", en: "This status cannot be set here.", zh: "此处不能设置该状态。" },
  "err_hubrise_time_invalid": { fr: "Heure non valable.", en: "Invalid time.", zh: "时间无效。" },
  "err_hubrise_menu_managed": { fr: "Le menu est géré par HubRise : modifiez-le là.", en: "The menu is managed in HubRise: edit it there.", zh: "菜单由HubRise管理，请在那里修改。" },
  "err_hubrise_open_orders": { fr: "Clôturez d'abord toutes les commandes ouvertes.", en: "Close every open order first.", zh: "请先结清所有未结订单。" },
  "err_webshop_training": { fr: "Pas de commande en ligne en mode formation.", en: "No online orders in training mode.", zh: "培训模式下不能处理在线订单。" },
  "err_webshop_already_handled": { fr: "Cette commande a déjà été traitée.", en: "This order was already handled.", zh: "该订单已处理。" },
  "err_webshop_needs_review": { fr: "Commande à vérifier : elle ne correspond pas au menu.", en: "Order to check: it does not match the menu.", zh: "订单需核对：与菜单不符。" },
  "err_webshop_not_awaiting_payment": { fr: "Cette commande n'attend pas de paiement.", en: "This order is not awaiting payment.", zh: "该订单无需收款。" },
  "err_webshop_not_cancelled": { fr: "Seule une commande signée puis annulée se rembourse ou se clôture.", en: "Only a signed order that was then cancelled is refunded or closed.", zh: "只有已签署后被取消的订单才能退款或结清。" },
  "err_webshop_payment_not_at_till": { fr: "Ce moyen de paiement n'est pas accepté en caisse.", en: "This payment method is not taken at the till.", zh: "收银台不接受此支付方式。" },
  "err_hubrise_channel_invalid": { fr: "Réglage de canal non valable.", en: "Invalid channel setting.", zh: "渠道设置无效。" },
  "err_hubrise_nothing_pending": { fr: "Aucun catalogue en attente.", en: "No catalog is waiting.", zh: "没有待处理的菜单。" },
};
