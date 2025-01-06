use std::{
    cell::{Cell, RefCell},
    num::NonZeroU32,
    time::Instant,
};

use femtovg::CompositeOperation;
use gtk::glib::{
    subclass::{prelude::*, Signal, SignalType},
    Type,
};
use gtk::{glib, glib::Properties, prelude::*, subclass::prelude::*};

use crate::SIGNALS;

#[derive(Properties)]
#[properties(wrapper_type = super::RadialMenu)]
pub struct RadialMenu {
    canvas: RefCell<Option<femtovg::Canvas<femtovg::renderer::OpenGl>>>,
    start_time: Cell<Instant>,
    #[property(name = "x", set, type = f32)]
    x: RefCell<f32>,
    #[property(name = "y", set, type = f32)]
    y: RefCell<f32>,
}

impl Default for RadialMenu {
    fn default() -> Self {
        Self {
            canvas: Default::default(),
            start_time: Cell::new(Instant::now()),
            x: RefCell::new(0.0),
            y: RefCell::new(0.0),
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for RadialMenu {
    const NAME: &'static str = "RadialMenu";
    type Type = super::RadialMenu;
    type ParentType = gtk::GLArea;
}

#[glib::derived_properties]
impl ObjectImpl for RadialMenu {
    fn signals() -> &'static [Signal] {
        SIGNALS.get_or_init(|| {
            vec![Signal::builder("update-input-vectors")
                .param_types([SignalType::from(Type::F32), SignalType::from(Type::F32)])
                .build()]
        })
    }

    fn constructed(&self) {
        self.parent_constructed();
        let area = self.obj();
        area.set_has_stencil_buffer(true);
        area.add_tick_callback(|area, _| {
            area.queue_render();
            glib::ControlFlow::Continue
        });
    }
}

impl WidgetImpl for RadialMenu {
    fn realize(&self) {
        self.parent_realize();
        self.start_time.set(Instant::now());
    }
    fn unrealize(&self) {
        self.obj().make_current();
        self.canvas.replace(None);
        self.parent_unrealize();
    }
}

impl GLAreaImpl for RadialMenu {
    fn resize(&self, width: i32, height: i32) {
        self.ensure_canvas();
        let mut canvas = self.canvas.borrow_mut();
        let canvas = canvas.as_mut().unwrap();
        canvas.set_size(
            width as u32,
            height as u32,
            self.obj().scale_factor() as f32,
        );
    }
    fn render(&self, _context: &gtk::gdk::GLContext) -> glib::Propagation {
        use femtovg::{Color, Paint, Path};

        self.ensure_canvas();
        let mut canvas = self.canvas.borrow_mut();
        let canvas = canvas.as_mut().unwrap();

        let area = self.obj();
        let w = area.width() as u32;
        let h = area.height() as u32;
        canvas.reset_transform();
        canvas.global_composite_operation(CompositeOperation::Copy);
        canvas.clear_rect(0, 0, w, h, Color::rgba(0, 0, 0, 0));

        canvas.translate(w as f32 / 2., h as f32 / 2.);
        canvas.rotate(self.start_time.get().elapsed().as_secs_f32() * 0.01);

        let mut path = Path::new();
        path.circle(0.0, 0.0, w as f32 * 0.45);
        path.close();
        let mut paint = Paint::color(Color::rgba(128, 128, 225, 128));
        paint.set_line_width(4.);
        canvas.stroke_path(&path, &paint);

        let mut path = Path::new();
        let x = *self.x.borrow();
        path.circle(x, 0.0, 40.0);
        // path.circle(
        //     *self.x.borrow() * (w / 2) as f32,
        //     *self.y.borrow() * (h / 2) as f32,
        //     40.0,
        // );
        path.close();
        let mut paint = Paint::color(Color::rgba(178, 0, 225, 200));
        paint.set_line_width(2.);
        canvas.stroke_path(&path, &paint);

        // let r = (n - DEFAULT_N as f32) / (MAX_N - DEFAULT_N) as f32;
        // canvas.fill_path(&path, &Paint::color(Color::rgba(0, 255, 0, 128)));
        canvas.flush();

        glib::Propagation::Stop
    }
}

impl RadialMenu {
    fn ensure_canvas(&self) {
        use femtovg::{renderer, Canvas};
        use glow::HasContext;

        if self.canvas.borrow().is_some() {
            return;
        }
        let widget = self.obj();
        widget.attach_buffers();

        static LOAD_FN: fn(&str) -> *const std::ffi::c_void =
            |s| epoxy::get_proc_addr(s) as *const _;
        // SAFETY: Need to get the framebuffer id that gtk expects us to draw into, so
        // femtovg knows which framebuffer to bind. This is safe as long as we
        // call attach_buffers beforehand. Also unbind it here just in case,
        // since this can be called outside render.
        let (mut renderer, fbo) = unsafe {
            let renderer =
                renderer::OpenGl::new_from_function(LOAD_FN).expect("Cannot create renderer");
            let ctx = glow::Context::from_loader_function(LOAD_FN);
            let id = NonZeroU32::new(ctx.get_parameter_i32(glow::DRAW_FRAMEBUFFER_BINDING) as u32)
                .expect("No GTK provided framebuffer binding");
            ctx.bind_framebuffer(glow::FRAMEBUFFER, None);
            (renderer, glow::NativeFramebuffer(id))
        };
        renderer.set_screen_target(Some(fbo));
        let canvas = Canvas::new(renderer).expect("Cannot create canvas");
        self.canvas.replace(Some(canvas));
    }

    pub fn update_values(&mut self, x: f32, y: f32) {
        println!("updating properties on the thing...");
        // let analogs = bcs.analogs;
        self.x = RefCell::new(x);
        self.y = RefCell::new(y);
    }
}
