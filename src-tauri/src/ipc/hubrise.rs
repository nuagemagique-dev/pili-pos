use tauri::State;

use crate::commands::hubrise::{
    AuthorizeLink, HubriseConnectionInfo, HubriseStatus, PatchStatusRequest,
    SetConfirmedTimeRequest, WebshopOrderSummary,
};

use super::Managed;

#[tauri::command]
pub fn hubrise_get_connection_status(
    state: State<'_, Managed>,
) -> Result<HubriseConnectionInfo, String> {
    crate::commands::hubrise::do_hubrise_get_connection_status(state.inner())
}

#[tauri::command]
pub fn hubrise_start_oauth_flow(state: State<'_, Managed>) -> Result<AuthorizeLink, String> {
    crate::commands::hubrise::do_hubrise_start_oauth_flow(state.inner())
}

#[tauri::command]
pub async fn hubrise_complete_oauth(
    state: State<'_, Managed>,
    code: String,
) -> Result<HubriseConnectionInfo, String> {
    crate::commands::hubrise::do_hubrise_complete_oauth(state.inner(), &code).await
}

#[tauri::command]
pub fn hubrise_disconnect(state: State<'_, Managed>) -> Result<(), String> {
    crate::commands::hubrise::do_hubrise_disconnect(state.inner())
}

#[tauri::command]
pub fn hubrise_list_webshop_orders(
    state: State<'_, Managed>,
) -> Result<Vec<WebshopOrderSummary>, String> {
    crate::commands::hubrise::do_list_webshop_orders(state.inner())
}

#[tauri::command]
pub async fn hubrise_patch_order_status(
    state: State<'_, Managed>,
    req: PatchStatusRequest,
) -> Result<(), String> {
    crate::commands::hubrise::do_patch_order_status(
        state.inner(),
        &req.hubrise_order_id,
        &req.new_status,
    )
}

/// Queue a new `confirmed_time` for HubRise and show it at once.
#[tauri::command]
pub async fn hubrise_set_confirmed_time(
    state: State<'_, Managed>,
    req: SetConfirmedTimeRequest,
) -> Result<(), String> {
    crate::commands::hubrise::do_set_confirmed_time(
        state.inner(),
        &req.hubrise_order_id,
        &req.confirmed_time,
    )
}

#[tauri::command]
pub fn hubrise_status(state: State<'_, Managed>) -> Result<HubriseStatus, String> {
    crate::commands::hubrise::do_hubrise_status(state.inner())
}

/// Fetch and apply the HubRise catalog now (Settings, "Resynchroniser").
#[tauri::command]
pub async fn hubrise_sync_catalog(
    state: State<'_, Managed>,
) -> Result<Option<pos_catalog::ApplyOutcome>, String> {
    crate::commands::hubrise::do_sync_catalog(state.inner()).await
}

/// The local products that can keep their id when HubRise takes over
/// the menu (same French name and price as a catalog product).
#[tauri::command]
pub fn hubrise_adoption_preview(
    state: State<'_, Managed>,
) -> Result<Vec<pos_catalog::Adoption>, String> {
    crate::commands::hubrise::do_adoption_preview(state.inner())
}

/// The manager confirms the take-over: the chosen products keep their
/// id, every other hand-entered product is deactivated, and the menu
/// mirrors HubRise. Refused while a POS order is open, since its
/// products could vanish from the menu.
#[tauri::command]
pub fn hubrise_adopt_catalog(
    state: State<'_, Managed>,
    adoptions: Vec<pos_catalog::Adoption>,
) -> Result<pos_catalog::ApplyOutcome, String> {
    crate::commands::hubrise::do_adopt_catalog(state.inner(), &adoptions)
}

/// How each HubRise channel is signed (Settings).
#[tauri::command]
pub fn hubrise_list_channels(
    state: State<'_, Managed>,
) -> Result<Vec<pos_hubrise::webshop::Channel>, String> {
    crate::commands::hubrise::do_list_channels(state.inner())
}

/// Add or change a channel; pending orders resolve again. Returns how
/// many of them now resolve cleanly.
#[tauri::command]
pub fn hubrise_set_channel(
    state: State<'_, Managed>,
    channel: pos_hubrise::webshop::Channel,
) -> Result<usize, String> {
    crate::commands::hubrise::do_set_channel(state.inner(), &channel)
}
