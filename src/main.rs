use std::borrow::BorrowMut;
use std::ptr;
use std::sync::{Arc, Mutex, OnceLock, RwLock};

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

mod types;
use types::{axis_to_bcs, button_to_bcs, BasicControllerState, ValueStore};

const APP_ID: &str = "bug.junelva.padmixer";
static RES: Dir = include_dir!("$CARGO_MANIFEST_DIR/res");

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

    // prepare virtual keyboard (prototype style)
    let mut keyset = AttributeSet::<Key>::new();
    keyset.insert(Key::KEY_1);
    keyset.insert(Key::KEY_2);
    keyset.insert(Key::KEY_H);
    keyset.insert(Key::KEY_M);
    keyset.insert(Key::KEY_N);
    keyset.insert(Key::KEY_ESC);
    keyset.insert(Key::KEY_RIGHTSHIFT);
    let mut keys = [
        ("_", Key::KEY_SPACE, (0.0, 0.0)),
        ("j", Key::KEY_J, (0.0, 0.0)),
        ("k", Key::KEY_K, (0.0, 0.0)),
        ("y", Key::KEY_Y, (0.0, 0.0)),
        ("u", Key::KEY_U, (0.0, 0.0)),
        ("i", Key::KEY_I, (0.0, 0.0)),
        ("o", Key::KEY_O, (0.0, 0.0)),
        ("p", Key::KEY_P, (0.0, 0.0)),
    ];
    let mut keys_string = String::new();
    let len = keys.len() as f32;
    for (i, key) in keys.iter_mut().enumerate() {
        keys_string.push_str(key.0);
        keyset.insert(key.1);
        let theta = std::f64::consts::TAU as f32 * (i as f32 / len);
        let x = f32::cos(theta);
        let y = f32::sin(theta);
        key.2 = (x, y);
    }
    let mut vd = VirtualDeviceBuilder::new()
        .expect("vd new")
        .name("USB-HID Keyboard")
        .with_keys(&keyset)
        .expect("vd with_keys")
        .build()
        .expect("vd build");

    let mut store = ValueStore::new();
    let radial_x = store.insert("radial_x", 0.0);
    let radial_y = store.insert("radial_y", 0.0);
    let arc_store = Arc::new(Mutex::new(store));

    // personal logic loop that waits for pad input
    let mut runtime_store_binding = arc_store.clone();
    runtime().spawn(async move {
        println!("spawned input thread...");
        let mut gilrs = GilrsBuilder::new().set_update_state(false).build().unwrap();
        let mut current_gamepad = None;
        loop {
            // println!("polling input...");
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
                            // println!("insert to radial_x in store: {}", value);
                        } else if axis == Axis::RightStickY {
                            store.get("radial_y").replace(Box::new(value), &mut store);
                            // println!("insert to radial_y in store: {}", value);
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
                // here is the basic prototype of button remapping to keyboard.
                // pad 'x' or 'y' (mappings vary) sends KEY_H.

                let gp = gilrs.gamepad(current_gamepad.unwrap());
                let st = gp.state();

                let but_start = st.button_data(Gamepad::button_code(&gp, Button::Start).unwrap());
                if but_start.is_some() {
                    let but_start = but_start.unwrap();
                    if but_start.is_pressed() {
                        let ie = InputEvent::new(EventType::KEY, Key::KEY_ESC.code(), 1);
                        let res = vd.emit(&[ie]);
                        if res.is_err() {
                            println!("{:?}", res);
                        }
                    } else {
                        let ie = InputEvent::new(EventType::KEY, Key::KEY_ESC.code(), 0);
                        let res = vd.emit(&[ie]);
                        if res.is_err() {
                            println!("{:?}", res);
                        }
                    }
                }

                let but_y = st.button_data(Gamepad::button_code(&gp, Button::North).unwrap());
                if but_y.is_some() {
                    let but_y = but_y.unwrap();
                    if but_y.is_pressed() {
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

                let but_a = st.button_data(Gamepad::button_code(&gp, Button::South).unwrap());
                if but_a.is_some() {
                    let but_a = but_a.unwrap();
                    if but_a.is_pressed() {
                        let ie = InputEvent::new(EventType::KEY, Key::KEY_SPACE.code(), 1);
                        let res = vd.emit(&[ie]);
                        if res.is_err() {
                            println!("{:?}", res);
                        }
                    } else {
                        let ie = InputEvent::new(EventType::KEY, Key::KEY_SPACE.code(), 0);
                        let res = vd.emit(&[ie]);
                        if res.is_err() {
                            println!("{:?}", res);
                        }
                    }
                }

                let but_x = st.button_data(Gamepad::button_code(&gp, Button::West).unwrap());
                if but_x.is_some() {
                    let but_x = but_x.unwrap();
                    if but_x.is_pressed() {
                        let ie = InputEvent::new(EventType::KEY, Key::KEY_1.code(), 1);
                        let res = vd.emit(&[ie]);
                        if res.is_err() {
                            println!("{:?}", res);
                        }
                    } else {
                        let ie = InputEvent::new(EventType::KEY, Key::KEY_1.code(), 0);
                        let res = vd.emit(&[ie]);
                        if res.is_err() {
                            println!("{:?}", res);
                        }
                    }
                }

                // here we do the keys on the radial menu
                let bcs = bcs.read().unwrap();
                let rs_x = bcs
                    .analog_state_by_type(types::CommonAnalog::RightStickX)
                    .value;
                let rs_y = bcs
                    .analog_state_by_type(types::CommonAnalog::RightStickY)
                    .value;

                // let rs_z = bcs
                //     .analog_state_by_type(types::CommonAnalog::RightLever)
                //     .value;
                // if rs_z > 0.9 {
                //     let ie = InputEvent::new(EventType::KEY, Key::KEY_RIGHTCTRL.code(), 0);
                //     let res = vd.emit(&[ie]);
                //     if res.is_err() {
                //         println!("{:?}", res);
                // }
                // if rs_z > 0.4 {
                //     let ie = InputEvent::new(EventType::KEY, Key::KEY_RIGHTSHIFT.code(), 0);
                //     let res = vd.emit(&[ie]);
                //     if res.is_err() {
                //         println!("{:?}", res);
                //     }
                // } else {
                //     let ie = InputEvent::new(EventType::KEY, Key::KEY_RIGHTSHIFT.code(), 1);
                //     let res = vd.emit(&[ie]);
                //     if res.is_err() {
                //         println!("{:?}", res);
                //     }
                // }

                if (f32::abs(rs_x) + f32::abs(rs_y)) > 0.5 {
                    // calculate nearest coordinate in keys mapping
                    // println!("are we pressing a key with the radial menu yet");
                    let mut nearest = Key::KEY_UNKNOWN;
                    let mut nearest_distance = 4.0;
                    for key in keys.iter() {
                        let co = key.2;
                        let distance = f32::abs(f32::sqrt(
                            f32::powf(co.0 - rs_x, 2.0) + f32::powf(co.1 - rs_y, 2.0),
                        ));
                        if distance < nearest_distance {
                            // println!("co {:?} dist {} {}", co, distance, key.0);
                            nearest_distance = distance;
                            nearest = key.1;
                        }
                    }
                    if nearest != Key::KEY_UNKNOWN {
                        // println!("well, it is something");
                        // release every key in the binding
                        for key in keys {
                            let ie = InputEvent::new(EventType::KEY, key.1.code(), 0);
                            let res = vd.emit(&[ie]);
                            if res.is_err() {
                                println!("{:?}", res);
                            }
                        }

                        let ie = InputEvent::new(EventType::KEY, nearest.code(), 1);
                        let res = vd.emit(&[ie]);
                        if res.is_err() {
                            println!("{:?}", res);
                        }
                    }
                } else {
                    for key in keys {
                        let ie = InputEvent::new(EventType::KEY, key.1.code(), 0);
                        let res = vd.emit(&[ie]);
                        if res.is_err() {
                            println!("{:?}", res);
                        }
                    }
                }
            }
        }
    });

    let app = Application::builder().application_id(APP_ID).build();
    app.connect_startup(|_| {
        // load gtk css. using this style to hide window backdrop
        //
        // window {
        //     background-color: rgba(0, 0, 0, 0);
        // }
        //
        let provider = CssProvider::new();
        provider.load_from_string(RES.get_file("style.css").unwrap().contents_utf8().unwrap());
        gtk::style_context_add_provider_for_display(
            &Display::default().expect("display default"),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    });

    app.connect_activate(move |app| {
        // window surface
        let window = gtk::ApplicationWindow::new(app);
        let window_native = window.native().unwrap();
        window.set_title(Some("padmixer (in-development build)"));
        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_size_request(380, 380);
        window.set_margin(Edge::Bottom, 200);
        window.set_margin(Edge::Right, 200);
        let anchors = [
            (Edge::Left, false),
            (Edge::Top, false),
            (Edge::Right, true),
            (Edge::Bottom, true),
        ];
        for (anchor, state) in anchors {
            window.set_anchor(anchor, state);
        }

        let radial = RadialMenu::default();
        radial.set_labels(&*keys_string);
        let rxc = radial_x.clone();
        let ryc = radial_y.clone();
        let store = arc_store.clone();
        radial.add_tick_callback(move |wdg, _clk| {
            // .queue_render() is automatic for GLArea.
            let store = store.lock().unwrap();

            let mut x = 0.0;
            let x_value = rxc.lock().unwrap();
            let x_opt = x_value.load(&store).as_any().downcast_ref::<f32>();
            if let Some(new_x) = x_opt {
                x = *new_x;
            } else {
                // println!("x might be nothing");
            }

            let mut y = 0.0;
            let y_value = ryc.lock().unwrap();
            let y_opt = y_value.load(&store).as_any().downcast_ref::<f32>();
            if let Some(new_y) = y_opt {
                y = *new_y;
            } else {
                // println!("y might be nothing");
            }

            wdg.set_x(x);
            wdg.set_y(y);

            glib::ControlFlow::Continue
        });

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
