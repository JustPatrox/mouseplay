//! Temporary platform-neutral input surface for the phase 1 build boundary.

pub struct RawInput {
    mouse: [i32; 2],
}

impl RawInput {
    pub fn new() -> Result<Self, &'static str> {
        Ok(Self { mouse: [0; 2] })
    }

    pub fn key(&self, _button: &str) -> bool {
        false
    }

    pub fn mouse_x(&self) -> i32 {
        self.mouse[0]
    }

    pub fn mouse_y(&self) -> i32 {
        self.mouse[1]
    }

    pub fn accumulate(&mut self) {}
}

#[allow(dead_code)]
pub fn hijack_wndproc() -> Result<(), &'static str> {
    Err("window input capture is not implemented on macOS yet")
}
