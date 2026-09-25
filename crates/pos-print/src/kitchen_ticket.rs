//! Kitchen ticket formatter for HubRise webshop orders.
//!
//! Layout is **non-fiscal** - no VAT breakdown, no FDM signature,
//! no QR code. The cook only needs items, options, and notes.
//! Built on top of the same `EscPosBuilder` used by `receipt.rs` so
//! the binary command pipeline stays uniform.

use crate::escpos::{Alignment, EscPosBuilder, TextStyle};

/// Owned, formatter-friendly view of a HubRise order line. The
/// caller (the Tauri command layer) parses the raw HubRise JSON and
/// hands us only what we need to print, so this crate stays free of
/// any HubRise type dependency.
#[derive(Debug, Clone)]
pub struct KitchenItem {
    pub quantity: i64,
    pub product_name: String,
    /// Free-form option labels ("Sauce soja sucrée", "Extra wasabi"…)
    pub options: Vec<String>,
    /// Per-item customer comment, if any.
    pub notes: Option<String>,
}

#[derive(Debug, Clone)]
pub struct KitchenTicket {
    /// HubRise channel ("Deliveroo", "Uber Eats", "Web", …).
    pub source: String,
    /// Pre-formatted service-type label ("Livraison", "À emporter", …).
    pub service_type: String,
    /// Short id displayed on the slip - typically the first 8 chars
    /// of the HubRise order id.
    pub short_ref: String,
    /// Optional ready-at clock string ("12:45") to print at the top.
    pub ready_at: Option<String>,
    pub items: Vec<KitchenItem>,
    /// Order-level note from the customer (delivery instructions,
    /// allergies, …).
    pub order_notes: Option<String>,
}

/// Build the ESC/POS byte stream for a kitchen ticket. The output
/// is ready to send to a `TcpPrinter`.
pub fn format_kitchen_ticket(ticket: &KitchenTicket) -> Vec<u8> {
    let mut p = EscPosBuilder::new();

    // ── Header ────────────────────────────────────────────────────
    p.align(Alignment::Center);
    p.style(TextStyle {
        bold: true,
        double_height: true,
        double_width: true,
        ..Default::default()
    });
    p.textln("CUISINE");
    p.style_reset();
    p.separator('=');

    p.align(Alignment::Center);
    p.style(TextStyle {
        bold: true,
        ..Default::default()
    });
    p.textln(&format!("{}  |  {}", ticket.source, ticket.service_type));
    p.style_reset();
    p.textln(&format!("Ref: {}", ticket.short_ref));
    if let Some(ready) = &ticket.ready_at {
        p.style(TextStyle {
            bold: true,
            ..Default::default()
        });
        p.textln(&format!("Pret a: {}", ready));
        p.style_reset();
    }

    p.separator('-');

    // ── Items ─────────────────────────────────────────────────────
    p.align(Alignment::Left);
    for item in &ticket.items {
        // Main line: " 2x  Maki saumon x8"
        let line = format!(" {}x {}", item.quantity, item.product_name);
        p.style(TextStyle {
            bold: true,
            ..Default::default()
        });
        p.textln(&line);
        p.style_reset();

        // Options indented with a chevron
        for opt in &item.options {
            p.textln(&format!("    > {}", opt));
        }

        // Per-item customer note in quotes
        if let Some(note) = &item.notes {
            p.textln(&format!("    \"{}\"", note));
        }
    }

    // ── Order-level notes ────────────────────────────────────────
    if let Some(note) = &ticket.order_notes {
        p.separator('-');
        p.align(Alignment::Center);
        p.style(TextStyle {
            bold: true,
            ..Default::default()
        });
        p.textln(&format!("\"{}\"", note));
        p.style_reset();
    }

    // ── End ──────────────────────────────────────────────────────
    p.feed(4);
    p.cut();
    p.build()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> KitchenTicket {
        KitchenTicket {
            source: "Deliveroo".to_string(),
            service_type: "Livraison".to_string(),
            short_ref: "abcd1234".to_string(),
            ready_at: Some("12:45".to_string()),
            items: vec![
                KitchenItem {
                    quantity: 2,
                    product_name: "Maki saumon x8".to_string(),
                    options: vec!["Sauce soja sucree".to_string()],
                    notes: Some("sans gingembre".to_string()),
                },
                KitchenItem {
                    quantity: 1,
                    product_name: "California Roll x8".to_string(),
                    options: vec![],
                    notes: None,
                },
            ],
            order_notes: Some("Sonner a l'etage".to_string()),
        }
    }

    #[test]
    fn contains_cuisine_header() {
        let bytes = format_kitchen_ticket(&sample());
        assert!(bytes.windows(7).any(|w| w == b"CUISINE"));
    }

    #[test]
    fn contains_channel_and_service_type() {
        let bytes = format_kitchen_ticket(&sample());
        assert!(bytes.windows(9).any(|w| w == b"Deliveroo"));
        assert!(bytes.windows(9).any(|w| w == b"Livraison"));
    }

    #[test]
    fn contains_items_with_quantity() {
        let bytes = format_kitchen_ticket(&sample());
        assert!(bytes.windows(14).any(|w| w == b"Maki saumon x8"));
        assert!(bytes.windows(2).any(|w| w == b"2x"));
    }

    #[test]
    fn contains_options_and_notes() {
        let bytes = format_kitchen_ticket(&sample());
        assert!(bytes.windows(17).any(|w| w == b"Sauce soja sucree"));
        assert!(bytes.windows(14).any(|w| w == b"sans gingembre"));
    }

    #[test]
    fn ends_with_cut() {
        let bytes = format_kitchen_ticket(&sample());
        // GS V 66 3 - partial cut
        assert!(bytes.windows(4).any(|w| w == [0x1D, b'V', 66, 3]));
    }

    #[test]
    fn contains_order_level_notes() {
        let bytes = format_kitchen_ticket(&sample());
        assert!(bytes.windows(15).any(|w| w == b"Sonner a l'etag"));
    }

    #[test]
    fn omits_ready_at_and_order_notes_blocks_when_absent() {
        let mut ticket = sample();
        ticket.ready_at = None;
        ticket.order_notes = None;
        let text = crate::render::render_plain_text(&format_kitchen_ticket(&ticket));
        assert!(!text.contains("Pret a:"));
    }

    #[test]
    fn cjk_product_names_and_options_degrade_to_question_marks_not_panic() {
        let mut ticket = sample();
        ticket.items.push(KitchenItem {
            quantity: 1,
            product_name: "寿司盛り合わせ".to_string(),
            options: vec!["わさび抜き".to_string()],
            notes: None,
        });
        let text = crate::render::render_plain_text(&format_kitchen_ticket(&ticket));
        assert!(text.contains('?'), "CJK chars must degrade to '?':\n{text}");
    }
}
