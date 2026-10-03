mod ffi;

pub use crate::controller::state::ControllerState;

const SUCCESS: i32 = 0;
const NOT_LINKED: i32 = 1;

const CROSS: u32 = 1 << 0;
const CIRCLE: u32 = 1 << 1;
const SQUARE: u32 = 1 << 2;
const TRIANGLE: u32 = 1 << 3;
const L1: u32 = 1 << 8;
const R1: u32 = 1 << 9;
const L3: u32 = 1 << 10;
const R3: u32 = 1 << 11;
const OPTIONS: u32 = 1 << 12;
const SHARE: u32 = 1 << 13;
const TOUCHPAD: u32 = 1 << 14;
const PS: u32 = 1 << 15;
const ANALOG_L2: u32 = 1 << 16;
const ANALOG_R2: u32 = 1 << 17;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemotePlayError {
    CoreNotLinked,
    InvalidState,
    Core(i32),
}

impl std::fmt::Display for RemotePlayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CoreNotLinked => write!(f, "Chiaki core is not linked"),
            Self::InvalidState => write!(f, "invalid Chiaki bridge state"),
            Self::Core(code) => write!(f, "Chiaki bridge error {code}"),
        }
    }
}

impl std::error::Error for RemotePlayError {}

pub struct ChiakiRemotePlay {
    context: *mut ffi::ChiakiBridgeContext,
}

impl ChiakiRemotePlay {
    pub fn new() -> Result<Self, RemotePlayError> {
        let context = unsafe { ffi::mouseplay_chiaki_context_new() };
        if context.is_null() {
            return Err(RemotePlayError::CoreNotLinked);
        }
        Ok(Self { context })
    }

    pub fn start(&mut self) -> Result<(), RemotePlayError> {
        self.call(unsafe { ffi::mouseplay_chiaki_context_start(self.context) })
    }

    pub fn send_controller_state(
        &mut self,
        state: &ControllerState,
    ) -> Result<(), RemotePlayError> {
        let state = to_chiaki_controller_state(state);
        self.call(unsafe {
            ffi::mouseplay_chiaki_context_set_controller_state(self.context, &state)
        })
    }

    pub fn stop(&mut self) -> Result<(), RemotePlayError> {
        self.call(unsafe { ffi::mouseplay_chiaki_context_stop(self.context) })
    }

    fn call(&self, code: i32) -> Result<(), RemotePlayError> {
        match code {
            SUCCESS => Ok(()),
            NOT_LINKED => Err(RemotePlayError::CoreNotLinked),
            2 => Err(RemotePlayError::InvalidState),
            other => Err(RemotePlayError::Core(other)),
        }
    }
}

impl Drop for ChiakiRemotePlay {
    fn drop(&mut self) {
        unsafe { ffi::mouseplay_chiaki_context_free(self.context) }
    }
}

pub(crate) fn to_chiaki_controller_state(state: &ControllerState) -> ffi::ChiakiControllerState {
    let mut buttons = 0;
    let b = &state.buttons;
    if b.cross {
        buttons |= CROSS;
    }
    if b.circle {
        buttons |= CIRCLE;
    }
    if b.square {
        buttons |= SQUARE;
    }
    if b.triangle {
        buttons |= TRIANGLE;
    }
    if b.l1 {
        buttons |= L1;
    }
    if b.r1 {
        buttons |= R1;
    }
    if b.l3 {
        buttons |= L3;
    }
    if b.r3 {
        buttons |= R3;
    }
    if b.options {
        buttons |= OPTIONS;
    }
    if b.share {
        buttons |= SHARE;
    }
    if b.touch {
        buttons |= TOUCHPAD;
    }
    if b.ps {
        buttons |= PS;
    }
    if b.l2 {
        buttons |= ANALOG_L2;
    }
    if b.r2 {
        buttons |= ANALOG_R2;
    }

    ffi::ChiakiControllerState {
        buttons,
        l2_state: state.l2,
        r2_state: state.r2,
        left_x: stick_value(state.left_stick.x),
        left_y: stick_value(state.left_stick.y),
        right_x: stick_value(state.right_stick.x),
        right_y: stick_value(state.right_stick.y),
        touch_id_next: 0,
        touches: [
            ffi::ChiakiControllerTouch { x: 0, y: 0, id: -1 },
            ffi::ChiakiControllerTouch { x: 0, y: 0, id: -1 },
        ],
        gyro_x: 0.0,
        gyro_y: 0.0,
        gyro_z: 0.0,
        accel_x: 0.0,
        accel_y: 1.0,
        accel_z: 0.0,
        orient_x: 0.0,
        orient_y: 0.0,
        orient_z: 0.0,
        orient_w: 1.0,
    }
}

fn stick_value(value: u8) -> i16 {
    if value < 128 {
        -((128 - value) as i32 * 32_768 / 128) as i16
    } else {
        ((value - 128) as i32 * 32_767 / 127) as i16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_state_maps_to_neutral_chiaki_state() {
        let mapped = to_chiaki_controller_state(&ControllerState::default());
        assert_eq!(mapped.buttons, 0);
        assert_eq!(mapped.left_x, 0);
        assert_eq!(mapped.left_y, 0);
        assert_eq!(mapped.right_x, 0);
        assert_eq!(mapped.right_y, 0);
        assert_eq!(mapped.l2_state, 0);
        assert_eq!(mapped.r2_state, 0);
        assert_eq!(mapped.touches[0].id, -1);
    }

    #[test]
    fn sticks_cover_signed_extremes() {
        assert_eq!(stick_value(0), -32_768);
        assert_eq!(stick_value(128), 0);
        assert_eq!(stick_value(255), 32_767);
    }

    #[test]
    fn triggers_are_copied_and_digital_bits_are_separate() {
        let mut state = ControllerState::default();
        state.l2 = 17;
        state.r2 = 231;
        let mapped = to_chiaki_controller_state(&state);
        assert_eq!(mapped.l2_state, 17);
        assert_eq!(mapped.r2_state, 231);
        assert_eq!(mapped.buttons, 0);

        state.buttons.l2 = true;
        state.buttons.r2 = true;
        let mapped = to_chiaki_controller_state(&state);
        assert_eq!(mapped.buttons, ANALOG_L2 | ANALOG_R2);
    }

    #[test]
    fn all_supported_buttons_map_to_chiaki_bits() {
        let mut state = ControllerState::default();
        state.buttons.triangle = true;
        state.buttons.circle = true;
        state.buttons.cross = true;
        state.buttons.square = true;
        state.buttons.l1 = true;
        state.buttons.r1 = true;
        state.buttons.l2 = true;
        state.buttons.r2 = true;
        state.buttons.l3 = true;
        state.buttons.r3 = true;
        state.buttons.share = true;
        state.buttons.options = true;
        state.buttons.ps = true;
        state.buttons.touch = true;
        let mapped = to_chiaki_controller_state(&state);
        assert_eq!(mapped.buttons, 0x3ff0f);
    }

    #[test]
    fn repeated_snapshots_do_not_retain_previous_button_bits() {
        let mut state = ControllerState::default();
        state.buttons.cross = true;
        assert_eq!(to_chiaki_controller_state(&state).buttons, CROSS);
        state.buttons.cross = false;
        state.buttons.circle = true;
        assert_eq!(to_chiaki_controller_state(&state).buttons, CIRCLE);
    }

    #[cfg(not(windows))]
    #[test]
    fn rust_ffi_layout_matches_pinned_chiaki_header() {
        assert_eq!(std::mem::size_of::<ffi::ChiakiControllerState>(), unsafe {
            ffi::mouseplay_chiaki_controller_state_size()
        });
        assert_eq!(std::mem::align_of::<ffi::ChiakiControllerState>(), unsafe {
            ffi::mouseplay_chiaki_controller_state_alignment()
        });
    }
}
