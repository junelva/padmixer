mod imp;

use gio::{glib::Value, prelude::ObjectExt};
use gtk::glib;

use crate::BasicControllerState;

glib::wrapper! {
    pub struct FemtoVGArea(ObjectSubclass<imp::FemtoVGArea>)
        @extends gtk::Widget, gtk::GLArea,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for FemtoVGArea {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl FemtoVGArea {
    pub fn update_values(&mut self, bcs: BasicControllerState) {
        self.set_property_from_value("X", &Value::from(bcs.analogs[3].value));
        self.set_property_from_value("Y", &Value::from(bcs.analogs[4].value));
    }
}
