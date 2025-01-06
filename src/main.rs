use std::borrow::BorrowMut;
use std::ptr;
use std::sync::{Arc, Mutex, OnceLock, RwLock};

use gtk::gdk::Display;
use gtk::glib::subclass::Signal;
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

mod types;
use types::ValueStore;

const APP_ID: &str = "bug.junelva.padmixer";
static RES: Dir = include_dir!("$CARGO_MANIFEST_DIR/res");

static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();

fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| Runtime::new().expect("tokio runtime new"))
}

fn main() -> glib::ExitCode {
    // platform-specific injections of libepoxy which binds the glarea for rendering
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

    // prepare data sending
    let bcs = BasicControllerState::default();
    let bcs = RwLock::new(bcs);

    let mut store = ValueStore::new();
    let radial_x = store.insert("radial_x", 50.0);
    let radial_y = store.insert("radial_y", 0.0);
    let arc_store = Arc::new(Mutex::new(store));

    // prepare virtual keyboard (prototype style)
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

    // personal logic loop that waits for pad input
    let mut runtime_store_binding = arc_store.clone();
    // let runtime_store = *runtime_store_binding.lock().unwrap();
    runtime().spawn(async move {
        println!("spawned input thread...");
        let mut gilrs = GilrsBuilder::new().set_update_state(false).build().unwrap();
        let mut current_gamepad = None;
        loop {
            println!("polling input...");
            while let Some(event) = gilrs.next_event_blocking(None) {
                gilrs.update(&event);
                current_gamepad = Some(event.id);
                let mut bcs = bcs.write().unwrap();
                match event.event {
                    gilrs::EventType::ButtonPressed(button, _code) => {
                        bcs.try_update_button(button_to_bcs(button), 1.0)
                    }
                    gilrs::EventType::ButtonRepeated(button, _code) => {
                        bcs.try_update_button(button_to_bcs(button), 1.0)
                    }
                    gilrs::EventType::ButtonReleased(button, _code) => {
                        bcs.try_update_button(button_to_bcs(button), 0.0)
                    }
                    gilrs::EventType::ButtonChanged(button, value, _code) => {
                        bcs.try_update_button(button_to_bcs(button), value)
                    }
                    gilrs::EventType::AxisChanged(axis, value, _code) => {
                        let store = runtime_store_binding.borrow_mut();
                        let mut store = store.lock().unwrap();
                        if axis == Axis::RightStickX {
                            store.get("radial_x").replace(Box::new(value), &mut store);
                            println!("insert to radial_x in store: {}", value);
                        } else if axis == Axis::RightStickY {
                            store.get("radial_y").replace(Box::new(value), &mut store);
                            println!("insert to radial_y in store: {}", value);
                        }
                        bcs.try_update_analog(axis_to_bcs(axis), value);
                    }
                    gilrs::EventType::Connected => (),
                    gilrs::EventType::Disconnected => (),
                    gilrs::EventType::Dropped => (),
                    gilrs::EventType::ForceFeedbackEffectCompleted => (),
                    _ => (),
                }
            }
            if current_gamepad.is_some() {
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
            // let b = *bcs.read().unwrap();
            // let mut store_borrow = store.borrow_mut();
            // let store = store_borrow.deref_mut();
            // vs.get("x").repl
            // let res = tx.send(b).await;
            // if res.is_err() {};
        }
    });

    // #[derive(Copy, Clone)]
    // struct UIValues {
    //     x: f32,
    //     y: f32,
    // }
    // let rasync =
    //     Arc::<Mutex<Box<UIValues>>>::new(Mutex::new(Box::new(UIValues { x: 0.0, y: 0.0 })));
    // let rv = rasync.clone();
    // runtime().spawn({
    // let rx = rx.clone();
    // println!("spawned ui update thread...");
    // let radial = rasync.lock().unwrap();
    //     async move {
    //         if let Ok(rec) = rx.recv().await {
    //             let mut rv = rv.lock().unwrap();
    //             // let rv = rv.borrow_mut();
    //             rv.x = rec.analogs[3].value;
    //             rv.y = rec.analogs[4].value;
    //             // rv = &mut [rec.analogs[3].value, rec.analogs[4].value];
    //             // rv[0] = rec.analogs[3].value;
    //             // rv[1] = rec.analogs[4].value;
    //         }
    //     }
    // });

    let app = Application::builder().application_id(APP_ID).build();
    app.connect_startup(|_| {
        // load gtk css. using this style to hide window backdrop
        // window {
        //     background-color: rgba(0, 0, 0, 0);
        // }
        let provider = CssProvider::new();
        provider.load_from_string(RES.get_file("style.css").unwrap().contents_utf8().unwrap());
        gtk::style_context_add_provider_for_display(
            &Display::default().expect("display default"),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    });

    // runtime().spawn(async {
    //     read_notify.notified().await;
    //     let store = arc_store.clone();
    //     let store = store.lock().unwrap();
    //     let x_value = radial_x.lock().unwrap();
    //     let x_opt = x_value.load(&store).as_any().downcast_ref::<f32>();
    //     if let Some(new_x) = x_opt {
    //         xbox = Box::new(*new_x);
    //     }

    //     let y_value = radial_y.lock().unwrap();
    //     let y_opt = y_value.load(&store).as_any().downcast_ref::<f32>();
    //     if let Some(new_y) = y_opt {
    //         ybox = Box::new(*new_y);
    //     }
    // });

    app.connect_activate(move |app| {
        // window surface
        let window = gtk::ApplicationWindow::new(app);
        let window_native = window.native().unwrap();
        window.set_title(Some("padmixer (in-development build)"));
        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_size_request(380, 380);
        window.set_margin(Edge::Bottom, 40);
        window.set_margin(Edge::Right, 40);
        let anchors = [
            (Edge::Left, false),
            (Edge::Top, false),
            (Edge::Right, true),
            (Edge::Bottom, true),
        ];
        for (anchor, state) in anchors {
            window.set_anchor(anchor, state);
        }

        // radial.connect_closure(
        //     "update-input-vectors",
        //     false,
        //     closure_local!(move |x: f32, y: f32| {
        //         println!("these values are in the ui activate function. {}, {}", x, y);
        //     }),
        // );
        // radial.set_x(50.0);
        // radial.set_y(0.0);
        // let store = ui_store_binding.lock().unwrap();
        // let x = store
        //     .get("radial_x")
        //     .load(&store)
        //     .as_any()
        //     .downcast_ref::<f32>()
        //     .unwrap();
        // let y = store
        //     .get("radial_y")
        //     .load(&store)
        //     .as_any()
        //     .downcast_ref::<f32>()
        //     .unwrap();
        let radial = RadialMenu::default();
        let rxc = radial_x.clone();
        let ryc = radial_y.clone();
        let store = arc_store.clone();
        radial.add_tick_callback(move |wdg, _clk| {
            // area.queue_render();
            let store = store.lock().unwrap();

            let mut x = 0.0;
            let x_value = rxc.lock().unwrap();
            let x_opt = x_value.load(&store).as_any().downcast_ref::<f32>();
            if let Some(new_x) = x_opt {
                x = *new_x;
            } else {
                println!("x might be nothing");
            }

            let mut y = 0.0;
            let y_value = ryc.lock().unwrap();
            let y_opt = y_value.load(&store).as_any().downcast_ref::<f32>();
            if let Some(new_y) = y_opt {
                y = *new_y;
            } else {
                println!("y might be nothing");
            }

            wdg.set_x(x);
            wdg.set_y(y);

            glib::ControlFlow::Continue
        });
        // radial.add_tick_callback(move || {});

        window.set_child(Some(&radial));
        window.present();

        // now that window is presented, nullify its input region
        let surface = window_native.surface();
        if surface.is_some() {
            let surface = surface.unwrap();
            let input_region = gtk::cairo::Region::create();
            surface.set_input_region(&input_region);
        } else {
            println!("unable to disallow input region due to lack of surface on window");
        }
    });
    app.run()
}

// fn initialize_app(app: &Application) {
//     // window surface
//     let window = gtk::ApplicationWindow::new(app);
//     let window_native = window.native().unwrap();
//     window.set_title(Some("padmixer (in-development build)"));
//     window.init_layer_shell();
//     window.set_layer(Layer::Overlay);
//     window.set_size_request(380, 380);
//     window.set_margin(Edge::Bottom, 40);
//     window.set_margin(Edge::Right, 40);
//     let anchors = [
//         (Edge::Left, false),
//         (Edge::Top, false),
//         (Edge::Right, true),
//         (Edge::Bottom, true),
//     ];
//     for (anchor, state) in anchors {
//         window.set_anchor(anchor, state);
//     }

//     // let sig = Signal::builder("update-widget")
//     //     .param_types([SignalType::from(Type::F32), SignalType::from(Type::F32)])
//     //     .build();

//     let radial = RadialMenu::default();
//     window.set_child(Some(&radial));
//     window.present();

//     let surface = window_native.surface();

//     if surface.is_some() {
//         let surface = surface.unwrap();
//         let input_region = gtk::cairo::Region::create();
//         surface.set_input_region(&input_region);
//     } else {
//         println!("unable to disallow input region due to lack of surface on window");
//     }
// }

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
