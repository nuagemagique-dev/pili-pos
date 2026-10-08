use std::sync::OnceLock;

static SINK: OnceLock<u8> = OnceLock::new();

#[cfg(target_os = "android")]
pub fn init() {}

#[cfg(not(target_os = "android"))]
pub fn init() {}
