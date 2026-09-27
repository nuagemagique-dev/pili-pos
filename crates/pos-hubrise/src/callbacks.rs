//! The passive callback the POS polls (`docs/hubrise/06-callbacks.md`).
//!
//! HubRise records events for a connection only once it has a
//! callback asking for them; with `url: null` it keeps them for
//! polling instead of calling a URL. Without this registration the
//! POS would never hear of an order.

use pos_hubrise_types::CallbackRegistration;
use serde_json::{Value, json};

use crate::client::Client;
use crate::errors::Result;

/// The events the POS needs: orders placed or changed, catalog changes.
pub fn wanted_events() -> Value {
    json!({
        "order": ["create", "update"],
        "catalog": ["create", "update"],
    })
}

/// Whether `registered` asks for every event of [`wanted_events`]
/// with no URL (passive).
pub fn covers(registered: &CallbackRegistration) -> bool {
    if registered.url.is_some() {
        return false;
    }
    let wanted = wanted_events();
    wanted.as_object().is_some_and(|w| {
        w.iter().all(|(resource, types)| {
            let have = registered.events.get(resource).and_then(Value::as_array);
            types.as_array().is_some_and(|types| {
                types
                    .iter()
                    .all(|t| have.is_some_and(|have| have.contains(t)))
            })
        })
    })
}

/// Register the passive callback unless it already is. Returns true
/// when it had to be (re)registered.
pub async fn ensure_passive(client: &Client) -> Result<bool> {
    let current: CallbackRegistration = client.get_json("/callback").await?;
    if covers(&current) {
        return Ok(false);
    }
    let body = json!({ "url": null, "events": wanted_events() });
    let _: Value = client.post_json("/callback", &body).await?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reg(url: Option<&str>, events: Value) -> CallbackRegistration {
        CallbackRegistration {
            url: url.map(str::to_string),
            events,
        }
    }

    #[test]
    fn a_complete_passive_registration_covers() {
        assert!(covers(&reg(None, wanted_events())));
        let more = json!({ "order": ["create", "update"], "catalog": ["create", "update", "delete"], "customer": ["create"] });
        assert!(covers(&reg(None, more)));
    }

    #[test]
    fn a_partial_or_active_registration_does_not() {
        assert!(!covers(&reg(None, json!({}))));
        assert!(!covers(&reg(None, json!({ "order": ["create"] }))));
        assert!(!covers(&reg(Some("https://x"), wanted_events())));
    }
}
