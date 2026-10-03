mod ffi;

pub use crate::controller::state::ControllerState;
use std::ffi::{CStr, CString};
use std::time::Duration;

const SUCCESS: i32 = 0;
const NOT_LINKED: i32 = 1;
const REGISTRATION_FAILED: i32 = 4;

const SESSION_AUTH_SIZE: usize = 16;

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
    BridgeUnavailable,
    InvalidState,
    SessionNotInitialized,
    RegistrationFailed,
    Core(i32),
}

impl std::fmt::Display for RemotePlayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BridgeUnavailable => write!(f, "Chiaki bridge is unavailable"),
            Self::InvalidState => write!(f, "invalid Chiaki bridge state"),
            Self::SessionNotInitialized => write!(f, "Chiaki session is not initialized"),
            Self::RegistrationFailed => write!(f, "Chiaki discovery or registration failed"),
            Self::Core(code) => write!(f, "Chiaki bridge error {code}"),
        }
    }
}

impl std::error::Error for RemotePlayError {}

pub struct ChiakiRemotePlay {
    context: *mut ffi::ChiakiBridgeContext,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChiakiConnectionConfig {
    pub host: String,
    pub ps5: bool,
    pub regist_key: [u8; SESSION_AUTH_SIZE],
    pub morning: [u8; SESSION_AUTH_SIZE],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChiakiDiscoveredHost {
    pub host: String,
    pub ps5: bool,
    pub target: i32,
    pub state: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChiakiRegistrationCredentials {
    pub target: i32,
    pub regist_key: [u8; SESSION_AUTH_SIZE],
    pub morning: [u8; SESSION_AUTH_SIZE],
}

impl ChiakiRegistrationCredentials {
    pub fn into_connection_config(self, host: String, ps5: bool) -> ChiakiConnectionConfig {
        ChiakiConnectionConfig {
            host,
            ps5,
            regist_key: self.regist_key,
            morning: self.morning,
        }
    }
}

impl ChiakiRemotePlay {
    pub fn new() -> Result<Self, RemotePlayError> {
        let context = unsafe { ffi::mouseplay_chiaki_context_new() };
        if context.is_null() {
            return Err(RemotePlayError::BridgeUnavailable);
        }
        Ok(Self { context })
    }

    /// Initializes a Chiaki session from data obtained by Chiaki discovery/registration.
    /// Mouseplay deliberately does not discover, pair, or register consoles itself.
    pub fn configure_session(
        &mut self,
        config: &ChiakiConnectionConfig,
    ) -> Result<(), RemotePlayError> {
        let host = std::ffi::CString::new(config.host.as_str())
            .map_err(|_| RemotePlayError::InvalidState)?;
        self.call(unsafe {
            ffi::mouseplay_chiaki_context_init_session(
                self.context,
                host.as_ptr(),
                config.ps5,
                config.regist_key.as_ptr(),
                config.morning.as_ptr(),
            )
        })
    }

    /// Runs Chiaki's direct discovery request against an address.
    pub fn discover(
        &mut self,
        address: &str,
        ps5: bool,
        timeout: Duration,
    ) -> Result<ChiakiDiscoveredHost, RemotePlayError> {
        let address = CString::new(address).map_err(|_| RemotePlayError::InvalidState)?;
        let mut result = ffi::ChiakiDiscoveryResult {
            host: [0; 256],
            ps5: false,
            target: 0,
            state: 0,
        };
        self.call(unsafe {
            ffi::mouseplay_chiaki_discover(
                self.context,
                address.as_ptr(),
                ps5,
                timeout.as_millis().min(u64::MAX as u128) as u64,
                &mut result,
            )
        })?;
        let host = unsafe { CStr::from_ptr(result.host.as_ptr()) }
            .to_str()
            .map_err(|_| RemotePlayError::InvalidState)?
            .to_owned();
        Ok(ChiakiDiscoveredHost {
            host,
            ps5: result.ps5,
            target: result.target,
            state: result.state,
        })
    }

    /// Runs Chiaki's registration flow and returns its opaque session credentials.
    pub fn register_ps5(
        &mut self,
        host: &str,
        target: i32,
        pin: u32,
        console_pin: u32,
        psn_account_id: &[u8; 8],
    ) -> Result<ChiakiRegistrationCredentials, RemotePlayError> {
        let host = CString::new(host).map_err(|_| RemotePlayError::InvalidState)?;
        let mut result = ffi::ChiakiRegistrationResult {
            target: 0,
            regist_key: [0; SESSION_AUTH_SIZE],
            morning: [0; SESSION_AUTH_SIZE],
        };
        self.call(unsafe {
            ffi::mouseplay_chiaki_register(
                self.context,
                host.as_ptr(),
                target,
                pin,
                console_pin,
                psn_account_id.as_ptr(),
                std::ptr::null(),
                &mut result,
            )
        })?;
        Ok(ChiakiRegistrationCredentials {
            target: result.target,
            regist_key: result.regist_key,
            morning: result.morning,
        })
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

    pub fn take_video_frame(&mut self) -> Result<Option<(u32, u32, Vec<u8>)>, RemotePlayError> {
        let mut rgba = std::ptr::null_mut();
        let mut size = 0usize;
        let mut width = 0u32;
        let mut height = 0u32;
        let code = unsafe {
            ffi::mouseplay_chiaki_context_take_video_frame(
                self.context,
                &mut rgba,
                &mut size,
                &mut width,
                &mut height,
            )
        };
        if code == REGISTRATION_FAILED {
            return Ok(None);
        }
        self.call(code)?;
        if rgba.is_null() {
            return Ok(None);
        }
        let bytes = unsafe { std::slice::from_raw_parts(rgba, size).to_vec() };
        unsafe { ffi::mouseplay_chiaki_video_frame_free(rgba) };
        Ok(Some((width, height, bytes)))
    }

    fn call(&self, code: i32) -> Result<(), RemotePlayError> {
        match code {
            SUCCESS => Ok(()),
            NOT_LINKED => Err(RemotePlayError::BridgeUnavailable),
            2 => Err(RemotePlayError::InvalidState),
            3 => Err(RemotePlayError::SessionNotInitialized),
            REGISTRATION_FAILED => Err(RemotePlayError::RegistrationFailed),
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

    #[cfg(target_os = "macos")]
    #[test]
    fn linked_core_initializes_without_a_remote_play_session() {
        let mut remote_play = ChiakiRemotePlay::new().expect("linked libchiaki must initialize");
        assert_eq!(
            remote_play.start(),
            Err(RemotePlayError::SessionNotInitialized)
        );
        assert_eq!(
            remote_play.send_controller_state(&ControllerState::default()),
            Err(RemotePlayError::SessionNotInitialized)
        );
        assert_eq!(
            remote_play.stop(),
            Err(RemotePlayError::SessionNotInitialized)
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn configured_session_uses_chiaki_session_state_without_connecting() {
        let mut remote_play = ChiakiRemotePlay::new().expect("linked libchiaki must initialize");
        let config = ChiakiConnectionConfig {
            host: "127.0.0.1".to_owned(),
            ps5: true,
            regist_key: [0x11; SESSION_AUTH_SIZE],
            morning: [0x22; SESSION_AUTH_SIZE],
        };

        remote_play
            .configure_session(&config)
            .expect("Chiaki should initialize a session from registered connection data");
        remote_play
            .send_controller_state(&ControllerState::default())
            .expect("controller state should be accepted by the initialized Chiaki session");
    }

    #[test]
    fn registration_credentials_become_session_configuration_without_reencoding() {
        let credentials = ChiakiRegistrationCredentials {
            target: 1_000_100,
            regist_key: [0x11; SESSION_AUTH_SIZE],
            morning: [0x22; SESSION_AUTH_SIZE],
        };
        let config = credentials.into_connection_config("192.0.2.10".to_owned(), true);
        assert_eq!(config.host, "192.0.2.10");
        assert!(config.ps5);
        assert_eq!(config.regist_key, [0x11; SESSION_AUTH_SIZE]);
        assert_eq!(config.morning, [0x22; SESSION_AUTH_SIZE]);
    }
}
