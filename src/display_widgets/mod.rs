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
    pub fn update_values(&mut self, bcs: RwLock<BasicControllerState>) {
        let analogs = bcs.read().unwrap().analogs;
        self.set_property_from_value("X", &Value::from(analogs[3].value));
        self.set_property_from_value("Y", &Value::from(analogs[4].value));
    }
}
