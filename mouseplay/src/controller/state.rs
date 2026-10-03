//! Platform-independent logical controller state.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stick {
    pub x: u8,
    pub y: u8,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Buttons {
    pub triangle: bool,
    pub circle: bool,
    pub cross: bool,
    pub square: bool,
    pub l1: bool,
    pub r1: bool,
    pub l2: bool,
    pub r2: bool,
    pub l3: bool,
    pub r3: bool,
    pub share: bool,
    pub options: bool,
    pub ps: bool,
    pub touch: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ControllerState {
    pub left_stick: Stick,
    pub right_stick: Stick,
    pub l2: u8,
    pub r2: u8,
    pub buttons: Buttons,
}

impl Default for ControllerState {
    fn default() -> Self {
        Self {
            left_stick: Stick { x: 128, y: 128 },
            right_stick: Stick { x: 128, y: 128 },
            l2: 0,
            r2: 0,
            buttons: Buttons::default(),
        }
    }
}

impl ControllerState {
    pub fn set_button(&mut self, button: &str, down: bool) {
        match button {
            "triangle" => self.buttons.triangle = down,
            "circle" => self.buttons.circle = down,
            "cross" => self.buttons.cross = down,
            "square" => self.buttons.square = down,
            "l1" => self.buttons.l1 = down,
            "r1" => self.buttons.r1 = down,
            "l2" => self.buttons.l2 = down,
            "r2" => self.buttons.r2 = down,
            "l3" => self.buttons.l3 = down,
            "r3" => self.buttons.r3 = down,
            "share" => self.buttons.share = down,
            "options" => self.buttons.options = down,
            "ps" => self.buttons.ps = down,
            "touch" => self.buttons.touch = down,
            _ => (),
        }
    }

    pub fn set_axis(&mut self, axis: &str, value: u8) {
        match axis {
            "lx" => self.left_stick.x = value,
            "ly" => self.left_stick.y = value,
            "rx" => self.right_stick.x = value,
            "ry" => self.right_stick.y = value,
            "l2" => self.l2 = value,
            "r2" => self.r2 = value,
            _ => (),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ControllerState;

    #[test]
    fn starts_centered_and_released() {
        let state = ControllerState::default();

        assert_eq!(state.left_stick.x, 128);
        assert_eq!(state.left_stick.y, 128);
        assert_eq!(state.right_stick.x, 128);
        assert_eq!(state.right_stick.y, 128);
        assert_eq!(state.l2, 0);
        assert_eq!(state.r2, 0);
        assert_eq!(state.buttons, Default::default());
    }

    #[test]
    fn updates_buttons_sticks_and_triggers() {
        let mut state = ControllerState::default();

        state.set_button("cross", true);
        state.set_button("touch", true);
        state.set_axis("lx", 10);
        state.set_axis("ry", 240);
        state.set_axis("l2", 80);
        state.set_axis("r2", 200);

        assert!(state.buttons.cross);
        assert!(state.buttons.touch);
        assert_eq!(state.left_stick.x, 10);
        assert_eq!(state.right_stick.y, 240);
        assert_eq!(state.l2, 80);
        assert_eq!(state.r2, 200);
    }
}
