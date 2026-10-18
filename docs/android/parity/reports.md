# Parity: reports

Port of `ui/src/views/ReportsView.vue` (cards P5b-T3 for the state and P5b-T9 for the screen). The screen is a native
Compose view of module `:app`, package `dev.nuagemagique.pilipos.feature.reports`, laid out for 1920 x 1080 dp. The
engine owns every rule (what a report contains, who may copy or refund, the fiscal signing, the cash rounding); the
screen only calls it through the view model and the shared stores.

## Rows

| Vue | Compose | Commands |
| --- | --- | --- |
| ui/src/views/ReportsView.vue (day tab: X, user X, Z, the reminder, the Z history) | feature/reports/ReportsScreen.kt (ReportsRoute, ReportsContent), ReportsDayTab.kt, ReportsDialogs.kt (ZListDialog), ReportsViewModel.kt, ReportsEntryPoint.kt | `generate_x_report` `generate_user_x_report` `generate_z_report` `list_z_reports` `needs_z_report` |
| ReportsView.vue (cash tab: money in and out, drawer count, drawer opening) | feature/reports/ReportsCashTab.kt, ReportsDialogs.kt (CashDialogs over ui/components/AmountDialog.kt) | `money_in_out` `open_drawer_fiscal` |
| ReportsView.vue (tickets tab and the copy flow, event C) | feature/reports/ReportsTicketsTab.kt, ReportsCopyDialog.kt (over ui/components/TicketMediumPicker.kt and DeliveryOutcomePanel.kt), ReportsCallbacks.kt over ui/state/DeliveryStore.kt | `list_copyable_tickets` `copy_ticket` |
| ReportsView.vue (refund flow: lines, quantity, delivery, result) | feature/reports/ReportsRefundDialog.kt, ReportsRefundLines.kt, RefundPreview.kt, ReportsViewModel.kt | `list_refundable_events` `list_order_lines` `refund_checkout` |
| ReportsView.vue (tickets tab: payment correction, event F, manager PIN) and components/PaymentCorrectionDialog.vue | feature/reports/ReportsTicketsTab.kt, ReportsPaymentCorrectionDialog.kt, ReportsViewModel.kt, ReportsCallbacks.kt (PIN through ui/components/PinDialog.kt) | `list_sale_payments` `correct_payment` |
| stores/delivery.ts (reset when a flow opens and closes, the mail probe, the print action of the result) | feature/reports/ReportsCallbacks.kt over ui/state/DeliveryStore.kt | - |
| ReportsView.vue (tickets tab: sale ticks, Facture action) and stores/invoice.ts (sales, selection, dialog state) | feature/reports/ReportsTicketsTab.kt, InvoiceState.kt, InvoiceViewModel.kt, ReportsCallbacks.kt | `list_invoiceable_sales` |
| components/InvoiceDialog.vue and TextKeyboard.vue (customer step with the offline VAT check, known customers, new customer, Peppol chip; preview step; signing; result) | feature/reports/ReportsInvoiceDialog.kt, InvoiceTextField.kt (system keyboard), InvoiceViewModel.kt | `lookup_b2b_customer` `search_b2b_customers` `save_b2b_customer` `invoice_preview` `sign_invoice_fiscal` |
| components/InvoiceList.vue (invoices, Peppol status, Copie) | feature/reports/ReportsInvoiceList.kt, InvoiceViewModel.kt | `list_invoices` `copy_ticket` |
| components/PriceDialog.vue (other amount, drawer count with zero allowed) | ui/components/AmountDialog.kt, drawn in the overlay slot of the scaffold | - |
| components/SalesReportPanel.vue and utils/salesRange.ts (Ventes tab: presets, custom range with the number pad, per-day table with Z number and concordance mark, by product, by hour bars, rounding and refunds apart, print) | feature/reports/ReportsSalesTab.kt, SalesRange.kt, SalesState.kt, SalesViewModel.kt, ReportsCallbacks.kt | `sales_report` `print_sales_report` |

## Differences

- **Invoices (V11-T2).** The invoice dialog, the invoice list and the Facture action follow `InvoiceDialog.vue`,
  `InvoiceList.vue` and `stores/invoice.ts`. Text fields use the system keyboard instead of `TextKeyboard.vue`
  (the D3 Pro has one); the view model upper-cases what is typed and cuts it to the lengths of the Vue keyboard
  (VAT number 16, postal code 10, country 2, other fields 80). The VAT "live check" is the Vérifier button calling
  `lookup_b2b_customer` (the engine checks the digits offline, no copy of the rule in Kotlin); Continuer runs the
  lookup when none was done. The invoice list is drawn only when at least one invoice exists, and the Copie button
  only for a manager (the engine refuses `copy_ticket` on an invoice to anyone else). No money is computed in
  Kotlin: the totals are the engine's, the sum of the ticked sales is `CashDisplay.sumCents`.
- **No redirect on start.** Vue sends the operator to the login screen when there is no user. The Compose screen
  never navigates when it starts (PD205): without an operator it shows an empty state (`reports_empty`) and reads
  nothing. The root carries the tag `placeholder_reports` (PD204: the tag means "feature root"). A route test with
  the engine closed proves it.
- **Dialogs in the window.** Vue teleports the Z history, the copy flow, the refund flow and the two amount keypads
  to `body`. Compose draws them in the `overlay` slot of `PosScaffold`, inside the window, so the kiosk's immersive
  mode is never lost (PD214). The open state of the two keypads (`otherAmountOpen`, `declarationOpen`) lives in the
  view model so that the dialog and the cash tab agree.
- **Z confirmation.** As in Vue, the first tap on the Z button arms it (its label becomes "Confirmer la clôture ?"),
  the second one within five seconds generates it, and leaving the day tab disarms it. While a report runs, the X,
  user X and Z buttons are disabled. The answer of the last report is kept in a panel under the cards (Vue only
  shows a toast); a refusal shows the text of the engine's error there.
- **Reprint of a Z is a copy.** The history dialog closes and the copy flow opens on the picker, before anything is
  signed, exactly as Vue.
- **Refund preview (UI-PD307).** Vue shows `round5(sum of unit price times quantity)` on the refund buttons. The
  Kotlin screen computes no money. Each ticked line's amount is the single `vatSplit` row of
  `VatLine(vatCode, unitPriceCents, ticked quantity)` (as `CartAmounts.lineAmount`), the VAT summary is `vatSplit`
  of the whole selection, and the preview total is `CashDisplay.sumCents` of those line totals, then
  `EngineHelpers.cashDueCents(total, [PaymentPart(CASH, null)])` for the cash rounding. The cash amount is what the
  refund buttons carry ("Rembourser 9,95 €"). When a ticked line has VAT code X, which `vatSplit` leaves out, no
  total is shown and the screen says that the engine gives the amount when it signs; the refund itself is never
  blocked. The code is in `RefundPreview.kt`; tests prove the sources of the amounts and the X case.
- **Delivery store.** Opening a copy or the delivery step of a refund resets the choice to paper and reads the mail
  status; closing a flow resets it again, as `delivery.reset()` and `delivery.refreshMailStatus()` in Vue. A paper
  choice is sent as no request, as in Vue. The one print action of the result panel is offered to an operator who
  may take payments.
- **Line prices.** The price of each refund line is the engine amount of the full line; a line with VAT code X shows
  `quantity x unit price` instead of a total (PD208).
- **Tickets list.** The list is a lazy column of rows (number, kind, table, time, amount, copy and refund buttons),
  newest first; the amount of a refund ticket is red. Times are Europe/Brussels through `PosDateTime`.
- **Texts.** Every caption is a Vue key through `vueText`; the few texts Vue has no key for are in
  `strings_ui_reports.xml` (`ui_reports_*`, three locales).
- **Refund from the list.** The view model can open the refund on the list of refundable sales
  (`RefundStep.CHOOSE_EVENT`); the screen draws that step although the tickets tab only starts a refund from a row,
  as Vue does.
