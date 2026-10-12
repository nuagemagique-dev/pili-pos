// Checkout screen strings added with the Pili redesign. Merged into the
// dictionary by i18n.ts. Keys: snake_case; every entry has fr, en, zh.
import type { Messages } from "./i18n";

export const checkoutMessages: Messages = {
  // Payment method buttons and part labels (buttons are uppercased in CSS).
  "method_cash": { fr: "Espèces", en: "Cash", zh: "现金" },
  "method_card": { fr: "Carte", en: "Card", zh: "刷卡" },
  "method_meal_voucher": { fr: "Chèques-repas", en: "Meal vouchers", zh: "餐券" },

  // Equal split (§7.4): staff only pick the number of parts.
  "split_into": { fr: "Partager en", en: "Split into", zh: "分成" },
  "equal_parts": { fr: "parts égales", en: "equal parts", zh: "等份" },
  "part_n": { fr: "Part {n}", en: "Part {n}", zh: "第{n}份" },
  "cash_part_taken": {
    fr: "Une part est déjà payée en espèces : les autres se paient par carte ou chèques-repas.",
    en: "One part is already paid in cash: the others are paid by card or meal vouchers.",
    zh: "已有一份以现金支付：其余请用刷卡或餐券支付。",
  },

  // Amount entry.
  "amount_received": { fr: "Montant reçu", en: "Amount received", zh: "收到金额" },
  "exact_rounded": { fr: "exact {exact} · arrondi 5 cts", en: "exact {exact} · rounded to 5 cts", zh: "精确 {exact} · 5分进位" },

  // Order summary (right pane).
  "items_count": { fr: "{n} art.", en: "{n} items", zh: "{n} 件" },
  "ticket_medium": { fr: "Ticket", en: "Receipt", zh: "小票" },

  // Main action.
  "validate_part": { fr: "Valider part {n}", en: "Confirm part {n}", zh: "确认第{n}份" },
  "received_amount": { fr: "reçu {amount} €", en: "received {amount} €", zh: "收到 {amount} €" },

  // Linked card payment terminal (EFT): the waiting overlay, locked card
  // parts, manual entry, and the settings line. The refusal texts
  // (err_eft_*) live in i18n.ts with the other backend errors.
  // The blocking overlay while the terminal works.
  "eft_overlay_title": { fr: "Paiement sur le terminal", en: "Payment on the terminal", zh: "终端支付" },
  "eft_present_card": {
    fr: "Le client présente sa carte sur le terminal",
    en: "The customer presents the card on the terminal",
    zh: "请顾客在终端上刷卡",
  },
  "eft_cancel": { fr: "Annuler", en: "Cancel", zh: "取消" },
  "eft_cancel_requested": { fr: "Annulation demandée...", en: "Cancellation requested...", zh: "已请求取消..." },
  "eft_checking": { fr: "Vérification auprès du terminal...", en: "Asking the terminal...", zh: "正在向终端查询..." },
  "eft_declined": { fr: "Paiement refusé", en: "Payment declined", zh: "付款被拒绝" },
  "eft_cancelled": { fr: "Paiement annulé", en: "Payment cancelled", zh: "付款已取消" },
  "eft_not_charged": { fr: "Rien n'a été débité.", en: "Nothing was charged.", zh: "未扣款。" },
  "eft_refused": { fr: "Paiement impossible", en: "Payment not possible", zh: "无法付款" },
  "eft_unknown_title": { fr: "Réponse du terminal inconnue", en: "Terminal answer unknown", zh: "终端结果未知" },
  "eft_unknown_explain": {
    fr: "La carte a peut-être été débitée. La vente attend la réponse du terminal : vérifiez le paiement pour continuer.",
    en: "The card may have been charged. The sale waits for the terminal's answer: check the payment to continue.",
    zh: "银行卡可能已被扣款。销售需等待终端的结果：请先查询付款状态。",
  },
  "eft_check": { fr: "Vérifier", en: "Check", zh: "查询" },
  "eft_close": { fr: "Fermer", en: "Close", zh: "关闭" },
  "eft_ok": { fr: "OK", en: "OK", zh: "好" },

  // Card parts of the checkout.
  "eft_approved": { fr: "Accepté", en: "Approved", zh: "已批准" },
  "eft_unknown_row": { fr: "Réponse inconnue", en: "Answer unknown", zh: "结果未知" },
  "eft_blocking": {
    fr: "Un paiement par carte attend la réponse du terminal : vérifiez-le pour continuer.",
    en: "A card payment is waiting for the terminal's answer: check it to continue.",
    zh: "有一笔刷卡付款在等待终端结果：请先查询后再继续。",
  },
  "eft_mode_terminal": { fr: "Terminal", en: "Terminal", zh: "终端" },
  "eft_mode_manual": { fr: "Saisie manuelle", en: "Manual entry", zh: "手动录入" },
  "eft_hint_terminal": {
    fr: "Le montant est envoyé au terminal à la validation",
    en: "The amount is sent to the terminal on confirmation",
    zh: "确认后金额将发送到终端",
  },

  // Settings: the linked terminal.
  "settings_payment_terminal_label": { fr: "Terminal de paiement", en: "Payment terminal", zh: "支付终端" },
  "settings_payment_terminal_none": { fr: "Aucun (saisie manuelle)", en: "None (manual entry)", zh: "无（手动录入）" },
  "eft_kind_worldline_terminal_api": { fr: "Worldline", en: "Worldline", zh: "Worldline" },
  "eft_kind_ccv_payment_api": { fr: "CCV cloud", en: "CCV cloud", zh: "CCV 云端" },
  "eft_kind_ccv_opi": { fr: "CCV réseau local", en: "CCV local network", zh: "CCV 局域网" },
};
