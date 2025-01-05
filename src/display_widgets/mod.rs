mod imp;

use std::sync::RwLock;

use gtk::gio::{glib::Value, prelude::ObjectExt};
use gtk::glib;

use crate::BasicControllerState;

glib::wrapper! {
    pub struct RadialMenu(ObjectSubclass<imp::RadialMenu>)
        @extends gtk::Widget, gtk::GLArea,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for RadialMenu {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl RadialMenu {
    pub fn update_values(&mut self, px: f32, py: f32) {
        println!("updating properties on the thing...");
        // let analogs = bcs.analogs;
        let px = &Value::from(px);
        let py = &Value::from(py);
        self.set_property_from_value("px", px);
        self.set_property_from_value("py", py);
    }
}
