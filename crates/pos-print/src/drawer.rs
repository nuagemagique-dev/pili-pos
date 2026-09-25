use crate::escpos::EscPosBuilder;

/// Generate the ESC/POS command to kick the cash drawer.
///
/// Command: ESC p m t1 t2
/// - m = 0x00 (connector pin 2)
/// - t1 = 0x19 (50ms on-time, in 2ms units)
/// - t2 = 0xFA (500ms off-time, in 2ms units)
///
/// Compatible with standard RJ11-connected cash drawers via
/// Epson TM-m30III printer's DK port.
pub fn kick_drawer_bytes() -> Vec<u8> {
    let mut builder = EscPosBuilder::new();
    builder.open_drawer();
    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drawer_command_is_correct() {
        let bytes = kick_drawer_bytes();
        // Should contain: ESC p 0x00 0x19 0xFA
        let expected: &[u8] = &[0x1B, b'p', 0x00, 0x19, 0xFA];
        assert!(bytes.windows(5).any(|w| w == expected));
    }

    #[test]
    fn drawer_command_starts_with_init() {
        let bytes = kick_drawer_bytes();
        // ESC @ (init) should be the first 2 bytes
        assert_eq!(&bytes[..2], &[0x1B, b'@']);
    }

    #[test]
    fn kick_drawer_bytes_is_exactly_init_plus_kick_no_extra_content() {
        // Byte-exact check: the drawer kick must be nothing but the
        // printer init sequence followed by the CLAUDE.md-documented
        // ESC/POS kick command (0x1B 0x70 0x00 0x19 0xFA) — no stray
        // text or extra commands sneaking in.
        let bytes = kick_drawer_bytes();
        assert_eq!(
            bytes,
            vec![0x1B, b'@', 0x1B, b't', 19, 0x1B, b'p', 0x00, 0x19, 0xFA]
        );
    }
}
