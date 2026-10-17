import { computed } from "vue";
import { usePosStore } from "./stores/pos";
import type { AppLanguage } from "./stores/pos";
import { webshopMessages } from "./i18n-webshop";
import { checkoutMessages } from "./i18n-checkout";
import { reportsMessages } from "./i18n-reports";
import { settingsMessages } from "./i18n-settings";

export type Messages = Record<string, Record<AppLanguage, string>>;

const coreMessages: Messages = {
  // OrderView
  "tables": { fr: "Tables", en: "Tables", zh: "桌位" },
  "reports": { fr: "Rapports", en: "Reports", zh: "报表" },
  "config": { fr: "Config", en: "Config", zh: "设置" },
  "logout": { fr: "Sortir", en: "Logout", zh: "退出" },

  "table": { fr: "Table", en: "Table", zh: "桌" },

  // Cart
  "order": { fr: "Commande", en: "Order", zh: "订单" },
  "no_items": { fr: "Aucun article", en: "No items", zh: "暂无商品" },
  "total": { fr: "TOTAL", en: "TOTAL", zh: "合计" },
  "pay": { fr: "PAYER", en: "PAY", zh: "结账" },
  "pre_bill": { fr: "Addition", en: "Bill", zh: "账单" },
  "unpaid_close": { fr: "Client parti sans payer", en: "Customer left without paying", zh: "客人未付款离开" },
  "unpaid_hint": { fr: "La vente sera enregistrée sans paiement et son ticket imprimé. Confirmez avec votre PIN.", en: "The sale will be recorded without payment and its ticket printed. Confirm with your PIN.", zh: "该销售将以未付款登记并打印票据。请输入PIN确认。" },
  "unpaid_closed": { fr: "Vente clôturée sans paiement", en: "Sale closed without payment", zh: "销售已按未付款结束" },
  "pre_bill_printed": { fr: "Addition provisoire imprimée", en: "Provisional bill printed", zh: "临时账单已打印" },
  "pre_bill_failed": { fr: "Addition non enregistrée", en: "Bill not registered", zh: "账单未登记" },
  "order_register_failed": { fr: "Commande non enregistrée par le FDM", en: "Order not registered by the FDM", zh: "订单未被FDM登记" },
  "presence_failed": { fr: "Pointage non enregistré", en: "Clock-in/out not registered", zh: "打卡未登记" },

  // Checkout
  "cash": { fr: "CASH", en: "CASH", zh: "现金" },
  "card": { fr: "CARTE", en: "CARD", zh: "刷卡" },
  "meal_voucher": { fr: "CHÈQUES-REPAS", en: "MEAL VOUCHERS", zh: "餐券" },
  "takeaways_open": { fr: "À emporter en cours", en: "Takeaways in progress", zh: "进行中的外带" },
  "rest_to_pay": { fr: "Reste à payer", en: "Left to pay", zh: "剩余应付" },
  "clear_payments": { fr: "Annuler les paiements", en: "Cancel the payments", zh: "取消付款" },
  "amount": { fr: "Montant", en: "Amount", zh: "金额" },
  "amount_over_rest": { fr: "Plus que le reste à payer", en: "More than what is left to pay", zh: "超过剩余应付" },
  "exact": { fr: "Exact", en: "Exact", zh: "刚好" },
  "other": { fr: "Autre", en: "Other", zh: "其他" },
  "clear": { fr: "Effacer", en: "Clear", zh: "清除" },
  "validate": { fr: "Valider", en: "Confirm", zh: "确认" },
  "card_wait": { fr: "Encaisser sur le terminal", en: "Process on terminal", zh: "请在终端上刷卡" },
  "back": { fr: "Retour", en: "Back", zh: "返回" },
  "open_price_title": { fr: "Prix libre", en: "Open price", zh: "自定价格" },
  "open_price_badge": { fr: "Prix libre", en: "Open price", zh: "自定价" },
  "to_pay": { fr: "À payer", en: "To pay", zh: "应付" },
  "received": { fr: "Reçu", en: "Received", zh: "收到" },
  "change": { fr: "Rendu", en: "Change", zh: "找零" },
  "processing_fdm": { fr: "Communication FDM...", en: "FDM communication...", zh: "FDM通信中..." },
  "finish": { fr: "Terminer", en: "Done", zh: "完成" },
  "fdm_no_sig": { fr: "FDM non connecté : ticket sans signature", en: "FDM offline: ticket unsigned", zh: "FDM未连接：票据无签名" },
  "before_rounding": { fr: "avant arrondi", en: "before rounding", zh: "四舍五入前" },

  // Login / PIN
  "login": { fr: "Connexion", en: "Login", zh: "登录" },
  "pin_required": { fr: "PIN requis", en: "PIN required", zh: "需要PIN" },
  "pin_wrong": { fr: "PIN incorrect", en: "Wrong PIN", zh: "PIN错误" },
  "pin_not_set_warning": {
    fr: "PIN non configuré : touchez votre nom en haut de l'écran",
    en: "PIN not set: tap your name at the top of the screen",
    zh: "未设置PIN：请点击屏幕顶部的您的名字",
  },
  "pin_enter": { fr: "Saisir le PIN", en: "Enter PIN", zh: "输入PIN" },
  "pin_change": { fr: "Changer le PIN", en: "Change PIN", zh: "修改PIN" },
  "pin_changed": { fr: "PIN modifié", en: "PIN changed", zh: "PIN已修改" },
  "pin_confirm_manager": { fr: "Confirmez avec votre PIN de gérant", en: "Confirm with your manager PIN", zh: "请输入您的经理PIN确认" },

  // Remote support (commands/support.rs)
  "support_title": { fr: "Assistance à distance", en: "Remote support", zh: "远程协助" },
  "support_desc": {
    fr: "Démarre RustDesk : lisez au technicien l'ID et le mot de passe affichés. Arrêtez la session quand il a terminé.",
    en: "Starts RustDesk: read the ID and password it shows to the technician. Stop the session when they are done.",
    zh: "启动RustDesk：将显示的ID和密码告诉技术人员。完成后请结束会话。",
  },
  "support_start": { fr: "Démarrer", en: "Start", zh: "开始" },
  "support_stop": { fr: "Arrêter la session", en: "Stop the session", zh: "结束会话" },
  "support_running_since": { fr: "En cours depuis {time} ({name})", en: "Running since {time} ({name})", zh: "自{time}起进行中（{name}）" },
  "support_not_installed": { fr: "Non installée sur cette caisse", en: "Not installed on this terminal", zh: "此收银机未安装" },
  "support_banner_anonymous": {
    fr: "Assistance à distance en cours",
    en: "Remote support in progress",
    zh: "远程协助进行中",
  },
  "support_banner": {
    fr: "Assistance à distance en cours (démarrée par {name})",
    en: "Remote support in progress (started by {name})",
    zh: "远程协助进行中（由{name}启动）",
  },
  "support_started": { fr: "Assistance à distance démarrée", en: "Remote support started", zh: "远程协助已开始" },
  "support_stopped": { fr: "Assistance à distance arrêtée", en: "Remote support stopped", zh: "远程协助已结束" },

  // Backend error codes (src-tauri/src/commands/errors.rs), shown by
  // utils/errorMessages.ts::describeError.
  "err_failed": { fr: "L'opération a échoué.", en: "The operation failed.", zh: "操作失败。" },
  "err_unexpected": {
    fr: "Erreur inattendue. Contactez votre technicien. ({detail})",
    en: "Unexpected error. Contact your technician. ({detail})",
    zh: "意外错误，请联系技术人员。（{detail}）",
  },
  "err_not_logged_in": { fr: "Session fermée : reconnectez-vous.", en: "Session closed: log in again.", zh: "会话已关闭：请重新登录。" },
  "err_forbidden": { fr: "Action réservée à un autre profil.", en: "This action is reserved for another role.", zh: "此操作仅限其他角色。" },
  "err_not_the_operator": { fr: "Action au nom d'un autre employé refusée.", en: "Acting for another employee is refused.", zh: "不能以其他员工的名义操作。" },
  "err_insz_invalid": { fr: "INSZ invalide", en: "Invalid INSZ", zh: "INSZ无效" },
  "err_user_unknown": { fr: "Utilisateur inconnu", en: "Unknown user", zh: "未知用户" },
  "err_pin_required": { fr: "PIN requis", en: "PIN required", zh: "需要PIN" },
  "err_pin_wrong": { fr: "PIN incorrect", en: "Wrong PIN", zh: "PIN错误" },
  "err_pin_locked": { fr: "Trop de PIN incorrects : réessayez dans {seconds} s.", en: "Too many wrong PINs: try again in {seconds} s.", zh: "PIN错误次数过多：请{seconds}秒后重试。" },
  "err_pin_format": { fr: "Le PIN doit compter exactement 4 chiffres.", en: "The PIN must be exactly 4 digits.", zh: "PIN必须为4位数字。" },
  "err_pin_setup_required": { fr: "Un gérant doit choisir un PIN.", en: "A manager must choose a PIN.", zh: "经理必须设置PIN。" },
  "err_pin_already_set": { fr: "Ce compte a déjà un PIN.", en: "This account already has a PIN.", zh: "该账户已有PIN。" },
  "err_first_pin_needs_manager": { fr: "Un gérant doit choisir le PIN de ce compte dans Paramètres.", en: "A manager must set this account's PIN in Settings.", zh: "此账户的PIN须由经理在设置中设定。" },
  "err_system_user": { fr: "L'identité système ne peut pas se connecter.", en: "The system identity cannot log in.", zh: "系统身份无法登录。" },
  "err_name_required": { fr: "Nom requis", en: "Name required", zh: "需要姓名" },
  "err_role_invalid": { fr: "Profil inconnu", en: "Unknown role", zh: "未知角色" },
  "err_deactivate_self": { fr: "Vous ne pouvez pas vous désactiver vous-même.", en: "You cannot deactivate yourself.", zh: "不能停用自己。" },
  "err_presence_only_user": { fr: "Employé enregistré pour la présence seulement : pas de connexion à la caisse", en: "This employee is registered for presence only and cannot log in", zh: "该员工仅用于考勤，不能登录收银机" },
  "err_presence_training": { fr: "Mode formation : le pointage d'un employé est impossible. Quittez le mode formation.", en: "Training mode: an employee cannot be clocked in or out. Leave training mode.", zh: "培训模式下不能为员工打卡，请先退出培训模式。" },
  "err_presence_self": { fr: "Votre arrivée et votre départ se font à la connexion et par « Fin de service »", en: "Your own clock-in and clock-out happen at login and with \"End of shift\"", zh: "您本人的上下班在登录时和\"下班\"时登记" },
  "err_presence_already_in": { fr: "Cet employé est déjà en service", en: "This employee is already on shift", zh: "该员工已在岗" },
  "err_presence_not_in": { fr: "Cet employé n'est pas en service", en: "This employee is not on shift", zh: "该员工不在岗" },
  "err_badge_reader_off": { fr: "Le lecteur de badges est désactivé", en: "The badge reader is off", zh: "工牌读卡器已关闭" },
  "err_badge_token_invalid": { fr: "Badge illisible : présentez-le de nouveau", en: "Badge not readable: present it again", zh: "无法读取工牌，请重新出示" },
  "err_badge_key_unavailable": { fr: "Clé des badges inaccessible : contactez l'assistance", en: "Badge key unavailable: contact support", zh: "工牌密钥不可用，请联系技术支持" },
  "err_badge_unknown": { fr: "Badge inconnu", en: "Unknown badge", zh: "未知工牌" },
  "err_badge_revoked": { fr: "Badge retiré : demandez un nouveau badge au Gérant", en: "Badge removed: ask the manager for a new one", zh: "工牌已解除，请向经理申请新工牌" },
  "err_badge_in_use": { fr: "Ce badge est déjà lié à un autre employé", en: "This badge is already linked to another employee", zh: "该工牌已绑定其他员工" },
  "err_badge_already_linked": { fr: "Cet employé a déjà un badge : retirez-le d'abord", en: "This employee already has a badge: remove it first", zh: "该员工已有工牌，请先解除" },
  "err_badge_none": { fr: "Cet employé n'a pas de badge", en: "This employee has no badge", zh: "该员工没有工牌" },
  "err_badge_settings_invalid": { fr: "Réglages du lecteur de badges invalides", en: "Invalid badge reader settings", zh: "工牌读卡器设置无效" },
  "err_deactivate_on_shift": { fr: "Cet employé est en service : enregistrez d'abord son départ", en: "This employee is on shift: clock them out first", zh: "该员工在岗，请先登记下班" },
  "err_operator_invalid": { fr: "Employé invalide : reconnectez-vous.", en: "Invalid employee: log in again.", zh: "员工无效：请重新登录。" },
  "err_dine_in_needs_table": { fr: "Une commande sur place a besoin d'une table.", en: "A dine-in order needs a table.", zh: "堂食订单需要桌号。" },
  "err_takeaway_has_no_table": { fr: "Une commande à emporter n'a pas de table.", en: "A takeaway order has no table.", zh: "外带订单没有桌号。" },
  "err_table_plan_invalid": { fr: "Écrivez les numéros de table comme ceci : 1-12, 20-25 (de 1 à 999, 200 tables au plus).", en: "Write the table numbers like this: 1-12, 20-25 (1 to 999, at most 200 tables).", zh: "请按此格式填写桌号：1-12, 20-25（1至999，最多200张桌）。" },
  "err_quantity_positive": { fr: "La quantité doit être d'au moins 1.", en: "The quantity must be at least 1.", zh: "数量至少为1。" },
  "err_order_not_open": { fr: "Cette commande est déjà clôturée.", en: "This order is already closed.", zh: "该订单已结束。" },
  "err_price_positive": { fr: "Saisissez un prix supérieur à zéro.", en: "Enter a price greater than zero.", zh: "请输入大于零的价格。" },
  "err_fixed_price": { fr: "Le produit {product} a un prix fixe.", en: "Product {product} has a fixed price.", zh: "商品{product}为固定价格。" },
  "err_product_unavailable": { fr: "Le produit {product} n'est pas disponible.", en: "Product {product} is not available.", zh: "商品{product}不可用。" },
  "err_line_not_found": { fr: "Cette ligne n'existe plus.", en: "This line no longer exists.", zh: "该行已不存在。" },
  "err_order_no_lines": { fr: "La commande est vide.", en: "The order is empty.", zh: "订单为空。" },
  "err_payment_invalid": { fr: "Paiement non valable : vérifiez les montants.", en: "Invalid payment: check the amounts.", zh: "付款无效：请检查金额。" },
  "err_payment_exceeds_total": { fr: "Les paiements dépassent le total.", en: "The payments exceed the total.", zh: "付款超过总额。" },
  "err_payment_incomplete": { fr: "Il reste un montant à payer.", en: "An amount is still to be paid.", zh: "仍有金额未付。" },
  "err_unpaid_nothing_due": { fr: "Rien à payer : encaissez la vente normalement.", en: "Nothing to pay: take the sale normally.", zh: "无需付款：请正常结账。" },
  "err_refund_unpaid_sale": { fr: "Cette vente n'a pas été payée : rien à rembourser.", en: "This sale was not paid: nothing to refund.", zh: "此笔销售未付款：无可退款。" },
  "err_takeaway_not_registered": { fr: "Une vente à emporter se termine par un paiement.", en: "A takeaway ends with a payment.", zh: "外带订单须以付款结束。" },
  "err_order_total_negative": { fr: "Le total ne peut pas être négatif.", en: "The total cannot be negative.", zh: "总额不能为负数。" },
  "err_amount_negative": { fr: "Le montant ne peut pas être négatif.", en: "The amount cannot be negative.", zh: "金额不能为负数。" },
  "err_previous_day_open": { fr: "La journée précédente n'est pas clôturée : son rapport Z automatique est en cours. Réessayez dans une minute.", en: "The previous day is not closed: its automatic Z report is under way. Try again in a minute.", zh: "前一天尚未结账：正在自动生成Z报表。请一分钟后重试。" },
  "err_order_already_closed": { fr: "Cette commande est déjà payée.", en: "This order is already paid.", zh: "该订单已付款。" },
  "err_event_not_found": { fr: "Ticket introuvable.", en: "Ticket not found.", zh: "未找到票据。" },
  "err_original_incomplete": {
    fr: "Le ticket d'origine est incomplet ({field}). Contactez votre technicien.",
    en: "The original ticket is incomplete ({field}). Contact your technician.",
    zh: "原始票据不完整（{field}），请联系技术人员。",
  },
  "err_event_label_unknown": { fr: "Type de ticket inconnu ({label}).", en: "Unknown ticket type ({label}).", zh: "未知票据类型（{label}）。" },
  "err_nothing_to_refund": { fr: "Choisissez au moins un article à rembourser.", en: "Choose at least one item to refund.", zh: "请至少选择一件退款商品。" },
  "err_not_refundable": { fr: "Seul un ticket de vente se rembourse (ici : {label}).", en: "Only a sale ticket can be refunded (here: {label}).", zh: "只能对销售票据退款（此处：{label}）。" },
  "err_refund_quantity_positive": { fr: "Quantité à rembourser invalide (produit {product}).", en: "Invalid refund quantity (product {product}).", zh: "退款数量无效（商品{product}）。" },
  "err_refund_product_not_sold": { fr: "Le produit {product} n'est pas sur le ticket d'origine.", en: "Product {product} is not on the original ticket.", zh: "商品{product}不在原始票据上。" },
  "err_refund_exceeds_sold": {
    fr: "Produit {product} : {requested} demandé(s), {sold} vendu(s), {refunded} déjà remboursé(s).",
    en: "Product {product}: {requested} requested, {sold} sold, {refunded} already refunded.",
    zh: "商品{product}：申请{requested}，已售{sold}，已退{refunded}。",
  },
  "err_print_original_unsupported": { fr: "Seul le ticket d'une vente ou d'une copie peut être imprimé ici.", en: "Only a sale's or a copy's ticket can be printed here.", zh: "此处只能打印销售或副本的票据。" },
  "err_refund_line_ambiguous": { fr: "Le produit {product} a été vendu à plusieurs prix : choisissez la ligne à rembourser.", en: "Product {product} was sold at several prices: pick the line to refund.", zh: "商品{product}以多个价格售出：请选择要退款的行。" },
  "err_refund_in_training": { fr: "Une vente réelle ne se rembourse pas en mode formation.", en: "A real sale cannot be refunded in training mode.", zh: "培训模式下不能退款真实销售。" },
  "err_refund_of_refund": { fr: "Un remboursement ne se rembourse pas.", en: "A refund cannot be refunded.", zh: "退款不能再退款。" },
  "err_open_orders_training": {
    fr: "Encaissez ou videz les commandes ouvertes avant de changer de mode",
    en: "Pay or empty the open orders before switching mode",
    zh: "切换模式前请结清或清空未结订单",
  },
  "err_eft_not_configured": { fr: "Aucun terminal de paiement n'est relié à cette caisse.", en: "No payment terminal is linked to this till.", zh: "此收银机未连接支付终端。" },
  "err_eft_training": { fr: "Mode formation : aucune carte réelle n'est débitée. Encodez le paiement par carte à la main.", en: "Training mode: no real card is charged. Enter the card payment by hand.", zh: "培训模式：不会从真实银行卡扣款。请手动登记刷卡付款。" },
  "err_eft_amount_invalid": { fr: "Montant invalide : il doit être positif et ne pas dépasser ce qu'il reste à payer.", en: "Invalid amount: it must be positive and not more than what is left to pay.", zh: "金额无效：必须为正数，且不得超过剩余应付金额。" },
  "err_eft_pending": { fr: "Un paiement par carte attend encore sa réponse : vérifiez-le d'abord. Rien d'autre n'est encaissé ni enregistré.", en: "A card payment is still waiting for its answer: check it first. Nothing else is charged or recorded.", zh: "有一笔刷卡付款仍在等待结果：请先查询。在此之前不会收取或登记其他款项。" },
  "err_eft_busy": { fr: "Le terminal de paiement est occupé avec un autre paiement. Rien n'a été débité.", en: "The payment terminal is busy with another payment. Nothing was charged.", zh: "支付终端正忙于另一笔付款。未扣款。" },
  "err_eft_unreachable": { fr: "Le terminal de paiement est injoignable. Rien n'a été débité. ({detail})", en: "The payment terminal cannot be reached. Nothing was charged. ({detail})", zh: "无法连接支付终端。未扣款。（{detail}）" },
  "err_eft_rejected": { fr: "Le terminal de paiement a refusé la demande. Rien n'a été débité. ({detail})", en: "The payment terminal refused the request. Nothing was charged. ({detail})", zh: "支付终端拒绝了请求。未扣款。（{detail}）" },
  "err_eft_payment_not_usable": { fr: "Ce paiement par carte ne peut pas être utilisé ici : utilisez chaque paiement approuvé de cette commande, pour son montant exact.", en: "This card payment cannot be used here: use every approved payment of this order, for its exact amount.", zh: "此刷卡付款无法在此使用：请按准确金额使用本订单的每一笔已批准付款。" },
  "err_eft_below_charged": { fr: "La carte du client a déjà été débitée de {amount} € : la commande ne peut pas descendre sous ce montant.", en: "The customer's card has already been charged {amount} €: the order cannot go below that amount.", zh: "客户的银行卡已被扣款 {amount} €：订单金额不能低于此数。" },
  "err_eft_edit_pending": { fr: "Un paiement par carte attend encore sa réponse : vérifiez-le d'abord. La commande ne peut pas être modifiée entre-temps.", en: "A card payment is still waiting for its answer: check it first. The order cannot be changed meanwhile.", zh: "有一笔刷卡付款仍在等待结果：请先查询。在此之前不能修改订单。" },
  "err_digital_unavailable": { fr: "Le ticket par e-mail n'est pas configuré sur cette caisse.", en: "Email tickets are not set up on this terminal.", zh: "此收银机未配置电子邮件票据。" },
  "err_email_invalid": { fr: "Adresse e-mail invalide.", en: "Invalid email address.", zh: "电子邮件地址无效。" },
  "err_already_printed": { fr: "Le ticket original est déjà imprimé : imprimez une copie.", en: "The original ticket is already printed: print a copy.", zh: "原始票据已打印：请打印副本。" },
  "err_email_sent": { fr: "Le ticket a été envoyé par e-mail : imprimez une copie.", en: "The ticket was sent by email: print a copy.", zh: "票据已通过电子邮件发送：请打印副本。" },
  "err_printer_not_configured": { fr: "Aucune imprimante configurée.", en: "No printer configured.", zh: "未配置打印机。" },
  "err_print_failed": { fr: "L'impression a échoué. Vérifiez l'imprimante. ({detail})", en: "Printing failed. Check the printer. ({detail})", zh: "打印失败，请检查打印机。（{detail}）" },
  "err_support_unavailable": { fr: "L'assistance à distance n'est pas installée sur cette caisse.", en: "Remote support is not installed on this terminal.", zh: "此收银机未安装远程协助。" },
  "err_support_running": { fr: "Une session d'assistance est déjà en cours.", en: "A support session is already running.", zh: "远程协助会话已在进行中。" },
  "err_support_start_failed": { fr: "L'assistance à distance n'a pas démarré. ({detail})", en: "Remote support did not start. ({detail})", zh: "远程协助未能启动。（{detail}）" },
  "err_signing_busy": { fr: "La caisse enregistre une autre opération : réessayez dans un instant.", en: "The till is recording another operation: try again in a moment.", zh: "收银机正在登记另一操作：请稍后重试。" },
  "err_order_sign_pending": { fr: "Cette commande attend la réponse de la boîte noire : elle ne peut pas changer pour l'instant.", en: "This order waits for the FDM's answer: it cannot change for now.", zh: "此订单正在等待黑匣子的回应：暂时不能修改。" },
  "err_period_closed": { fr: "La journée de cet enregistrement est déjà clôturée : rien n'a été enregistré.", en: "The day of this record is already closed: nothing was recorded.", zh: "此记录所属的营业日已结账：未登记任何内容。" },
  "err_concurrent_operation": { fr: "Une autre opération était en cours : rien n'a été enregistré, réessayez.", en: "Another operation was in progress: nothing was recorded, try again.", zh: "另一操作正在进行：未记录任何内容，请重试。" },
  "err_config_tables": { fr: "Plan de salle invalide : indiquez des numéros et des plages, par exemple « 1-12, 20-25 ». Détail : {detail}", en: "Invalid table plan: numbers and ranges such as \"1-12, 20-25\" expected. Detail: {detail}", zh: "桌位设置无效：请填写编号和范围，例如“1-12, 20-25”。 详情：{detail}" },
  "err_config_pos_id": { fr: "Identifiant de la caisse (pos_id) invalide : 14 majuscules ou chiffres. Détail : {detail}", en: "Invalid till identifier (pos_id): 14 upper-case letters or digits. Detail: {detail}", zh: "收银机标识（pos_id）无效：需要14位大写字母或数字。 详情：{detail}" },
  "err_config_restaurant_vat": { fr: "Numéro de TVA du restaurant invalide : BE suivi de 10 chiffres (par exemple BE0499999960). Détail : {detail}", en: "Invalid restaurant VAT number: BE followed by 10 digits (for example BE0499999960). Detail: {detail}", zh: "餐厅增值税号无效：BE 后跟10位数字（例如 BE0499999960）。 详情：{detail}" },
  "err_config_restaurant_est_no": { fr: "Numéro d'unité d'établissement invalide : 10 chiffres. Détail : {detail}", en: "Invalid establishment unit number: 10 digits. Detail: {detail}", zh: "营业单位编号无效：需要10位数字。 详情：{detail}" },
  "err_config_identity": { fr: "Identifiant du terminal ou de l'appareil invalide. Détail : {detail}", en: "Invalid terminal or device identifier. Detail: {detail}", zh: "终端或设备标识无效。 详情：{detail}" },
  "err_config_ticket_header": { fr: "Le nom ou l'adresse du restaurant, imprimés sur chaque ticket, ne peuvent pas être vides. Détail : {detail}", en: "The restaurant name or address, printed on every ticket, must not be empty. Detail: {detail}", zh: "每张小票上打印的餐厅名称或地址不能为空。 详情：{detail}" },
  "err_config_fdm_url": { fr: "Adresse de la boîte noire (fdm_url) invalide : une adresse http ou https est attendue. Détail : {detail}", en: "Invalid FDM address (fdm_url): an http or https address is expected. Detail: {detail}", zh: "黑匣子地址（fdm_url）无效：需要 http 或 https 地址。 详情：{detail}" },
  "err_config_fdm_files": { fr: "Fichiers de la boîte noire (jeton, certificats) manquants ou inutilisables. Détail : {detail}", en: "FDM files (token, certificates) missing or unusable. Detail: {detail}", zh: "黑匣子文件（令牌、证书）缺失或无法使用。 详情：{detail}" },
  "err_config_language": { fr: "Langue de la configuration invalide : fr, nl, de, en ou zh. Détail : {detail}", en: "Invalid language setting: fr, nl, de, en or zh. Detail: {detail}", zh: "语言设置无效：fr、nl、de、en 或 zh。 详情：{detail}" },
  "err_config_day_end_hour": { fr: "Heure de fin de journée invalide : de 0 à 12. Détail : {detail}", en: "Invalid end-of-day hour: 0 to 12. Detail: {detail}", zh: "日结时间无效：0到12。 详情：{detail}" },
  "err_config_missing": { fr: "Aucune configuration : l'identité de développement n'est pas acceptable sur une caisse installée. Détail : {detail}", en: "No configuration: the development identity is not acceptable on an installed till. Detail: {detail}", zh: "没有配置：已安装的收银机不能使用开发身份。 详情：{detail}" },
  "err_config_hubrise": { fr: "Réglage HubRise invalide (identifiant client, fichier du secret ou adresses). Détail : {detail}", en: "Invalid HubRise setting (client id, secret file or addresses). Detail: {detail}", zh: "HubRise 设置无效（客户端标识、密钥文件或地址）。 详情：{detail}" },
  "err_config_digital_ticket": { fr: "Réglage du ticket par email invalide. Détail : {detail}", en: "Invalid e-mail ticket setting. Detail: {detail}", zh: "电子邮件小票设置无效。 详情：{detail}" },
  "err_config_export": { fr: "Réglage de l'export nocturne invalide. Détail : {detail}", en: "Invalid nightly export setting. Detail: {detail}", zh: "夜间导出设置无效。 详情：{detail}" },
  "err_config_payment_terminal": { fr: "Réglage du terminal de paiement invalide. Détail : {detail}", en: "Invalid payment terminal setting. Detail: {detail}", zh: "支付终端设置无效。 详情：{detail}" },
  "err_config_other": { fr: "Configuration invalide ou illisible. Détail : {detail}", en: "Invalid or unreadable configuration. Detail: {detail}", zh: "配置无效或无法读取。 详情：{detail}" },
  "err_internal_error": { fr: "Erreur interne. Contactez votre technicien. ({detail})", en: "Internal error. Contact your technician. ({detail})", zh: "内部错误，请联系技术人员。（{detail}）" },
  "pin_choose": { fr: "Choisissez votre PIN", en: "Choose your PIN", zh: "设置您的PIN" },
  "pin_chosen": { fr: "PIN enregistré", en: "PIN saved", zh: "PIN已保存" },
  "pin_first_login_hint": {
    fr: "Première connexion d'un gérant : choisissez un PIN à 4 chiffres. Il sera demandé à chaque connexion.",
    en: "First login of a manager: choose a 4-digit PIN. It will be asked at every login.",
    zh: "经理首次登录：请设置4位PIN，以后每次登录都需要输入。",
  },
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
  "confirm_z": { fr: "Confirmer la clôture ?", en: "Confirm the day close?", zh: "确认日结？" },
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

  // Header status indicators (§8.1) - functional labels, not technical
  "status_register": { fr: "Caisse", en: "Register", zh: "收银" },
  "status_printer": { fr: "Ticket", en: "Printer", zh: "打印" },

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
  "role": { fr: "Rôle", en: "Role", zh: "角色" },
  "config_problems": { fr: "Configuration invalide : le FDM refusera les tickets. Appelez l'installateur.", en: "Invalid configuration: the FDM will refuse tickets. Call the installer.", zh: "配置无效：FDM 将拒绝票据。请联系安装人员。" },
  "deactivate": { fr: "Désactiver", en: "Deactivate", zh: "停用" },
  "confirm_deactivate": { fr: "Confirmer ?", en: "Confirm?", zh: "确认？" },
  "role_admin": { fr: "Gérant", en: "Manager", zh: "经理" },
  "role_cashier": { fr: "Serveur", en: "Server", zh: "服务员" },
  "role_waiter": { fr: "Apprenti", en: "Apprentice", zh: "学徒" },
  "role_presence": { fr: "Présence seule", en: "Presence only", zh: "仅考勤" },
  "info": { fr: "Informations", en: "Information", zh: "信息" },
  "printer_none": { fr: "Non configurée", en: "Not configured", zh: "未配置" },

  // SettingsView - worker management + info rows
  "settings_name_placeholder": { fr: "Nom", en: "Name", zh: "姓名" },
  "settings_insz_placeholder": { fr: "INSZ (11 chiffres)", en: "INSZ (11 digits)", zh: "INSZ (11位)" },
  "settings_err_name_insz_required": { fr: "Nom et INSZ requis", en: "Name and INSZ required", zh: "姓名和INSZ必填" },
  "settings_err_11_digits_required": { fr: "11 chiffres requis", en: "11 digits required", zh: "必须为11位" },
  "settings_pos_id_label": { fr: "POS ID", en: "POS ID", zh: "POS ID" },
  "settings_terminal_label": { fr: "Terminal", en: "Terminal", zh: "终端" },
  "settings_restaurant_label": { fr: "Restaurant", en: "Restaurant", zh: "餐厅" },
  "settings_fdm_label": { fr: "FDM", en: "FDM", zh: "黑盒" },
  "settings_printer_label": { fr: "Imprimante", en: "Printer", zh: "打印机" },

  // Refund
  "copy_ticket": { fr: "Copie de ticket", en: "Ticket copy", zh: "票据副本" },
  "copy_select_ticket": { fr: "Ticket à copier", en: "Ticket to copy", zh: "选择要复制的票据" },
  "copy_none": { fr: "Aucun ticket récent", en: "No recent ticket", zh: "没有最近的票据" },
  "copy_kind_sale": { fr: "Vente", en: "Sale", zh: "销售" },
  "copy_kind_refund": { fr: "Remboursement", en: "Refund", zh: "退款" },
  "copy_kind_pre_bill": { fr: "Addition", en: "Bill", zh: "账单" },
  "copy_kind_training": { fr: "Formation", en: "Training", zh: "培训" },
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
  "cash_declaration": { fr: "Déclarer le contenu du tiroir", en: "Declare the drawer count", zh: "申报钱箱现金" },
  "cash_declaration_hint": { fr: "Montant compté dans le tiroir", en: "Amount counted in the drawer", zh: "钱箱中清点的金额" },
  "kitchen_slip_failed": { fr: "Bon de commande non imprimé : prévenez le bar ou la cuisine.", en: "Order slip not printed: tell the bar or kitchen.", zh: "点单小票未打印：请通知吧台或厨房。" },
  "cash_declared": { fr: "Contenu du tiroir déclaré", en: "Drawer count declared", zh: "钱箱现金已申报" },
  "cash_amount_placeholder": { fr: "Montant (€)", en: "Amount (€)", zh: "金额 (€)" },
  "cash_reason_placeholder": { fr: "Motif", en: "Reason", zh: "原因" },
  "cash_success": { fr: "Mouvement enregistré", en: "Movement recorded", zh: "已记录" },
  "cash_failed": { fr: "Mouvement non enregistré", en: "Movement not recorded", zh: "未记录" },
  "cash_drawer_opened": { fr: "Tiroir ouvert", en: "Drawer opened", zh: "钱箱已打开" },
  "cash_err_amount": { fr: "Montant invalide", en: "Invalid amount", zh: "金额无效" },

  // Training banner
  "training_banner": { fr: "MODE FORMATION · Les transactions ne sont PAS enregistrées", en: "TRAINING MODE · Transactions are NOT recorded", zh: "培训模式 · 交易不会被记录" },

  // Ticket delivery - medium picker (checkout / refund / copy)
  "delivery_paper": { fr: "Papier", en: "Paper", zh: "纸质" },
  "delivery_email": { fr: "E-mail", en: "Email", zh: "电子邮件" },
  "delivery_both": { fr: "Les deux", en: "Both", zh: "两者都要" },
  "delivery_unreachable_note": {
    fr: "Service e-mail indisponible : le ticket papier sera imprimé, l'e-mail sera envoyé dès que possible.",
    en: "Email service unavailable: the paper ticket will print now, the email will send later.",
    zh: "邮件服务暂不可用：将立即打印纸质票据，邮件稍后发送。",
  },

  // Ticket delivery - email keypad
  "delivery_enter_email_title": { fr: "Adresse e-mail du client", en: "Customer's email address", zh: "客户电子邮箱" },
  "delivery_email_placeholder": { fr: "Saisir l'adresse…", en: "Enter address…", zh: "输入邮箱地址…" },
  "delivery_invalid_email": { fr: "Adresse invalide", en: "Invalid address", zh: "地址无效" },
  "delivery_confirm_title": { fr: "Confirmer l'adresse", en: "Confirm the address", zh: "确认地址" },
  "delivery_confirm_btn": { fr: "Confirmer l'adresse", en: "Confirm the address", zh: "确认地址" },
  "delivery_edit": { fr: "Modifier", en: "Edit", zh: "修改" },
  "delivery_cancel": { fr: "Annuler", en: "Cancel", zh: "取消" },

  // Ticket delivery - outcome (done screen, copy/refund results)
  "delivery_outcome_printed": { fr: "Ticket imprimé", en: "Ticket printed", zh: "票据已打印" },
  "delivery_outcome_sent": { fr: "Ticket envoyé à {hint}", en: "Ticket sent to {hint}", zh: "票据已发送至{hint}" },
  "delivery_outcome_queued": {
    fr: "E-mail en attente, nouvel essai automatique",
    en: "Email pending, automatic retry",
    zh: "邮件待发送，将自动重试",
  },
  "delivery_print_missing": { fr: "Imprimer le ticket", en: "Print ticket", zh: "打印票据" },

  // Ticket delivery - Settings "Tickets numériques" panel
  "delivery_settings_title": { fr: "Tickets numériques", en: "Digital tickets", zh: "电子票据" },
  "delivery_settings_refresh": { fr: "Actualiser", en: "Refresh", zh: "刷新" },
  "delivery_settings_configured": { fr: "Configuré", en: "Configured", zh: "已配置" },
  "delivery_settings_not_configured": { fr: "Non configuré", en: "Not configured", zh: "未配置" },
  "delivery_settings_reachable": { fr: "Joignable", en: "Reachable", zh: "可连接" },
  "delivery_settings_unreachable": { fr: "Injoignable", en: "Unreachable", zh: "无法连接" },
  "delivery_settings_empty": { fr: "Aucun envoi récent", en: "No recent deliveries", zh: "暂无发送记录" },
  "delivery_attempts": { fr: "Tentatives", en: "Attempts", zh: "尝试次数" },
  "delivery_last_error": { fr: "Dernière erreur", en: "Last error", zh: "最后错误" },
  "delivery_status_pending": { fr: "En attente", en: "Pending", zh: "待处理" },
  "delivery_status_sending": { fr: "Envoi…", en: "Sending…", zh: "发送中…" },
  "delivery_status_sent": { fr: "Envoyé", en: "Sent", zh: "已发送" },
  "delivery_status_failed": { fr: "Échec", en: "Failed", zh: "失败" },
  "delivery_status_fallback_printed": { fr: "Imprimé (secours)", en: "Printed (fallback)", zh: "已打印（备用）" },
  "delivery_status_purged": { fr: "Purgé", en: "Purged", zh: "已清除" },

  // Pili header, device lights and FDM alerts
  "sw_version": { fr: "Version", en: "Version", zh: "版本" },
  "lock": { fr: "Verrouiller", en: "Lock", zh: "锁屏" },
  "status_terminal": { fr: "Terminal", en: "Terminal", zh: "刷卡机" },
  "status_kitchen": { fr: "Cuisine", en: "Kitchen", zh: "厨房" },
  "fdm_closes_in": { fr: "se ferme dans {s} s", en: "closes in {s} s", zh: "{s} 秒后关闭" },
  "fdm_retry_in": { fr: "nouvel essai automatique dans {s} s", en: "automatic retry in {s} s", zh: "{s} 秒后自动重试" },
  "fdm_retry_now": { fr: "Réessayer maintenant", en: "Retry now", zh: "立即重试" },

  // Order screen (Pili)
  "vat": { fr: "TVA", en: "VAT", zh: "增值税" },
  "qty_less": { fr: "Un de moins", en: "One less", zh: "减一" },
  "qty_more": { fr: "Un de plus", en: "One more", zh: "加一" },
  "line_remove": { fr: "Retirer la ligne", en: "Remove the line", zh: "删除此行" },
  "not_sent_to_kitchen": { fr: "Pas encore en cuisine", en: "Not sent to the kitchen yet", zh: "未送厨房" },
  "send_to_kitchen": { fr: "Envoyer en cuisine", en: "Send to kitchen", zh: "送厨房" },
  "sent_to_kitchen": { fr: "Envoyé en cuisine · {n} articles", en: "Sent to the kitchen · {n} items", zh: "已送厨房 · {n} 件" },

  // Tables screen (Pili)
  "takeaway_started_at": { fr: "Commencée à {time}", en: "Started at {time}", zh: "{time} 开始" },
  "takeaway_start": { fr: "Nouvelle vente à emporter", en: "New takeaway sale", zh: "新外带订单" },

  // Login screen (Pili)
  "who_are_you": { fr: "Qui êtes-vous ?", en: "Who are you?", zh: "您是谁？" },
  "pin_code": { fr: "Code PIN", en: "PIN code", zh: "PIN码" },
  "pin_auto_hint": { fr: "connexion au 4ᵉ chiffre", en: "logs in at the 4th digit", zh: "输入第4位数字即登录" },
  "other_person_insz": { fr: "Autre personne (INSZ)", en: "Someone else (INSZ)", zh: "其他人 (INSZ)" },
  "on_shift_since": { fr: "en service · {time}", en: "on shift · {time}", zh: "上班中 · {time}" },
  "off_shift": { fr: "hors service", en: "off shift", zh: "未上班" },
  "end_shift": { fr: "Fin de service", en: "End shift", zh: "下班" },
  "end_shift_hint": { fr: "le code PIN termine le service", en: "the PIN ends the shift", zh: "输入PIN即下班" },
  "shift_ended": { fr: "Service terminé · {name}", en: "Shift ended · {name}", zh: "已下班 · {name}" },
  "by_nuage_magique": { fr: "par Nuage Magique", en: "by Nuage Magique", zh: "Nuage Magique 出品" },
  "check_title": { fr: "Démarrage", en: "Start-up", zh: "启动检查" },
  "check_ticket_printer": { fr: "Imprimante ticket", en: "Ticket printer", zh: "小票打印机" },
  "check_kitchen_printer": { fr: "Imprimante cuisine", en: "Kitchen printer", zh: "厨房打印机" },
  "check_payment_terminal": { fr: "Terminal de paiement", en: "Payment terminal", zh: "支付终端" },
  "check_payment_busy": { fr: "paiement en cours", en: "payment in progress", zh: "支付进行中" },
  "check_email": { fr: "Email tickets", en: "Email receipts", zh: "电子小票" },
  "check_connected": { fr: "connecté", en: "connected", zh: "已连接" },
  "check_not_connected": { fr: "non connecté", en: "not connected", zh: "未连接" },
  "check_offline": { fr: "hors ligne", en: "offline", zh: "离线" },
  "check_warning": { fr: "avertissement", en: "warning", zh: "警告" },
  "check_ready": { fr: "prête", en: "ready", zh: "就绪" },
  "check_unreachable": { fr: "injoignable", en: "unreachable", zh: "无法连接" },
  "check_reachable": { fr: "joignable", en: "reachable", zh: "可连接" },
  "check_not_configured": { fr: "non configuré", en: "not configured", zh: "未配置" },
};

// Webshop strings are merged only when HubRise is built in, so the
// certification bundle carries no webshop wording.
const screenMessages: Messages = {
  ...coreMessages,
  ...checkoutMessages,
  ...reportsMessages,
  ...settingsMessages,
};
const messages: Messages = { ...screenMessages, ...webshopMessages };

/** Whether `key` has a text (used to tell a known error code). */
export function hasMessage(key: string): boolean {
  return key in messages;
}

export function useI18n() {
  const pos = usePosStore();

  /** `vars` fills `{name}` placeholders in the resolved string, e.g.
   * `t('delivery_outcome_sent', { hint: 'j***@e***.be' })`. */
  function t(key: string, vars?: Record<string, string | number>): string {
    const entry = messages[key];
    let value = entry ? (entry[pos.language] ?? entry.fr ?? key) : key;
    if (vars) {
      for (const [name, v] of Object.entries(vars)) {
        value = value.replaceAll(`{${name}}`, String(v));
      }
    }
    return value;
  }

  return { t };
}
