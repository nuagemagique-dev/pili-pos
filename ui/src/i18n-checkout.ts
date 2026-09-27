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
};
