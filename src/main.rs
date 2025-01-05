use std::ptr;
use std::sync::{OnceLock, RwLock};

use gtk::gdk::Display;
use include_dir::{include_dir, Dir};
use tokio::runtime::Runtime;

use evdev::uinput::VirtualDeviceBuilder;
use evdev::{AttributeSet, EventType, InputEvent, Key};
use gilrs::{Axis, Button, Gamepad, GilrsBuilder};

use gtk::gio::{glib, prelude::*};
use gtk::{prelude::*, Application, CssProvider};
use gtk4_layer_shell::{Edge, Layer, LayerShell};

mod display_widgets;
use display_widgets::RadialMenu;

const APP_ID: &str = "bug.junelva.padmixer";
static RES: Dir = include_dir!("$CARGO_MANIFEST_DIR/res");

fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| Runtime::new().expect("Setting up tokio runtime needs to succeed."))
}

fn load_css() {
    // Load the CSS file and add it to the provider
    let provider = CssProvider::new();
    provider.load_from_string(RES.get_file("style.css").unwrap().contents_utf8().unwrap());

    // Add the provider to the default screen
    gtk::style_context_add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn main() -> glib::ExitCode {
    #[cfg(target_os = "macos")]
    let library = unsafe { libloading::os::unix::Library::new("libepoxy.0.dylib") }.unwrap();
    #[cfg(all(unix, not(target_os = "macos")))]
    let library = unsafe { libloading::os::unix::Library::new("libepoxy.so.0") }.unwrap();
    #[cfg(windows)]
    let library = libloading::os::windows::Library::open_already_loaded("libepoxy-0.dll")
        .or_else(|_| libloading::os::windows::Library::open_already_loaded("epoxy-0.dll"))
        .unwrap();

    epoxy::load_with(|name| {
        unsafe { library.get::<_>(name.as_bytes()) }
            .map(|symbol| *symbol)
            .unwrap_or(ptr::null())
    });

    let app = Application::builder().application_id(APP_ID).build();
    app.connect_startup(|_| load_css());
    app.connect_activate(build_ui);

    app.run()
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum CommonAnalog {
    LeftStickX,
    LeftStickY,
    LeftLever,
    RightStickX,
    RightStickY,
    RightLever,
    DPadX,
    DPadY,
    Unknown,
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum CommonButton {
    LeftStickPress,
    RightStickPress,
    LeftShoulder,
    RightShoulder,
    FaceSouth,
    FaceEast,
    FaceWest,
    FaceNorth,
    DPadSouth,
    DPadEast,
    DPadWest,
    DPadNorth,
    Start,
    Select,
    Guide,
    LegacyC,
    LegacyZ,
    LegacyLT,
    LegacyLT2,
    LegacyRT,
    LegacyRT2,
    Unknown,
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
struct StateAnalog {
    ty: CommonAnalog,
    value: f32,
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
struct StateButton {
    ty: CommonButton,
    value: f32,
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
pub struct BasicControllerState {
    analogs: [StateAnalog; 6],
    buttons: [StateButton; 15],
}

impl BasicControllerState {
    fn try_update_button(&mut self, ty: CommonButton, value: f32) {
        for button in self.buttons.iter_mut() {
            if button.ty == ty {
                button.value = value;
            }
        }
    }

    fn try_update_analog(&mut self, ty: CommonAnalog, value: f32) {
        for analog in self.analogs.iter_mut() {
            if analog.ty == ty {
                analog.value = value;
            }
        }
    }
}

impl Default for BasicControllerState {
    fn default() -> Self {
        Self {
            analogs: [
                StateAnalog {
                    ty: CommonAnalog::LeftStickX,
                    value: 0.0,
                },
                StateAnalog {
                    ty: CommonAnalog::LeftStickY,
                    value: 0.0,
                },
                StateAnalog {
                    ty: CommonAnalog::LeftLever,
                    value: 0.0,
                },
                StateAnalog {
                    ty: CommonAnalog::RightStickX,
                    value: 0.0,
                },
                StateAnalog {
                    ty: CommonAnalog::RightStickY,
                    value: 0.0,
                },
                StateAnalog {
                    ty: CommonAnalog::RightLever,
                    value: 0.0,
                },
            ],
            buttons: [
                StateButton {
                    ty: CommonButton::LeftStickPress,
                    value: 0.0,
                },
                StateButton {
                    ty: CommonButton::RightStickPress,
                    value: 0.0,
                },
                StateButton {
                    ty: CommonButton::LeftShoulder,
                    value: 0.0,
                },
                StateButton {
                    ty: CommonButton::RightShoulder,
                    value: 0.0,
                },
                StateButton {
                    ty: CommonButton::FaceSouth,
                    value: 0.0,
                },
                StateButton {
                    ty: CommonButton::FaceEast,
                    value: 0.0,
                },
                StateButton {
                    ty: CommonButton::FaceWest,
                    value: 0.0,
                },
                StateButton {
                    ty: CommonButton::FaceNorth,
                    value: 0.0,
                },
                StateButton {
                    ty: CommonButton::DPadSouth,
                    value: 0.0,
                },
                StateButton {
                    ty: CommonButton::DPadEast,
                    value: 0.0,
                },
                StateButton {
                    ty: CommonButton::DPadWest,
                    value: 0.0,
                },
                StateButton {
                    ty: CommonButton::DPadNorth,
                    value: 0.0,
                },
                StateButton {
                    ty: CommonButton::Start,
                    value: 0.0,
                },
                StateButton {
                    ty: CommonButton::Select,
                    value: 0.0,
                },
                StateButton {
                    ty: CommonButton::Guide,
                    value: 0.0,
                },
            ],
        }
    }
}

fn button_to_bcs(button: Button) -> CommonButton {
    match button {
        Button::South => CommonButton::FaceSouth,
        Button::East => CommonButton::FaceEast,
        Button::North => CommonButton::FaceNorth,
        Button::West => CommonButton::FaceWest,
        Button::Select => CommonButton::Select,
        Button::Start => CommonButton::Start,
        Button::Mode => CommonButton::Guide,
        Button::LeftThumb => CommonButton::LeftStickPress,
        Button::RightThumb => CommonButton::RightStickPress,
        Button::DPadUp => CommonButton::DPadNorth,
        Button::DPadDown => CommonButton::DPadSouth,
        Button::DPadLeft => CommonButton::DPadWest,
        Button::DPadRight => CommonButton::DPadEast,
        Button::C => CommonButton::LegacyC,
        Button::Z => CommonButton::LegacyZ,
        Button::LeftTrigger => CommonButton::LegacyLT,
        Button::LeftTrigger2 => CommonButton::LegacyLT2,
        Button::RightTrigger => CommonButton::LegacyRT,
        Button::RightTrigger2 => CommonButton::LegacyRT2,
        Button::Unknown => CommonButton::Unknown,
    }
}

fn axis_to_bcs(axis: Axis) -> CommonAnalog {
    match axis {
        Axis::LeftStickX => CommonAnalog::LeftStickX,
        Axis::LeftStickY => CommonAnalog::LeftStickY,
        Axis::LeftZ => CommonAnalog::LeftLever,
        Axis::RightStickX => CommonAnalog::RightStickX,
        Axis::RightStickY => CommonAnalog::RightStickY,
        Axis::RightZ => CommonAnalog::RightLever,
        Axis::DPadX => CommonAnalog::DPadX,
        Axis::DPadY => CommonAnalog::DPadY,
        Axis::Unknown => CommonAnalog::Unknown,
    }
}

fn build_ui(app: &Application) {
    // let (sender, receiver) = async_channel::unbounded::<BasicControlState();
    let bcs = BasicControllerState::default();
    let bcs = RwLock::new(bcs);

    let mut keyset = AttributeSet::<Key>::new();
    let keys = [
        ("h", Key::KEY_H),
        ("j", Key::KEY_J),
        ("k", Key::KEY_K),
        ("y", Key::KEY_Y),
        ("u", Key::KEY_U),
        ("i", Key::KEY_I),
        ("o", Key::KEY_O),
        ("p", Key::KEY_P),
    ];
    for key in keys.iter() {
        keyset.insert(key.1);
    }
    let mut vd = VirtualDeviceBuilder::new()
        .expect("vd new")
        .name("USB-HID Keyboard")
        .with_keys(&keyset)
        .expect("vd with_keys")
        .build()
        .expect("vd build");

    println!("hi from input initialization");
    let mut gilrs = GilrsBuilder::new().set_update_state(false).build().unwrap();

    // window surface
    let window = gtk::ApplicationWindow::new(app);
    let window_native = window.native().unwrap();
    {
        let surface = window_native.surface();
        if surface.is_some() {
            let surface = surface.unwrap();
            let input_region = gtk::cairo::Region::create();
            surface.set_input_region(&input_region);
        } else {
            println!("oh no! too early");
        }
    }
    // let input_region = gtk::cairo::Region::create();
    // surface.set_input_region(&input_region);
    window.set_title(Some("BUG // padmixer"));
    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    // window.set_opacity(1.0);
    window.set_size_request(380, 380);
    // window.set_can_target(false);
    window.set_margin(Edge::Bottom, 40);
    window.set_margin(Edge::Right, 40);
    let anchors = [
        (Edge::Left, false),
        (Edge::Right, true),
        (Edge::Top, false),
        (Edge::Bottom, true),
    ];
    for (anchor, state) in anchors {
        window.set_anchor(anchor, state);
    }

    let radial = RadialMenu::default();
    radial.set_size_request(380, 380);
    window.set_child(Some(&radial));
    window.present();

    {
        let surface = window_native.surface();

        if surface.is_some() {
            let surface = surface.unwrap();
            let input_region = gtk::cairo::Region::create();
            surface.set_input_region(&input_region);
        } else {
            println!("oh no! too late");
        }
    }

    runtime().spawn({
        async move {
            let mut current_gamepad = None;
            loop {
                while let Some(event) = gilrs.next_event_blocking(None) {
                    gilrs.update(&event);
                    if current_gamepad.is_none() {
                        let mut bcs = bcs.write().unwrap();
                        match event.event {
                            gilrs::EventType::ButtonPressed(button, _code) => {
                                bcs.try_update_button(button_to_bcs(button), 1.0)
                            }
                            gilrs::EventType::ButtonRepeated(button, _code) => {
                                // TODO: handle button repeated properly
                                bcs.try_update_button(button_to_bcs(button), 1.0)
                            }
                            gilrs::EventType::ButtonReleased(button, _code) => {
                                bcs.try_update_button(button_to_bcs(button), 0.0)
                            }
                            gilrs::EventType::ButtonChanged(button, value, _code) => {
                                bcs.try_update_button(button_to_bcs(button), value)
                            }
                            gilrs::EventType::AxisChanged(axis, value, _code) => {
                                bcs.try_update_analog(axis_to_bcs(axis), value)
                            }
                            gilrs::EventType::Connected => (),
                            gilrs::EventType::Disconnected => (),
                            gilrs::EventType::Dropped => (),
                            gilrs::EventType::ForceFeedbackEffectCompleted => (),
                            _ => todo!(),
                        }
                        println!("connected to pad: {}", gilrs.gamepad(event.id).name());
                        current_gamepad = Some(event.id);
                    }
                }
                if current_gamepad.is_none() {
                } else {
                    let gp = gilrs.gamepad(current_gamepad.unwrap());
                    let st = gp.state();
                    let but_x = st.button_data(Gamepad::button_code(&gp, Button::West).unwrap());
                    if but_x.is_some() {
                        let but_x = but_x.unwrap();
                        if but_x.is_pressed() {
                            let ie = InputEvent::new(EventType::KEY, Key::KEY_H.code(), 1);
                            let res = vd.emit(&[ie]);
                            if res.is_err() {
                                println!("{:?}", res);
                            }
                        } else {
                            let ie = InputEvent::new(EventType::KEY, Key::KEY_H.code(), 0);
                            let res = vd.emit(&[ie]);
                            if res.is_err() {
                                println!("{:?}", res);
                            }
                        }
                    }
                }
            }
        }
    });
}
