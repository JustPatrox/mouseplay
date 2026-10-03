//! Global macOS keyboard and mouse input backed by a Quartz event tap.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;
use std::thread;

use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
use core_graphics::event::{
    CGEvent, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType,
    EventField, KeyCode,
};
use lazy_static::lazy_static;

use crate::controller::state::ControllerState;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> u8;
}

lazy_static! {
    pub static ref RAW_INPUT: RwLock<RawInput> = RwLock::new(RawInput::new().unwrap());
    pub static ref CONTROLLER_STATE: RwLock<ControllerState> =
        RwLock::new(ControllerState::default());
}

static EVENT_TAP_STARTED: AtomicBool = AtomicBool::new(false);

pub struct RawInput {
    keys: [bool; 128],
    mouse_buttons: [bool; 5],
    mouse: [i32; 2],
    mouse_accumulator: [i32; 2],
}

impl RawInput {
    pub fn new() -> Result<Self, &'static str> {
        Ok(Self {
            keys: [false; 128],
            mouse_buttons: [false; 5],
            mouse: [0; 2],
            mouse_accumulator: [0; 2],
        })
    }

    pub fn key(&self, button: &str) -> bool {
        match button {
            "mouse1" => self.mouse_buttons[0],
            "mouse2" => self.mouse_buttons[1],
            "mouse3" => self.mouse_buttons[2],
            "mouse4" => self.mouse_buttons[3],
            "mouse5" => self.mouse_buttons[4],
            _ => key_code(button)
                .and_then(|code| self.keys.get(code as usize).copied())
                .unwrap_or(false),
        }
    }

    pub fn mouse_x(&self) -> i32 {
        self.mouse[0]
    }

    pub fn mouse_y(&self) -> i32 {
        self.mouse[1]
    }

    /// Moves accumulated event-tap deltas into the frame consumed by the mapper.
    pub fn accumulate(&mut self) {
        self.mouse = self.mouse_accumulator;
        self.mouse_accumulator = [0; 2];
    }

    fn set_key(&mut self, code: u16, down: bool) {
        if let Some(key) = self.keys.get_mut(code as usize) {
            *key = down;
        }
    }

    fn add_mouse_delta(&mut self, x: i32, y: i32) {
        self.mouse_accumulator[0] += x;
        self.mouse_accumulator[1] += y;
    }

    fn set_mouse_button(&mut self, button: usize, down: bool) {
        if let Some(state) = self.mouse_buttons.get_mut(button) {
            *state = down;
        }
    }

    #[cfg(test)]
    pub fn with_mouse(mouse: [i32; 2]) -> Self {
        Self {
            keys: [false; 128],
            mouse_buttons: [false; 5],
            mouse,
            mouse_accumulator: [0; 2],
        }
    }
}

pub fn start() -> Result<(), &'static str> {
    if EVENT_TAP_STARTED.swap(true, Ordering::SeqCst) {
        return Ok(());
    }

    if unsafe { AXIsProcessTrusted() } == 0 {
        EVENT_TAP_STARTED.store(false, Ordering::SeqCst);
        return Err(
            "macOS Accessibility permission is required; enable Mouseplay in System Settings > Privacy & Security > Accessibility",
        );
    }

    thread::Builder::new()
        .name("mouseplay-macos-event-tap".to_string())
        .spawn(run_event_tap)
        .map_err(|_| {
            EVENT_TAP_STARTED.store(false, Ordering::SeqCst);
            "unable to start macOS event tap thread"
        })?;

    Ok(())
}

pub fn update_controller_state() {
    let Ok(mut raw_input) = RAW_INPUT.write() else {
        return;
    };
    raw_input.accumulate();

    let Ok(mut mapper) = crate::mapper::MAPPER.write() else {
        return;
    };
    let Some(mapper) = mapper.as_mut() else {
        return;
    };
    let Ok(mut state) = CONTROLLER_STATE.write() else {
        return;
    };

    mapper.map_controller(&raw_input, &mut state);
}

pub fn current_controller_state() -> ControllerState {
    CONTROLLER_STATE
        .read()
        .map(|state| *state)
        .unwrap_or_default()
}

fn run_event_tap() {
    let events = vec![
        CGEventType::KeyDown,
        CGEventType::KeyUp,
        CGEventType::MouseMoved,
        CGEventType::LeftMouseDown,
        CGEventType::LeftMouseUp,
        CGEventType::RightMouseDown,
        CGEventType::RightMouseUp,
        CGEventType::OtherMouseDown,
        CGEventType::OtherMouseUp,
        CGEventType::LeftMouseDragged,
        CGEventType::RightMouseDragged,
        CGEventType::OtherMouseDragged,
    ];

    let tap = match CGEventTap::new(
        CGEventTapLocation::HID,
        CGEventTapPlacement::HeadInsertEventTap,
        CGEventTapOptions::ListenOnly,
        events,
        |_proxy, event_type, event| {
            if let Ok(mut raw_input) = RAW_INPUT.write() {
                record_event(&mut raw_input, event_type, event);
            }
            None
        },
    ) {
        Ok(tap) => tap,
        Err(()) => {
            log::error!(
                "unable to create macOS Quartz event tap; check Input Monitoring permission"
            );
            EVENT_TAP_STARTED.store(false, Ordering::SeqCst);
            return;
        }
    };

    let run_loop = CFRunLoop::get_current();
    let source = match tap.mach_port.create_runloop_source(0) {
        Ok(source) => source,
        Err(()) => {
            log::error!("unable to create run-loop source for macOS event tap");
            EVENT_TAP_STARTED.store(false, Ordering::SeqCst);
            return;
        }
    };
    run_loop.add_source(&source, unsafe { kCFRunLoopCommonModes });
    tap.enable();
    CFRunLoop::run_current();
}

fn record_event(raw_input: &mut RawInput, event_type: CGEventType, event: &CGEvent) {
    match event_type {
        CGEventType::KeyDown | CGEventType::KeyUp => {
            let code = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);
            if (0..128).contains(&code) {
                raw_input.set_key(code as u16, matches!(event_type, CGEventType::KeyDown));
            }
        }
        CGEventType::MouseMoved
        | CGEventType::LeftMouseDragged
        | CGEventType::RightMouseDragged
        | CGEventType::OtherMouseDragged => {
            let x = event.get_integer_value_field(EventField::MOUSE_EVENT_DELTA_X);
            let y = event.get_integer_value_field(EventField::MOUSE_EVENT_DELTA_Y);
            raw_input.add_mouse_delta(x as i32, y as i32);
        }
        CGEventType::LeftMouseDown | CGEventType::LeftMouseUp => {
            raw_input.set_mouse_button(0, matches!(event_type, CGEventType::LeftMouseDown));
        }
        CGEventType::RightMouseDown | CGEventType::RightMouseUp => {
            raw_input.set_mouse_button(1, matches!(event_type, CGEventType::RightMouseDown));
        }
        CGEventType::OtherMouseDown | CGEventType::OtherMouseUp => {
            let button =
                event.get_integer_value_field(EventField::MOUSE_EVENT_BUTTON_NUMBER) as usize;
            raw_input.set_mouse_button(button, matches!(event_type, CGEventType::OtherMouseDown));
        }
        _ => {}
    }
}

fn key_code(name: &str) -> Option<u16> {
    Some(match name {
        "a" => 0,
        "s" => 1,
        "d" => 2,
        "f" => 3,
        "v" => 9,
        "r" => 15,
        "e" => 14,
        "q" => 12,
        "w" => 13,
        "space" => KeyCode::SPACE,
        "tab" => KeyCode::TAB,
        "escape" => KeyCode::ESCAPE,
        "shift" => KeyCode::SHIFT,
        "ctrl" => KeyCode::CONTROL,
        "f1" => KeyCode::F1,
        "f2" => KeyCode::F2,
        "f3" => KeyCode::F3,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::{key_code, RawInput};

    #[test]
    fn keyboard_events_use_existing_mapping_names() {
        let mut input = RawInput::new().unwrap();
        input.set_key(key_code("w").unwrap(), true);
        assert!(input.key("w"));
        input.set_key(key_code("w").unwrap(), false);
        assert!(!input.key("w"));
    }

    #[test]
    fn mouse_deltas_are_accumulated_as_relative_motion() {
        let mut input = RawInput::new().unwrap();
        input.add_mouse_delta(7, -4);
        input.add_mouse_delta(-2, 3);
        input.accumulate();

        assert_eq!(input.mouse_x(), 5);
        assert_eq!(input.mouse_y(), -1);
    }

    #[test]
    fn mouse_buttons_use_existing_mapping_names() {
        let mut input = RawInput::new().unwrap();
        input.set_mouse_button(0, true);
        input.set_mouse_button(1, true);

        assert!(input.key("mouse1"));
        assert!(input.key("mouse2"));
        assert!(!input.key("mouse3"));
    }
}
