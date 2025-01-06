pub mod imp;

use gtk::glib;

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
    pub fn set_custom_properties(&mut self, x: f32, y: f32) {
        self.set_x(x);
        self.set_y(y);
    }
}
