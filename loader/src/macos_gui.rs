#![allow(unexpected_cfgs)]

use base64::{engine::general_purpose::STANDARD, Engine as _};
use cocoa::appkit::{NSApp, NSApplication, NSApplicationActivationPolicy};
use cocoa::base::{id, nil, NO, YES};
use cocoa::foundation::{NSAutoreleasePool, NSPoint, NSRect, NSSize, NSString};
use objc::class;
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel};
use objc::{msg_send, sel, sel_impl, Encode};
use std::convert::TryFrom;
use std::ffi::{c_void, CStr};
use std::os::raw::c_char;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::OnceLock;
use std::thread;
use std::time::Duration;

use mouseplay::remote_play::{
    ChiakiConnectionConfig, ChiakiDiscoveredHost, ChiakiRegistrationCredentials, ChiakiRemotePlay,
};

const WINDOW_WIDTH: f64 = 720.0;
const WINDOW_HEIGHT: f64 = 620.0;

#[derive(Debug)]
enum Command {
    Discover {
        address: String,
    },
    Register {
        host: String,
        target: i32,
        pin: u32,
        account_id: [u8; 8],
    },
    Connect {
        config: ChiakiConnectionConfig,
    },
    Disconnect,
}

#[derive(Debug)]
enum UiEvent {
    Log(String),
    Status(String),
    Host(ChiakiDiscoveredHost),
    Credentials(ChiakiRegistrationCredentials),
}

struct WorkerHandle {
    command_tx: Sender<Command>,
}

struct GuiState {
    worker: WorkerHandle,
    events: Receiver<UiEvent>,
    credentials: Option<ChiakiRegistrationCredentials>,
}

fn start_worker(events: Sender<UiEvent>) -> Box<WorkerHandle> {
    let (command_tx, command_rx) = mpsc::channel();
    thread::Builder::new()
        .name("mouseplay-remote-play".to_owned())
        .spawn(move || worker_loop(command_rx, events))
        .expect("unable to start Mouseplay backend thread");
    Box::new(WorkerHandle { command_tx })
}

fn worker_loop(command_rx: Receiver<Command>, events: Sender<UiEvent>) {
    let mapping =
        std::env::var("MOUSEPLAY_MAPPING").unwrap_or_else(|_| "mappings/overwatch.json".to_owned());
    if let Err(error) = mouseplay::initialize_input(&mapping) {
        let _ = events.send(UiEvent::Log(format!("Input no disponible: {error}")));
    } else {
        let _ = events.send(UiEvent::Log(format!("Mapping cargado: {mapping}")));
    }

    let mut remote_play = match ChiakiRemotePlay::new() {
        Ok(remote_play) => Some(remote_play),
        Err(error) => {
            let _ = events.send(UiEvent::Log(format!("libchiaki no disponible: {error}")));
            None
        }
    };
    let mut connected = false;

    loop {
        if connected {
            match command_rx.try_recv() {
                Ok(Command::Disconnect) => {
                    if let Some(session) = remote_play.as_mut() {
                        let _ = session.stop();
                    }
                    remote_play = ChiakiRemotePlay::new().ok();
                    connected = false;
                    let _ = events.send(UiEvent::Status("Desconectado".to_owned()));
                    continue;
                }
                Ok(command) => handle_command(command, &mut remote_play, &events, &mut connected),
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => return,
            }
            mouseplay::tick_input();
            if let Some(remote_play) = remote_play.as_mut() {
                if let Err(error) =
                    remote_play.send_controller_state(&mouseplay::controller_state())
                {
                    let _ = events.send(UiEvent::Log(format!("ControllerState: {error}")));
                }
            }
            thread::sleep(Duration::from_millis(8));
        } else {
            match command_rx.recv() {
                Ok(command) => handle_command(command, &mut remote_play, &events, &mut connected),
                Err(_) => return,
            }
        }
    }
}

fn handle_command(
    command: Command,
    remote_play: &mut Option<ChiakiRemotePlay>,
    events: &Sender<UiEvent>,
    connected: &mut bool,
) {
    let Some(remote_play) = remote_play.as_mut() else {
        let _ = events.send(UiEvent::Log(
            "La biblioteca Chiaki no está disponible".to_owned(),
        ));
        return;
    };

    match command {
        Command::Discover { address } => {
            let _ = events.send(UiEvent::Status("Buscando PS5...".to_owned()));
            match remote_play.discover(&address, true, Duration::from_secs(3)) {
                Ok(host) => {
                    let _ = events.send(UiEvent::Log(format!(
                        "PS5 encontrada en {} ({})",
                        host.host, host.state
                    )));
                    let _ = events.send(UiEvent::Host(host));
                    let _ = events.send(UiEvent::Status("PS5 disponible".to_owned()));
                }
                Err(error) => {
                    let _ = events.send(UiEvent::Log(format!("Discovery fallido: {error}")));
                    let _ = events.send(UiEvent::Status("PS5 no encontrada".to_owned()));
                }
            }
        }
        Command::Register {
            host,
            target,
            pin,
            account_id,
        } => {
            let _ = events.send(UiEvent::Status("Registrando PS5...".to_owned()));
            match remote_play.register_ps5(&host, target, pin, 0, &account_id) {
                Ok(credentials) => {
                    let _ = events.send(UiEvent::Credentials(credentials));
                    let _ = events.send(UiEvent::Log("Registro completado".to_owned()));
                    let _ = events.send(UiEvent::Status("PS5 registrada".to_owned()));
                }
                Err(error) => {
                    let _ = events.send(UiEvent::Log(format!("Registro fallido: {error}")));
                    let _ = events.send(UiEvent::Status("Registro fallido".to_owned()));
                }
            }
        }
        Command::Connect { config } => {
            let _ = events.send(UiEvent::Status("Conectando...".to_owned()));
            match remote_play
                .configure_session(&config)
                .and_then(|_| remote_play.start())
            {
                Ok(()) => {
                    *connected = true;
                    let _ = events.send(UiEvent::Log("Sesión Chiaki iniciada".to_owned()));
                    let _ = events.send(UiEvent::Status("Conectado".to_owned()));
                }
                Err(error) => {
                    let _ = events.send(UiEvent::Log(format!("Conexión fallida: {error}")));
                    let _ = events.send(UiEvent::Status("Conexión fallida".to_owned()));
                }
            }
        }
        Command::Disconnect => {
            let _ = remote_play.stop();
            *connected = false;
            let _ = events.send(UiEvent::Status("Desconectado".to_owned()));
        }
    }
}

unsafe fn ns_string(value: &str) -> id {
    NSString::alloc(nil).init_str(value)
}

unsafe fn set_ivar<T: Copy + Encode>(object: &mut Object, name: &str, value: T) {
    object.set_ivar(name, value);
}

unsafe fn gui_state(object: &mut Object) -> &mut GuiState {
    let pointer = *object.get_ivar::<*mut c_void>("state") as *mut GuiState;
    &mut *pointer
}

unsafe fn text_value(field: id) -> String {
    let value: id = msg_send![field, stringValue];
    let utf8: *const c_char = msg_send![value, UTF8String];
    if utf8.is_null() {
        String::new()
    } else {
        CStr::from_ptr(utf8).to_string_lossy().into_owned()
    }
}

unsafe fn set_text(field: id, value: &str) {
    let string = ns_string(value);
    let _: () = msg_send![field, setStringValue: string];
}

unsafe fn add_label(content: id, text: &str, frame: NSRect) -> id {
    let label: id = msg_send![class!(NSTextField), labelWithString: ns_string(text)];
    let _: () = msg_send![label, setFrame: frame];
    let _: () = msg_send![content, addSubview: label];
    label
}

unsafe fn add_field(content: id, frame: NSRect, placeholder: &str, secure: bool) -> id {
    let class_name = if secure {
        "NSSecureTextField"
    } else {
        "NSTextField"
    };
    let class = Class::get(class_name).expect("AppKit text field class");
    let field: id = msg_send![class, alloc];
    let field: id = msg_send![field, initWithFrame: frame];
    let _: () = msg_send![field, setPlaceholderString: ns_string(placeholder)];
    let _: () = msg_send![content, addSubview: field];
    field
}

unsafe fn add_button(content: id, title: &str, frame: NSRect, target: id, action: Sel) -> id {
    let button: id = msg_send![class!(NSButton), buttonWithTitle: ns_string(title) target: target action: action];
    let _: () = msg_send![button, setFrame: frame];
    let _: () = msg_send![content, addSubview: button];
    button
}

fn app_delegate_class() -> &'static Class {
    static CLASS: OnceLock<&'static Class> = OnceLock::new();
    CLASS.get_or_init(|| unsafe {
        let mut declaration = ClassDecl::new("MouseplayAppDelegate", class!(NSObject))
            .expect("unable to declare AppKit delegate");
        declaration.add_ivar::<id>("window");
        declaration.add_ivar::<id>("status");
        declaration.add_ivar::<id>("logs");
        declaration.add_ivar::<id>("host");
        declaration.add_ivar::<id>("pin");
        declaration.add_ivar::<id>("account");
        declaration.add_ivar::<*mut c_void>("state");
        declaration.add_method(
            sel!(applicationDidFinishLaunching:),
            application_did_finish_launching as extern "C" fn(&mut Object, Sel, id),
        );
        declaration.add_method(
            sel!(search:),
            search_action as extern "C" fn(&mut Object, Sel, id),
        );
        declaration.add_method(
            sel!(register:),
            register_action as extern "C" fn(&mut Object, Sel, id),
        );
        declaration.add_method(
            sel!(connect:),
            connect_action as extern "C" fn(&mut Object, Sel, id),
        );
        declaration.add_method(
            sel!(disconnect:),
            disconnect_action as extern "C" fn(&mut Object, Sel, id),
        );
        declaration.add_method(
            sel!(poll:),
            poll_action as extern "C" fn(&mut Object, Sel, id),
        );
        declaration.register()
    })
}

extern "C" fn application_did_finish_launching(this: &mut Object, _: Sel, _: id) {
    unsafe {
        let window = *this.get_ivar::<id>("window");
        let _: () = msg_send![window, makeKeyAndOrderFront: nil];
    }
}

extern "C" fn search_action(this: &mut Object, _: Sel, _: id) {
    unsafe {
        let address = text_value(*this.get_ivar::<id>("host"));
        let address = if address.trim().is_empty() {
            "255.255.255.255".to_owned()
        } else {
            address
        };
        let state = gui_state(this);
        let _ = state.worker.command_tx.send(Command::Discover { address });
    }
}

extern "C" fn register_action(this: &mut Object, _: Sel, _: id) {
    unsafe {
        let host = text_value(*this.get_ivar::<id>("host"));
        let pin_text = text_value(*this.get_ivar::<id>("pin"));
        let account_text = text_value(*this.get_ivar::<id>("account"));
        let Ok(pin) = pin_text.parse::<u32>() else {
            set_text(*this.get_ivar::<id>("status"), "PIN inválido");
            return;
        };
        let Ok(decoded) = STANDARD.decode(account_text.trim()) else {
            set_text(
                *this.get_ivar::<id>("status"),
                "PSN Account-ID debe estar en Base64",
            );
            return;
        };
        let Ok(account_id) = <[u8; 8]>::try_from(decoded.as_slice()) else {
            set_text(
                *this.get_ivar::<id>("status"),
                "PSN Account-ID debe tener 8 bytes",
            );
            return;
        };
        let state = gui_state(this);
        let _ = state.worker.command_tx.send(Command::Register {
            host,
            target: 1_000_100,
            pin,
            account_id,
        });
    }
}

extern "C" fn connect_action(this: &mut Object, _: Sel, _: id) {
    unsafe {
        let host = text_value(*this.get_ivar::<id>("host"));
        let credentials = gui_state(this).credentials.clone();
        if let (Some(credentials), false) = (credentials, host.trim().is_empty()) {
            let config = credentials.into_connection_config(host, true);
            let _ = gui_state(this)
                .worker
                .command_tx
                .send(Command::Connect { config });
        } else {
            set_text(
                *this.get_ivar::<id>("status"),
                "Registra una PS5 y configura su host",
            );
        }
    }
}

extern "C" fn disconnect_action(this: &mut Object, _: Sel, _: id) {
    unsafe {
        let _ = gui_state(this).worker.command_tx.send(Command::Disconnect);
    }
}

extern "C" fn poll_action(this: &mut Object, _: Sel, _: id) {
    unsafe {
        let events: Vec<UiEvent> = gui_state(this).events.try_iter().collect();
        for event in events {
            match event {
                UiEvent::Log(message) => {
                    let logs = *this.get_ivar::<id>("logs");
                    let _: () = msg_send![logs, insertText: ns_string(&format!("> {message}\n"))];
                }
                UiEvent::Status(message) => set_text(*this.get_ivar::<id>("status"), &message),
                UiEvent::Host(host) => set_text(*this.get_ivar::<id>("host"), &host.host),
                UiEvent::Credentials(credentials) => {
                    gui_state(this).credentials = Some(credentials);
                }
            }
        }
    }
}

fn build_window(delegate: id) {
    unsafe {
        let frame = NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(WINDOW_WIDTH, WINDOW_HEIGHT),
        );
        let style = cocoa::appkit::NSWindowStyleMask::NSTitledWindowMask
            | cocoa::appkit::NSWindowStyleMask::NSClosableWindowMask
            | cocoa::appkit::NSWindowStyleMask::NSMiniaturizableWindowMask;
        let window: id = msg_send![class!(NSWindow), alloc];
        let window: id = msg_send![window, initWithContentRect: frame styleMask: style backing: cocoa::appkit::NSBackingStoreType::NSBackingStoreBuffered defer: NO];
        let _: () = msg_send![window, setTitle: ns_string("Mouseplay")];
        let _: () = msg_send![window, center];
        let content: id = msg_send![window, contentView];

        add_label(
            content,
            "MOUSEPLAY",
            NSRect::new(NSPoint::new(28.0, 560.0), NSSize::new(650.0, 30.0)),
        );
        add_label(
            content,
            "Keyboard + Mouse → PS5",
            NSRect::new(NSPoint::new(28.0, 535.0), NSSize::new(650.0, 22.0)),
        );
        add_label(
            content,
            "IP / host para discovery",
            NSRect::new(NSPoint::new(28.0, 485.0), NSSize::new(180.0, 20.0)),
        );
        let host = add_field(
            content,
            NSRect::new(NSPoint::new(210.0, 480.0), NSSize::new(270.0, 28.0)),
            "255.255.255.255 o IP",
            false,
        );
        add_label(
            content,
            "PIN Remote Play",
            NSRect::new(NSPoint::new(28.0, 440.0), NSSize::new(180.0, 20.0)),
        );
        let pin = add_field(
            content,
            NSRect::new(NSPoint::new(210.0, 435.0), NSSize::new(270.0, 28.0)),
            "PIN de 8 dígitos",
            true,
        );
        add_label(
            content,
            "PSN Account-ID (Base64)",
            NSRect::new(NSPoint::new(28.0, 395.0), NSSize::new(180.0, 20.0)),
        );
        let account = add_field(
            content,
            NSRect::new(NSPoint::new(210.0, 390.0), NSSize::new(350.0, 28.0)),
            "8 bytes codificados en Base64",
            false,
        );

        add_button(
            content,
            "Buscar PS5",
            NSRect::new(NSPoint::new(28.0, 335.0), NSSize::new(130.0, 34.0)),
            delegate,
            sel!(search:),
        );
        add_button(
            content,
            "Registrar PS5",
            NSRect::new(NSPoint::new(170.0, 335.0), NSSize::new(140.0, 34.0)),
            delegate,
            sel!(register:),
        );
        add_button(
            content,
            "Conectar",
            NSRect::new(NSPoint::new(322.0, 335.0), NSSize::new(120.0, 34.0)),
            delegate,
            sel!(connect:),
        );
        add_button(
            content,
            "Desconectar",
            NSRect::new(NSPoint::new(452.0, 335.0), NSSize::new(130.0, 34.0)),
            delegate,
            sel!(disconnect:),
        );
        let status = add_label(
            content,
            "Desconectado",
            NSRect::new(NSPoint::new(28.0, 290.0), NSSize::new(650.0, 25.0)),
        );
        add_label(
            content,
            "Controles: teclado + mouse mediante Quartz",
            NSRect::new(NSPoint::new(28.0, 255.0), NSSize::new(650.0, 22.0)),
        );
        add_label(
            content,
            "Estado / Logs",
            NSRect::new(NSPoint::new(28.0, 215.0), NSSize::new(650.0, 22.0)),
        );
        let logs: id = msg_send![class!(NSTextView), alloc];
        let logs: id = msg_send![logs, initWithFrame: NSRect::new(NSPoint::new(28.0, 28.0), NSSize::new(650.0, 180.0))];
        let _: () = msg_send![logs, setEditable: NO];
        let _: () = msg_send![content, addSubview: logs];

        set_ivar(&mut *delegate, "window", window);
        set_ivar(&mut *delegate, "host", host);
        set_ivar(&mut *delegate, "pin", pin);
        set_ivar(&mut *delegate, "account", account);
        set_ivar(&mut *delegate, "status", status);
        set_ivar(&mut *delegate, "logs", logs);
        let _: () = msg_send![window, makeKeyAndOrderFront: nil];

        let timer: id = msg_send![class!(NSTimer), scheduledTimerWithTimeInterval: 0.2f64 target: delegate selector: sel!(poll:) userInfo: nil repeats: YES];
        let _: () = msg_send![timer, retain];
    }
}

pub fn run() -> Result<(), String> {
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .init()
        .map_err(|error| error.to_string())?;
    unsafe {
        let _pool = NSAutoreleasePool::new(nil);
        let application = NSApp();
        application.setActivationPolicy_(
            NSApplicationActivationPolicy::NSApplicationActivationPolicyRegular,
        );
        let (events_tx, events_rx) = mpsc::channel();
        let worker = *start_worker(events_tx);
        let gui_state = Box::new(GuiState {
            worker,
            events: events_rx,
            credentials: None,
        });
        let delegate: id = msg_send![app_delegate_class(), new];
        set_ivar(
            &mut *delegate,
            "state",
            Box::into_raw(gui_state) as *mut c_void,
        );
        build_window(delegate);
        let _: () = msg_send![application, setDelegate: delegate];
        application.activateIgnoringOtherApps_(YES);
        application.run();
    }
    Ok(())
}
