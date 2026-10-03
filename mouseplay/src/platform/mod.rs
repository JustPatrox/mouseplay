#[cfg(windows)]
pub mod windows;

#[cfg(target_os = "macos")]
pub mod macos;

pub fn setup() {
    #[cfg(windows)]
    windows::setup();

    #[cfg(target_os = "macos")]
    macos::setup();
}
