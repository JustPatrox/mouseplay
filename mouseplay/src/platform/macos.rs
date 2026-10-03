pub fn setup() {
    match crate::input::raw_input::start() {
        Ok(()) => log::info!("macOS Quartz event tap started"),
        Err(error) => log::error!("macOS input unavailable: {}", error),
    }
}

pub fn tick() {
    crate::input::raw_input::update_controller_state();
}
