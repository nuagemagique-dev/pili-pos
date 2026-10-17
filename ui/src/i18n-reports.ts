// Reports screen strings added with the Pili redesign. Merged into the
// dictionary by i18n.ts. Keys: snake_case; every entry has fr, en, zh.
import type { Messages } from "./i18n";

export const reportsMessages: Messages = {
  // Tabs
  "reports_tab_day": { fr: "Journée", en: "Day", zh: "当日" },
  "reports_tab_cash": { fr: "Caisse", en: "Cash drawer", zh: "钱箱" },
  "reports_tab_tickets": { fr: "Tickets", en: "Tickets", zh: "票据" },

  // Day tab
  "reports_z_pending": { fr: "Rapport Z pas encore fait aujourd'hui", en: "Z report not done yet today", zh: "今天还没有日结" },
  "reports_reprint_z": { fr: "Réimprimer un Z", en: "Reprint a Z", zh: "重印Z报表" },
  "reports_last_z": { fr: "Dernier : Z #{no} · {date}", en: "Last: Z #{no} · {date}", zh: "最近：Z #{no} · {date}" },
  "reports_z_explain": {
    fr: "La clôture remet les compteurs à zéro. Le premier appui arme le bouton, le second dans les 5 secondes confirme.",
    en: "Closing resets the counters. The first tap arms the button, a second tap within 5 seconds confirms.",
    zh: "日结会将计数器清零。第一次点击启用按钮，5秒内再次点击确认。",
  },
  "reports_z_unsettled": {
    fr: "Paiements par carte encaissés mais pas encore enregistrés dans une vente :",
    en: "Card payments taken but not yet recorded in a sale:",
    zh: "已刷卡收款但尚未登记到销售中的付款：",
  },
  "reports_z_unsettled_item": { fr: "{where} · {amount} €{brand}", en: "{where} · {amount} €{brand}", zh: "{where} · {amount} €{brand}" },
  "reports_z_unsettled_note": {
    fr: "La clôture reste possible : chaque paiement sera enregistré avec la vente de sa commande.",
    en: "Closing is still possible: each payment will be recorded with the sale of its order.",
    zh: "仍可日结：每笔付款将随其订单的销售一并登记。",
  },
  "card_charged_badge": { fr: "Carte encaissée", en: "Card charged", zh: "已刷卡" },
  "reports_z_close": { fr: "Clôturer la journée (Z)", en: "Close the day (Z)", zh: "日结 (Z)" },

  // Cash tab
  "reports_step_direction": { fr: "1 · Sens", en: "1 · Direction", zh: "1 · 方向" },
  "reports_step_reason": { fr: "2 · Motif", en: "2 · Reason", zh: "2 · 原因" },
  "reports_step_amount": { fr: "3 · Montant", en: "3 · Amount", zh: "3 · 金额" },
  "reports_dir_in": { fr: "Entrée", en: "In", zh: "存入" },
  "reports_dir_out": { fr: "Sortie", en: "Out", zh: "取出" },
  "reports_reason_fond_de_caisse": { fr: "Fond de caisse", en: "Cash float", zh: "备用金" },
  "reports_reason_retrait": { fr: "Retrait", en: "Withdrawal", zh: "提现" },
  "reports_reason_pourboire": { fr: "Pourboire", en: "Tip", zh: "小费" },
  "reports_other_amount": { fr: "Autre montant", en: "Other amount", zh: "其他金额" },
  "reports_summary": { fr: "Récapitulatif", en: "Summary", zh: "汇总" },
  "reports_open_drawer": { fr: "Ouvrir le tiroir", en: "Open the drawer", zh: "打开钱箱" },
  "reports_declare_drawer": { fr: "Déclarer le tiroir", en: "Declare the drawer", zh: "申报钱箱" },
  "reports_save_in": { fr: "Enregistrer l'entrée", en: "Record the cash in", zh: "登记存入" },
  "reports_save_out": { fr: "Enregistrer la sortie", en: "Record the cash out", zh: "登记取出" },

  // Tickets tab
  "reports_tickets_title": { fr: "Tickets récents", en: "Recent tickets", zh: "最近票据" },
  "reports_tickets_hint": {
    fr: "Choisissez un ticket pour en faire une copie ou le rembourser.",
    en: "Choose a ticket to copy or refund it.",
    zh: "选择一张票据进行复制或退款。",
  },
  "reports_copy": { fr: "Copie", en: "Copy", zh: "副本" },

  // Payment correction (event F, manager + PIN)
  "reports_pc_action": { fr: "Corriger le paiement", en: "Correct payment", zh: "更正付款" },
  "reports_pc_title": { fr: "Corriger le paiement", en: "Correct the payment", zh: "更正付款" },
  "reports_pc_pick_line": { fr: "Quelle ligne de paiement faut-il corriger ?", en: "Which payment line is wrong?", zh: "要更正哪一笔付款？" },
  "reports_pc_line": { fr: "{method} · {amount} €", en: "{method} · {amount} €", zh: "{method} · {amount} €" },
  "reports_pc_terminal": { fr: "Terminal", en: "Terminal", zh: "终端" },
  "reports_pc_terminal_note": {
    fr: "Une carte prise sur le terminal ne se corrige pas.",
    en: "A card taken on the terminal cannot be corrected.",
    zh: "通过终端刷的卡不能更正。",
  },
  "reports_pc_pick_method": { fr: "Payé en réalité par", en: "Actually paid by", zh: "实际付款方式" },
  "reports_pc_method_cash": { fr: "Espèces", en: "Cash", zh: "现金" },
  "reports_pc_method_card": { fr: "Carte", en: "Card", zh: "银行卡" },
  "reports_pc_method_meal_voucher": { fr: "Titres-repas", en: "Meal vouchers", zh: "餐券" },
  "reports_pc_summary": { fr: "{from} {amount} € → {to} {amount} €", en: "{from} {amount} € → {to} {amount} €", zh: "{from} {amount} € → {to} {amount} €" },
  "reports_pc_confirm": { fr: "Corriger", en: "Correct", zh: "更正" },
  "reports_pc_pin_hint": {
    fr: "Confirmez la correction du paiement avec votre code PIN.",
    en: "Confirm the payment correction with your PIN.",
    zh: "请输入PIN码确认付款更正。",
  },
  "reports_pc_done": { fr: "Paiement corrigé", en: "Payment corrected", zh: "付款已更正" },
  "reports_pc_corrected": { fr: "Paiement corrigé : {from} → {to}", en: "Payment corrected: {from} → {to}", zh: "付款已更正：{from} → {to}" },
  "reports_pc_not_possible": { fr: "Ce paiement ne peut pas être corrigé", en: "This payment cannot be corrected", zh: "此付款无法更正" },
};
