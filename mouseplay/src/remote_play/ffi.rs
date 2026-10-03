#![allow(dead_code)]

use std::os::raw::c_int;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct ChiakiControllerTouch {
    pub x: u16,
    pub y: u16,
    pub id: i8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ChiakiControllerState {
    pub buttons: u32,
    pub l2_state: u8,
    pub r2_state: u8,
    pub left_x: i16,
    pub left_y: i16,
    pub right_x: i16,
    pub right_y: i16,
    pub touch_id_next: u8,
    pub touches: [ChiakiControllerTouch; 2],
    pub gyro_x: f32,
    pub gyro_y: f32,
    pub gyro_z: f32,
    pub accel_x: f32,
    pub accel_y: f32,
    pub accel_z: f32,
    pub orient_x: f32,
    pub orient_y: f32,
    pub orient_z: f32,
    pub orient_w: f32,
}

#[repr(C)]
pub(crate) struct ChiakiBridgeContext {
    _private: [u8; 0],
}

#[repr(C)]
pub(crate) struct ChiakiDiscoveryResult {
    pub host: [std::os::raw::c_char; 256],
    pub ps5: bool,
    pub target: c_int,
    pub state: c_int,
}

#[repr(C)]
pub(crate) struct ChiakiRegistrationResult {
    pub target: c_int,
    pub regist_key: [u8; 16],
    pub morning: [u8; 16],
}

extern "C" {
    pub(crate) fn mouseplay_chiaki_context_new() -> *mut ChiakiBridgeContext;
    pub(crate) fn mouseplay_chiaki_discover(
        context: *mut ChiakiBridgeContext,
        address: *const std::os::raw::c_char,
        ps5: bool,
        timeout_ms: u64,
        result: *mut ChiakiDiscoveryResult,
    ) -> c_int;
    pub(crate) fn mouseplay_chiaki_register(
        context: *mut ChiakiBridgeContext,
        host: *const std::os::raw::c_char,
        target: c_int,
        pin: u32,
        console_pin: u32,
        psn_account_id: *const u8,
        psn_online_id: *const std::os::raw::c_char,
        result: *mut ChiakiRegistrationResult,
    ) -> c_int;
    pub(crate) fn mouseplay_chiaki_context_init_session(
        context: *mut ChiakiBridgeContext,
        host: *const std::os::raw::c_char,
        ps5: bool,
        regist_key: *const u8,
        morning: *const u8,
    ) -> c_int;
    pub(crate) fn mouseplay_chiaki_context_start(context: *mut ChiakiBridgeContext) -> c_int;
    pub(crate) fn mouseplay_chiaki_context_set_controller_state(
        context: *mut ChiakiBridgeContext,
        state: *const ChiakiControllerState,
    ) -> c_int;
    pub(crate) fn mouseplay_chiaki_context_stop(context: *mut ChiakiBridgeContext) -> c_int;
    pub(crate) fn mouseplay_chiaki_context_free(context: *mut ChiakiBridgeContext);
    pub(crate) fn mouseplay_chiaki_controller_state_size() -> usize;
    pub(crate) fn mouseplay_chiaki_controller_state_alignment() -> usize;
}
