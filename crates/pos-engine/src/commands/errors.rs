//! Error codes returned to the screen.
//!
//! A command that refuses something returns one of these codes instead
//! of an English sentence; the UI shows it in the operator's language
//! (`ui/src/utils/errorMessages.ts`, keys `err_<code>` in
//! `ui/src/i18n.ts`). A code may carry values, written
//! `code|name=value|name=value` (see [`coded`]).
//!
//! FDM failures keep their own path: they start with "FDM" and are
//! translated from the FDM's codes (`ui/src/utils/fdmMessages.ts`).
//!
//! Every code must be listed in [`ALL`]: `tests/error_codes.rs` checks
//! that each one has a French, English and Chinese text.

use std::fmt::Display;

// Session and roles.
pub const NOT_LOGGED_IN: &str = "not_logged_in";
pub const FORBIDDEN: &str = "forbidden";
pub const NOT_THE_OPERATOR: &str = "not_the_operator";

// Login, users and PINs.
pub const INSZ_INVALID: &str = "insz_invalid";
pub const USER_UNKNOWN: &str = "user_unknown";
pub const PIN_REQUIRED: &str = "pin_required";
pub const PIN_WRONG: &str = "pin_wrong";
/// Too many wrong PINs: `pin_locked|seconds=N`.
pub const PIN_LOCKED: &str = "pin_locked";
pub const PIN_FORMAT: &str = "pin_format";
pub const PIN_SETUP_REQUIRED: &str = "pin_setup_required";
pub const PIN_ALREADY_SET: &str = "pin_already_set";
/// A manager tile without a PIN once a manager has one.
pub const FIRST_PIN_NEEDS_MANAGER: &str = "first_pin_needs_manager";
pub const SYSTEM_USER: &str = "system_user";
pub const NAME_REQUIRED: &str = "name_required";
pub const ROLE_INVALID: &str = "role_invalid";
pub const DEACTIVATE_SELF: &str = "deactivate_self";

// Orders.
pub const OPERATOR_INVALID: &str = "operator_invalid";
pub const DINE_IN_NEEDS_TABLE: &str = "dine_in_needs_table";
pub const TAKEAWAY_HAS_NO_TABLE: &str = "takeaway_has_no_table";
/// The table plan is not a list of table numbers and ranges ("1-12, 20-25").
pub const TABLE_PLAN_INVALID: &str = "table_plan_invalid";
/// A takeaway is a direct sale: it ends in a sale, never in an order
/// registration (FAQ Q205).
pub const TAKEAWAY_NOT_REGISTERED: &str = "takeaway_not_registered";
pub const QUANTITY_POSITIVE: &str = "quantity_positive";
pub const ORDER_NOT_OPEN: &str = "order_not_open";
pub const PRICE_POSITIVE: &str = "price_positive";
/// `product`: the menu item id.
pub const FIXED_PRICE: &str = "fixed_price";
/// `product`: the menu item id.
pub const PRODUCT_UNAVAILABLE: &str = "product_unavailable";
pub const LINE_NOT_FOUND: &str = "line_not_found";
pub const ORDER_NO_LINES: &str = "order_no_lines";
pub const ORDER_TOTAL_NEGATIVE: &str = "order_total_negative";
pub const AMOUNT_NEGATIVE: &str = "amount_negative";
/// The business day before is still open: its automatic Z comes first.
pub const PREVIOUS_DAY_OPEN: &str = "previous_day_open";
pub const ORDER_ALREADY_CLOSED: &str = "order_already_closed";

// Tickets: copies and refunds.
pub const EVENT_NOT_FOUND: &str = "event_not_found";
/// `field`: what the archived original lacks.
pub const ORIGINAL_INCOMPLETE: &str = "original_incomplete";
/// `label`: the archived label.
pub const EVENT_LABEL_UNKNOWN: &str = "event_label_unknown";
pub const NOTHING_TO_REFUND: &str = "nothing_to_refund";
/// `label`: the event type of the original.
pub const NOT_REFUNDABLE: &str = "not_refundable";
/// `product`: the menu item id.
pub const REFUND_QUANTITY_POSITIVE: &str = "refund_quantity_positive";
/// `product`: the menu item id.
pub const REFUND_PRODUCT_NOT_SOLD: &str = "refund_product_not_sold";
/// `product`, `requested`, `sold`, `refunded`.
pub const REFUND_EXCEEDS_SOLD: &str = "refund_exceeds_sold";
/// `product`: sold at more than one price; the line must be named.
pub const REFUND_LINE_AMBIGUOUS: &str = "refund_line_ambiguous";
/// Only a sale or a copy has an original ticket to print.
pub const PRINT_ORIGINAL_UNSUPPORTED: &str = "print_original_unsupported";
pub const REFUND_IN_TRAINING: &str = "refund_in_training";
pub const REFUND_OF_REFUND: &str = "refund_of_refund";
/// A sale closed unpaid: no money came in, none goes back.
pub const REFUND_UNPAID_SALE: &str = "refund_unpaid_sale";

// Payment.
/// A part without an amount, a zero or negative amount, a cash part
/// with an amount, or two cash parts.
pub const PAYMENT_INVALID: &str = "payment_invalid";
/// The card and meal voucher parts come to more than the total.
pub const PAYMENT_EXCEEDS_TOTAL: &str = "payment_exceeds_total";
/// Something is left to pay and no cash part pays it.
pub const PAYMENT_INCOMPLETE: &str = "payment_incomplete";
/// Only a sale with something to pay can be closed unpaid; a sale of
/// 0,00 EUR is paid normally (a cash payment of 0, FAQ Q206).
pub const UNPAID_NOTHING_DUE: &str = "unpaid_nothing_due";

// Payment terminal (EFT). Nothing was charged unless a code says otherwise.
/// `pos.json` has no `payment_terminal`: cards are typed by hand.
pub const EFT_NOT_CONFIGURED: &str = "eft_not_configured";
/// Training mode: a training sale never charges a real card; cards are
/// typed by hand.
pub const EFT_TRAINING: &str = "eft_training";
/// Zero or less, or more than what is still due on the order after its
/// other approved terminal payments.
pub const EFT_AMOUNT_INVALID: &str = "eft_amount_invalid";
/// A terminal payment of the till waits for its answer (sent or
/// unknown): check it first, nothing else is charged or signed meanwhile.
pub const EFT_PENDING: &str = "eft_pending";
/// The terminal is busy with another payment.
pub const EFT_BUSY: &str = "eft_busy";
/// The terminal or its provider cannot be reached (`detail`).
pub const EFT_UNREACHABLE: &str = "eft_unreachable";
/// The terminal or its provider refused the request itself, or the
/// `payment_terminal` section of `pos.json` cannot drive it (`detail`).
pub const EFT_REJECTED: &str = "eft_rejected";
/// The payment named by a card part is not an approved, unused payment
/// of this order for that amount, or an approved payment of the order
/// was left out of the parts.
pub const EFT_PAYMENT_NOT_USABLE: &str = "eft_payment_not_usable";
/// The customer's card was charged `{amount}` (euros) on this order:
/// an edit may not bring the total under it, since the sale must settle
/// the payment to the cent. Nothing was changed.
pub const EFT_BELOW_CHARGED: &str = "eft_below_charged";
/// A terminal payment of the order waits for its answer (sent or
/// unknown): its amount is not final, so the order cannot be edited
/// until it is checked. Nothing was changed.
pub const EFT_EDIT_PENDING: &str = "eft_edit_pending";

// Settings.
pub const OPEN_ORDERS_TRAINING: &str = "open_orders_training";

// Ticket delivery and printing.
pub const DIGITAL_UNAVAILABLE: &str = "digital_unavailable";
pub const EMAIL_INVALID: &str = "email_invalid";
pub const ALREADY_PRINTED: &str = "already_printed";
pub const EMAIL_SENT: &str = "email_sent";
pub const PRINTER_NOT_CONFIGURED: &str = "printer_not_configured";
/// `detail`: the printer's error, for the technician.
pub const PRINT_FAILED: &str = "print_failed";

// Remote support.
pub const SUPPORT_UNAVAILABLE: &str = "support_unavailable";
pub const SUPPORT_RUNNING: &str = "support_running";
/// `detail`: why the program did not start.
pub const SUPPORT_START_FAILED: &str = "support_start_failed";

// Signing.
/// Another event was being signed for longer than the wait allows.
pub const SIGNING_BUSY: &str = "signing_busy";
/// The order's event waits for the FDM's answer: it cannot change.
pub const ORDER_SIGN_PENDING: &str = "order_sign_pending";
/// The event names a booking period its Z closed.
pub const PERIOD_CLOSED: &str = "period_closed";
/// A sign request waits for the FDM's answer (`label`, `ticket`): no
/// other event is signed until it comes. `fdm`, when the FDM did answer
/// but not with a signature (INTERNAL_ERROR, UNDEFINED_ERROR,
/// UNDEFINED_OTHER, or `FDM_RESPONSE_MISMATCH` for an answer that does
/// not echo the request): the screens show that code's message first,
/// then say the same request is sent again. Starts with "FDM", so the
/// screen shows it as an FDM message.
pub const FDM_ANSWER_PENDING: &str = "FDM_ANSWER_PENDING";

// Integrity.
/// The ticket counter moved between number and archive: nothing was
/// recorded, the operator retries. Signing holds one lock, so this
/// only reports a broken invariant.
pub const CONCURRENT_OPERATION: &str = "concurrent_operation";
/// `detail`: what went wrong, for the technician. Also logged.
pub const INTERNAL_ERROR: &str = "internal_error";

// Configuration problems (`pos.json`), shown in the screen's language
// with the English text of the problem as `detail` for the installer.
// One code per setting, see `pos_core::config::problem_code`.
pub const CONFIG_TABLES: &str = "config_tables";
pub const CONFIG_POS_ID: &str = "config_pos_id";
pub const CONFIG_RESTAURANT_VAT: &str = "config_restaurant_vat";
pub const CONFIG_RESTAURANT_EST_NO: &str = "config_restaurant_est_no";
pub const CONFIG_IDENTITY: &str = "config_identity";
pub const CONFIG_TICKET_HEADER: &str = "config_ticket_header";
pub const CONFIG_FDM_URL: &str = "config_fdm_url";
pub const CONFIG_FDM_FILES: &str = "config_fdm_files";
pub const CONFIG_LANGUAGE: &str = "config_language";
pub const CONFIG_DAY_END_HOUR: &str = "config_day_end_hour";
pub const CONFIG_MISSING: &str = "config_missing";
pub const CONFIG_HUBRISE: &str = "config_hubrise";
pub const CONFIG_DIGITAL_TICKET: &str = "config_digital_ticket";
pub const CONFIG_EXPORT: &str = "config_export";
pub const CONFIG_PAYMENT_TERMINAL: &str = "config_payment_terminal";
pub const CONFIG_ALERTS: &str = "config_alerts";
pub const CONFIG_OTHER: &str = "config_other";

// Owner alerts (v1.2).
/// A setting of the owner alerts is out of range: `alerts_invalid|field=<name>`.
pub const ALERTS_INVALID: &str = "alerts_invalid";
/// No bot token: `pos.json` has no `alerts.telegram_bot_token_file`, or
/// the file is missing or empty.
pub const ALERTS_TOKEN_MISSING: &str = "alerts_token_missing";
/// The test message did not arrive (`detail`: the HTTP status or the
/// kind of transport error, never an address).
pub const ALERTS_SEND_FAILED: &str = "alerts_send_failed";

// Allergens (v1.2).
/// A code that is not one of the 14 allergens of Annex II (or a sheet
/// language that is not offered): `allergen_unknown|code=<code>`.
pub const ALLERGEN_UNKNOWN: &str = "allergen_unknown";
/// The product or option to edit, or the product to print, does not
/// exist: `allergen_target_not_found|id=<id>`.
pub const ALLERGEN_TARGET_NOT_FOUND: &str = "allergen_target_not_found";

// Courses (v1.2).
/// The line already went out on a kitchen slip: its course can no longer change.
pub const COURSE_ALREADY_SENT: &str = "course_already_sent";
/// A course outside 0 to 3, or a category that does not exist or is listed
/// twice: `course_invalid|course=<n>` or `course_invalid|category=<id>`.
pub const COURSE_INVALID: &str = "course_invalid";
/// Courses are switched off (Settings, "Service en plusieurs temps").
pub const COURSES_DISABLED: &str = "courses_disabled";
/// "Envoyer la suite" with no course waiting and nothing left to print.
pub const NO_COURSE_HELD: &str = "no_course_held";
/// A reprint of a course no slip ever printed.
pub const NOTHING_PRINTED: &str = "nothing_printed";

/// A problem of `pos.json` (an English text of
/// `PosConfig::problems`) as a coded error: its `config_<setting>` code
/// and the text as `detail`.
pub fn config_problem(problem: &str) -> String {
    coded(
        &format!("config_{}", pos_core::config::problem_code(problem)),
        &[("detail", &problem)],
    )
}

/// Every code, for the translation check.
pub const ALL: &[&str] = &[
    CONFIG_TABLES,
    CONFIG_POS_ID,
    CONFIG_RESTAURANT_VAT,
    CONFIG_RESTAURANT_EST_NO,
    CONFIG_IDENTITY,
    CONFIG_TICKET_HEADER,
    CONFIG_FDM_URL,
    CONFIG_FDM_FILES,
    CONFIG_LANGUAGE,
    CONFIG_DAY_END_HOUR,
    CONFIG_MISSING,
    CONFIG_HUBRISE,
    CONFIG_DIGITAL_TICKET,
    CONFIG_EXPORT,
    CONFIG_PAYMENT_TERMINAL,
    CONFIG_ALERTS,
    CONFIG_OTHER,
    ALERTS_INVALID,
    ALERTS_TOKEN_MISSING,
    ALERTS_SEND_FAILED,
    ALLERGEN_UNKNOWN,
    ALLERGEN_TARGET_NOT_FOUND,
    COURSE_ALREADY_SENT,
    COURSE_INVALID,
    COURSES_DISABLED,
    NO_COURSE_HELD,
    NOTHING_PRINTED,
    NOT_LOGGED_IN,
    FORBIDDEN,
    NOT_THE_OPERATOR,
    INSZ_INVALID,
    USER_UNKNOWN,
    PIN_REQUIRED,
    PIN_WRONG,
    PIN_LOCKED,
    PIN_FORMAT,
    PIN_SETUP_REQUIRED,
    PIN_ALREADY_SET,
    FIRST_PIN_NEEDS_MANAGER,
    SYSTEM_USER,
    NAME_REQUIRED,
    ROLE_INVALID,
    DEACTIVATE_SELF,
    OPERATOR_INVALID,
    DINE_IN_NEEDS_TABLE,
    TAKEAWAY_HAS_NO_TABLE,
    TABLE_PLAN_INVALID,
    TAKEAWAY_NOT_REGISTERED,
    QUANTITY_POSITIVE,
    ORDER_NOT_OPEN,
    PRICE_POSITIVE,
    FIXED_PRICE,
    PRODUCT_UNAVAILABLE,
    LINE_NOT_FOUND,
    ORDER_NO_LINES,
    ORDER_TOTAL_NEGATIVE,
    AMOUNT_NEGATIVE,
    PREVIOUS_DAY_OPEN,
    ORDER_ALREADY_CLOSED,
    EVENT_NOT_FOUND,
    ORIGINAL_INCOMPLETE,
    EVENT_LABEL_UNKNOWN,
    NOTHING_TO_REFUND,
    NOT_REFUNDABLE,
    REFUND_QUANTITY_POSITIVE,
    REFUND_PRODUCT_NOT_SOLD,
    REFUND_EXCEEDS_SOLD,
    REFUND_LINE_AMBIGUOUS,
    PRINT_ORIGINAL_UNSUPPORTED,
    REFUND_IN_TRAINING,
    REFUND_OF_REFUND,
    REFUND_UNPAID_SALE,
    PAYMENT_INVALID,
    PAYMENT_EXCEEDS_TOTAL,
    PAYMENT_INCOMPLETE,
    UNPAID_NOTHING_DUE,
    EFT_NOT_CONFIGURED,
    EFT_TRAINING,
    EFT_AMOUNT_INVALID,
    EFT_PENDING,
    EFT_BUSY,
    EFT_UNREACHABLE,
    EFT_REJECTED,
    EFT_PAYMENT_NOT_USABLE,
    EFT_BELOW_CHARGED,
    EFT_EDIT_PENDING,
    OPEN_ORDERS_TRAINING,
    DIGITAL_UNAVAILABLE,
    EMAIL_INVALID,
    ALREADY_PRINTED,
    EMAIL_SENT,
    PRINTER_NOT_CONFIGURED,
    PRINT_FAILED,
    SUPPORT_UNAVAILABLE,
    SUPPORT_RUNNING,
    SUPPORT_START_FAILED,
    SIGNING_BUSY,
    ORDER_SIGN_PENDING,
    PERIOD_CLOSED,
    CONCURRENT_OPERATION,
    INTERNAL_ERROR,
];

/// Codes of the HubRise integration (feature `hubrise`), translated
/// in `ui/src/i18n-webshop.ts`.
#[cfg(feature = "hubrise")]
pub mod hubrise {
    /// No HubRise connection is set up.
    pub const NOT_CONNECTED: &str = "hubrise_not_connected";
    /// The order is not one HubRise sent to this POS.
    pub const ORDER_UNKNOWN: &str = "hubrise_order_unknown";
    /// That status is set by the fiscal commands (accept, reject,
    /// hand over), or is not a HubRise status.
    pub const STATUS_NOT_ALLOWED: &str = "hubrise_status_not_allowed";
    /// Not an RFC 3339 time.
    pub const TIME_INVALID: &str = "hubrise_time_invalid";
    /// The menu mirrors an external catalog: it is edited there.
    pub const MENU_MANAGED: &str = "hubrise_menu_managed";
    /// Taking over the menu waits until every POS order is closed.
    pub const OPEN_ORDERS: &str = "hubrise_open_orders";
    /// No catalog is waiting for the manager's adoption.
    pub const NOTHING_PENDING: &str = "hubrise_nothing_pending";
    /// A channel setting outside platform/webshop, APP/ONLINE,
    /// include/exclude, or without a name.
    pub const CHANNEL_INVALID: &str = "hubrise_channel_invalid";
    /// Online orders are real sales: not in training mode.
    pub const TRAINING: &str = "webshop_training";
    /// Accepted, rejected or being signed already.
    pub const ALREADY_HANDLED: &str = "webshop_already_handled";
    /// The order does not match the menu, or its payment is unclear.
    pub const NEEDS_REVIEW: &str = "webshop_needs_review";
    /// Only an accepted unpaid order is collected.
    pub const NOT_AWAITING_PAYMENT: &str = "webshop_not_awaiting_payment";
    /// App and online payments are not taken at the till.
    pub const PAYMENT_NOT_AT_TILL: &str = "webshop_payment_not_at_till";
    /// Only a signed order HubRise cancelled is refunded or closed.
    pub const NOT_CANCELLED: &str = "webshop_not_cancelled";
    /// An option that is not one of the product's (or is inactive).
    pub const MODIFIER_UNKNOWN: &str = "modifier_unknown";
    /// Too few or too many options in a group ({group}, {min}, {max}).
    pub const MODIFIER_COUNT: &str = "modifier_count";
    /// A free-price product takes no options.
    pub const MODIFIER_OPEN_PRICE: &str = "modifier_open_price";
    /// A line sold with a discount is refunded whole, never by product.
    pub const REFUND_DISCOUNTED_LINE: &str = "refund_discounted_line";
    /// The integration is switched off on this installation.
    pub const DISABLED: &str = "hubrise_disabled";
    /// `pos.json` has no `hubrise` section: this installation has no
    /// HubRise client to connect with.
    pub const NOT_CONFIGURED: &str = "hubrise_not_configured";
    /// The `hubrise.client_secret_file` of `pos.json` is missing or
    /// empty.
    pub const SECRET_UNREADABLE: &str = "hubrise_secret_unreadable";
    /// Switching off waits until every online order is settled
    /// ({count}).
    pub const ONLINE_ORDERS_OPEN: &str = "hubrise_online_orders_open";
    /// HubRise refused the code of the connection, or did not answer
    /// (`detail`: what it said, for the installer).
    pub const OAUTH_FAILED: &str = "hubrise_oauth_failed";

    pub const ALL: &[&str] = &[
        NOT_CONNECTED,
        ORDER_UNKNOWN,
        STATUS_NOT_ALLOWED,
        TIME_INVALID,
        MENU_MANAGED,
        OPEN_ORDERS,
        NOTHING_PENDING,
        CHANNEL_INVALID,
        TRAINING,
        ALREADY_HANDLED,
        NEEDS_REVIEW,
        NOT_AWAITING_PAYMENT,
        PAYMENT_NOT_AT_TILL,
        NOT_CANCELLED,
        MODIFIER_UNKNOWN,
        MODIFIER_COUNT,
        MODIFIER_OPEN_PRICE,
        REFUND_DISCOUNTED_LINE,
        DISABLED,
        ONLINE_ORDERS_OPEN,
        NOT_CONFIGURED,
        SECRET_UNREADABLE,
        OAUTH_FAILED,
    ];
}

/// A code with values: `code|name=value|...`. A `|` inside a value
/// becomes `/`, so the UI can always split the string back.
pub fn coded(code: &str, values: &[(&str, &dyn Display)]) -> String {
    let mut out = code.to_string();
    for (name, value) in values {
        out.push('|');
        out.push_str(name);
        out.push('=');
        out.push_str(&value.to_string().replace('|', "/"));
    }
    out
}

/// An unexpected failure (database, file, lock): logged in full for
/// the technician, shown as "internal error" with its detail.
pub fn internal(detail: impl Display) -> String {
    tracing::error!(%detail, "internal error returned to the screen");
    coded(INTERNAL_ERROR, &[("detail", &detail)])
}

/// The archived original of a copy or refund lacks `field`.
pub fn incomplete(field: &str) -> String {
    coded(ORIGINAL_INCOMPLETE, &[("field", &field)])
}

/// The ticket counter moved between peek and commit. The transaction
/// rolled back; nothing was recorded.
pub fn concurrent(what: impl Display) -> String {
    tracing::error!(%what, "counter drifted between peek and commit");
    CONCURRENT_OPERATION.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_code_without_values_is_the_code() {
        assert_eq!(coded(ORDER_NO_LINES, &[]), "order_no_lines");
    }

    #[test]
    fn values_follow_the_code() {
        assert_eq!(
            coded(REFUND_EXCEEDS_SOLD, &[("product", &12), ("sold", &2)]),
            "refund_exceeds_sold|product=12|sold=2"
        );
    }

    #[test]
    fn a_separator_inside_a_value_cannot_split_it() {
        assert_eq!(
            coded(PRINT_FAILED, &[("detail", &"a|b")]),
            "print_failed|detail=a/b"
        );
    }

    #[test]
    fn every_code_is_listed_once_and_well_formed() {
        let mut seen = std::collections::HashSet::new();
        for code in ALL {
            assert!(seen.insert(code), "{code} listed twice");
            assert!(
                code.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "{code}"
            );
        }
    }

    #[test]
    fn an_internal_error_keeps_its_detail() {
        assert_eq!(internal("disk full"), "internal_error|detail=disk full");
    }
}
