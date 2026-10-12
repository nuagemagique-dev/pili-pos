//! Ownership of every FFI record and enum (PD56), written by P2-T4 from the engine sources.
//!
//! One row per type reachable from the 86 cert commands (parameters and return types, through
//! their fields), from the event payloads and from the money helpers. `invoice` is excluded
//! (peppol). The owner is the card that defines the type: the file of a domain card is
//! `records/<owner>.rs`, the shared ones are in `records/common.rs` (this card).
//!
//! - **common**: reachable from two or more domains of the command map
//!   (`check_ipc_wrappers.py --map`), from an event payload, or from a helper. A domain card
//!   never redefines one: it imports it with `use super::common::...`.
//! - **orders** = order, modifiers, menu, tables, session; **checkout** = checkout, unpaid,
//!   refund, copy, delivery, print; **reports** = report, social, financial, settings,
//!   fdm_monitor; **auth** = auth, integrations, support; **hubrise** = hubrise, webshop.
//! - The FFI name is the engine name, except `Display` (`pos_fiscal::types`), which is
//!   `FdmMessageDisplay`. No two reachable engine types share a name (`Order`, `Category` and
//!   `Language` also exist in `pos_hubrise_types` and `pos_catalog`, but none of those is
//!   reachable from a command); a domain card that ever needs one of them prefixes its domain.
//! - The two JSON commands get typed records built from their JSON shape: `FdmStatusReport` (the
//!   FDM status as the FDM answers it, or `{connected: false, error}` when it does not; every
//!   field of both shapes is optional, absent keys stay `None`) and `FiscalEventSummary` (one
//!   element of the `list_fiscal_events` array, camelCase keys).
//!   Shapes, for the checkout card (camelCase keys, `deny_unknown_fields`, `convert::out` from the
//!   engine's JSON value):
//!   `FdmStatusReport { connected: Option<bool>, error: Option<String>, device: Option<FdmDevice>,
//!   initialized: Option<bool>, informations / warnings / errors: Option<Vec<MessageItem>> }` with
//!   `FdmDevice { fdm_id, fdm_sw_version, fdm_date_time: String, buffer_capacity_used: f64 }`
//!   (`pos_fiscal::response::FdmStatus` when the FDM answers, `{connected: false, error}` when it
//!   does not); `FiscalEventSummary { id: i64, event_type: String, sequence_number: i64,
//!   order_id: Option<i64>, fdm_id / fdm_datetime / event_label / short_signature: Option<String> }`
//!   (keys `id`, `eventType`, `sequenceNumber`, `orderId`, `fdmId`, `fdmDatetime`, `eventLabel`,
//!   `shortSignature`; note `fdmDatetime` with a lowercase t).
//! - Types the helpers return (`settle_preview`, `cash_due_cents`: the checkout card; `vat_split`:
//!   the orders card) are defined by the card that implements the helper, in its own file, and
//!   take their shared members from `common` (`PaymentPart`, `PaymentMethod`, `VatCode`).
//! - "used by": the commands, `event <topic>` or `helper <name>` that reach the type; at most
//!   three commands are named, the rest is counted.
//!
//! Rules for every record in this directory: the engine's serde attributes, derives
//! `Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record` (or
//! `uniffi::Enum`), `#[serde(deny_unknown_fields)]` on records, `u32`/`u64`/`usize` become `i64`,
//! chrono times `String`, `serde_json::Value` a `String` with
//! `#[serde(with = "crate::convert::json_text")]` (`json_text_opt` for an `Option`), `f64` stays
//! `f64`. A type serde cannot mirror carries `// contract: custom` and its own conversion and test.
//!
//! | FFI name | engine path | owner | used by |
//! |---|---|---|---|
//! | `CallbackEvent` | `pos_hubrise_types::callback::CallbackEvent` | common | event webshop-order-*, catalog-* |
//! | `CheckoutResult` | `pos_engine::commands::checkout::CheckoutResult` | common | checkout, close_unpaid, refund_checkout (+4 commands) |
//! | `DataItem` | `pos_fiscal::response::DataItem` | common | event fdm-sign-messages |
//! | `DeliveryOutcome` | `pos_engine::commands::delivery::DeliveryOutcome` | common | checkout, close_unpaid, copy_ticket (+5 commands) |
//! | `EmailStatus` | `pos_engine::commands::delivery::EmailStatus` | common | checkout, close_unpaid, copy_ticket (+5 commands) |
//! | `EventLabel` | `pos_fiscal::types::EventLabel` | common | checkout, close_unpaid, copy_ticket (+5 commands) |
//! | `ExtensionItem` | `pos_fiscal::response::ExtensionItem` | common | event fdm-sign-messages |
//! | `FdmHealth` | `pos_engine::commands::fdm_monitor::FdmHealth` | common | fdm_check_now, fdm_health; event fdm-status |
//! | `FdmMessageDisplay` | `pos_fiscal::types::Display` | common | event fdm-sign-messages |
//! | `FdmReference` | `pos_fiscal::response::FdmReference` | common | checkout, close_unpaid, copy_ticket (+5 commands) |
//! | `FdmSeverity` | `pos_engine::commands::fdm_monitor::FdmSeverity` | common | fdm_check_now, fdm_health; event fdm-status |
//! | `FdmWarningCode` | `pos_fiscal::types::FdmWarningCode` | common | event fdm-sign-messages |
//! | `FiscalTicket` | `pos_fiscal::ticket::FiscalTicket` | common | checkout, close_unpaid, copy_ticket (+5 commands) |
//! | `LocationItem` | `pos_fiscal::response::LocationItem` | common | event fdm-sign-messages |
//! | `LockReason` | `pos_engine::commands::session::LockReason` | common | event session-locked |
//! | `MessageCategory` | `pos_fiscal::types::MessageCategory` | common | event fdm-sign-messages |
//! | `MessageItem` | `pos_fiscal::response::MessageItem` | common | event fdm-sign-messages |
//! | `OriginalControlData` | `pos_fiscal::ticket::OriginalControlData` | common | checkout, close_unpaid, copy_ticket (+5 commands) |
//! | `PaymentMethod` | `pos_fiscal::payment::PaymentMethod` | common | checkout, webshop_handover_order; helper settle_preview, cash_due_cents |
//! | `PaymentPart` | `pos_fiscal::payment::PaymentPart` | common | checkout, webshop_handover_order; helper settle_preview, cash_due_cents |
//! | `PresenceFailed` | `pos_engine::commands::social::PresenceFailed` | common | event presence-failed |
//! | `ReceiptLanguage` | `pos_fiscal::ticket::ReceiptLanguage` | common | checkout, close_unpaid, copy_ticket (+5 commands) |
//! | `RequestedMedium` | `pos_engine::commands::delivery::RequestedMedium` | common | checkout, copy_ticket, refund_checkout (+2 commands) |
//! | `SessionLocked` | `pos_engine::commands::session::SessionLocked` | common | event session-locked |
//! | `SignMessages` | `pos_fiscal::fdm_client::SignMessages` | common | event fdm-sign-messages |
//! | `SlipOutcome` | `pos_engine::commands::bar_slip::SlipOutcome` | common | checkout, close_unpaid, leave_order (+7 commands) |
//! | `TicketDeliveryRequest` | `pos_engine::commands::delivery::TicketDeliveryRequest` | common | checkout, copy_ticket, refund_checkout (+2 commands) |
//! | `TicketLine` | `pos_fiscal::ticket::TicketLine` | common | checkout, close_unpaid, copy_ticket (+5 commands) |
//! | `TicketMedium` | `pos_fiscal::types::TicketMedium` | common | checkout, close_unpaid, copy_ticket (+5 commands) |
//! | `TicketPayment` | `pos_fiscal::ticket::TicketPayment` | common | checkout, close_unpaid, copy_ticket (+5 commands) |
//! | `VatCalcItem` | `pos_fiscal::response::VatCalcItem` | common | checkout, close_unpaid, copy_ticket (+5 commands) |
//! | `VatCode` | `pos_core::vat::VatCode` | common | add_order_line, add_order_line_with_modifiers, get_menu_items (+3 commands); helper vat_split |
//! | `VatLabel` | `pos_fiscal::types::VatLabel` | common | checkout, close_unpaid, copy_ticket (+5 commands) |
//! | `AddOrderLineInput` | `pos_engine::commands::order::AddOrderLineInput` | orders | add_order_line |
//! | `AddWithModifiersInput` | `pos_engine::commands::modifiers::AddWithModifiersInput` | orders | add_order_line_with_modifiers |
//! | `CartLine` | `pos_db::queries::orders::CartLine` | orders | list_cart_lines |
//! | `Category` | `pos_core::menu::Category` | orders | get_categories |
//! | `CreateOrderInput` | `pos_engine::commands::order::CreateOrderInput` | orders | create_order |
//! | `LeaveReason` | `pos_engine::commands::session::LeaveReason` | orders | leave_order |
//! | `LockOutcome` | `pos_engine::commands::session::LockOutcome` | orders | lock_session |
//! | `MenuItem` | `pos_core::menu::MenuItem` | orders | get_menu_items |
//! | `ModifierGroup` | `pos_core::menu::ModifierGroup` | orders | get_item_modifiers |
//! | `Order` | `pos_core::order::Order` | orders | create_order, get_order, list_open_orders |
//! | `OrderLine` | `pos_core::order::OrderLine` | orders | add_order_line, add_order_line_with_modifiers, list_cart_lines (+1 commands) |
//! | `OrderStatus` | `pos_core::order::OrderStatus` | orders | create_order, get_order, list_open_orders |
//! | `OrderType` | `pos_core::order::OrderType` | orders | create_order, get_order, list_open_orders |
//! | `TableInfo` | `pos_engine::commands::tables::TableInfo` | orders | list_tables |
//! | `TablePlan` | `pos_engine::commands::table_plan::TablePlan` | orders | get_table_plan, set_table_plan |
//! | `TablePlanChange` | `pos_db::queries::table_plan::PlanChange` | orders | get_table_plan, set_table_plan |
//! | `TakeawayInfo` | `pos_engine::commands::tables::TakeawayInfo` | orders | list_open_takeaways |
//! | `CopyResult` | `pos_engine::commands::copy::CopyResult` | checkout | copy_ticket |
//! | `CopyableTicket` | `pos_db::queries::fiscal::CopyableTicket` | checkout | list_copyable_tickets |
//! | `DeliveryView` | `pos_engine::commands::delivery::DeliveryView` | checkout | list_ticket_deliveries |
//! | `EftPayment` | `pos_engine::commands::eft::EftPayment` | eft | eft_pay, eft_check, list_eft_payments |
//! | `FdmDevice` | `pos_fiscal::response::FdmDevice` | checkout | check_fdm_status (inside FdmStatusReport) |
//! | `FdmStatusReport` | `json: pos_engine::commands::checkout::do_check_fdm_status` | checkout | check_fdm_status (typed form of its serde_json::Value) |
//! | `FiscalEventSummary` | `json: pos_engine::commands::copy::do_list_fiscal_events` | checkout | list_fiscal_events (typed form of its serde_json::Value) |
//! | `MailStatus` | `pos_engine::commands::delivery::MailStatus` | checkout | check_mail_status |
//! | `RefundItem` | `pos_engine::commands::refund::RefundItem` | checkout | refund_checkout |
//! | `RefundableEvent` | `pos_db::queries::fiscal::RefundableEvent` | checkout | list_refundable_events |
//! | `FinancialResult` | `pos_engine::commands::financial::FinancialResult` | reports | money_in_out, open_drawer_fiscal |
//! | `MoneyDirection` | `pos_engine::commands::financial::MoneyDirection` | reports | money_in_out |
//! | `OpenShift` | `pos_db::queries::shifts::OpenShift` | reports | list_open_shifts |
//! | `PosSettings` | `pos_engine::commands::settings::PosSettings` | reports | get_settings |
//! | `ReportResult` | `pos_engine::commands::report::ReportResult` | reports | generate_user_x_report, generate_x_report, generate_z_report |
//! | `SocialResult` | `pos_engine::commands::social::SocialResult` | reports | end_shift, work_in, work_out |
//! | `ZReportSummary` | `pos_db::queries::reports::ZReportSummary` | reports | list_z_reports |
//! | `CreateUserInput` | `pos_engine::commands::auth::CreateUserInput` | auth | create_user |
//! | `Integrations` | `pos_engine::commands::integrations::Integrations` | auth | get_integrations, set_hubrise_enabled |
//! | `SupportRunning` | `pos_engine::commands::support::SupportRunning` | auth | remote_support_running |
//! | `SupportStatus` | `pos_engine::commands::support::SupportStatus` | auth | remote_support_status, start_remote_support, stop_remote_support |
//! | `SwitchChange` | `pos_db::queries::integrations::SwitchChange` | auth | get_integrations, set_hubrise_enabled |
//! | `User` | `pos_core::user::User` | auth | create_user, get_current_user, list_users (+2 commands) |
//! | `UserRole` | `pos_core::user::UserRole` | auth | create_user, get_current_user, list_users (+2 commands) |
//! | `AcceptRequest` | `pos_engine::commands::webshop::AcceptRequest` | hubrise | webshop_accept_order |
//! | `AcceptResult` | `pos_engine::commands::webshop::AcceptResult` | hubrise | webshop_accept_order |
//! | `Adoption` | `pos_catalog::apply::Adoption` | hubrise | hubrise_adopt_catalog, hubrise_adoption_preview |
//! | `ApplyOutcome` | `pos_catalog::apply::ApplyOutcome` | hubrise | hubrise_adopt_catalog, hubrise_sync_catalog |
//! | `Attention` | `pos_hubrise::webshop::Attention` | hubrise | hubrise_list_webshop_orders |
//! | `AuthorizeLink` | `pos_engine::commands::hubrise::AuthorizeLink` | hubrise | hubrise_start_oauth_flow |
//! | `Backlog` | `pos_hubrise::outbox::Backlog` | hubrise | hubrise_status |
//! | `Channel` | `pos_hubrise::webshop::Channel` | hubrise | hubrise_list_channels, hubrise_set_channel |
//! | `ConnectionHealth` | `pos_hubrise::token_store::ConnectionHealth` | hubrise | hubrise_status |
//! | `HandoverRequest` | `pos_engine::commands::webshop::HandoverRequest` | hubrise | webshop_handover_order |
//! | `HubriseConnectionInfo` | `pos_engine::commands::hubrise::HubriseConnectionInfo` | hubrise | hubrise_complete_oauth, hubrise_get_connection_status |
//! | `HubriseStatus` | `pos_engine::commands::hubrise::HubriseStatus` | hubrise | hubrise_status |
//! | `Issue` | `pos_catalog::validate::Issue` | hubrise | hubrise_adopt_catalog, hubrise_status, hubrise_sync_catalog |
//! | `IssueCode` | `pos_catalog::validate::IssueCode` | hubrise | hubrise_adopt_catalog, hubrise_status, hubrise_sync_catalog |
//! | `LineDiscount` | `pos_hubrise::webshop::LineDiscount` | hubrise | hubrise_list_webshop_orders |
//! | `LineKind` | `pos_hubrise::webshop::LineKind` | hubrise | hubrise_list_webshop_orders |
//! | `PatchStatusRequest` | `pos_engine::commands::hubrise::PatchStatusRequest` | hubrise | hubrise_patch_order_status |
//! | `RefundRequest` | `pos_engine::commands::webshop::RefundRequest` | hubrise | webshop_refund_order |
//! | `ResolvedLine` | `pos_hubrise::webshop::ResolvedLine` | hubrise | hubrise_list_webshop_orders |
//! | `SetConfirmedTimeRequest` | `pos_engine::commands::hubrise::SetConfirmedTimeRequest` | hubrise | hubrise_set_confirmed_time |
//! | `SyncState` | `pos_catalog::apply::SyncState` | hubrise | hubrise_status |
//! | `Warning` | `pos_hubrise::webshop::Warning` | hubrise | hubrise_list_webshop_orders |
//! | `WebshopOrderItem` | `pos_engine::commands::hubrise::WebshopOrderItem` | hubrise | hubrise_list_webshop_orders |
//! | `WebshopOrderSummary` | `pos_engine::commands::hubrise::WebshopOrderSummary` | hubrise | hubrise_list_webshop_orders |
//! | `WebshopState` | `pos_hubrise::webshop::WebshopState` | hubrise | hubrise_list_webshop_orders |

//! The FFI records (`#[derive(uniffi::Record)]`), one file per domain, mirroring the engine's serde
//! shapes; `convert::out` and `convert::inp` move values between the two.

pub mod auth;
pub mod checkout;
pub mod common;
pub mod eft;
pub mod export;
pub mod hubrise;
pub mod orders;
pub mod reports;
