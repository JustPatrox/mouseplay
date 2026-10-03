#[cfg(windows)]
pub mod raw_input;
#[cfg(target_os = "macos")]
#[path = "macos_input.rs"]
pub mod raw_input;
