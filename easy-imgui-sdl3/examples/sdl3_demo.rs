use easy_imgui_sdl3::Application;
use sdl3::video::{GLProfile, SwapInterval, WindowPos};

fn main() {
    let sdl = sdl3::init().unwrap();

    let sdl_video = sdl.video().unwrap();
    let sdl_event = sdl.event().unwrap();

    let gla = sdl_video.gl_attr();
    gla.set_context_version(3, 2);
    gla.set_context_profile(GLProfile::Core);
    gla.set_depth_size(0);
    let main_scale = sdl_video
        .get_primary_display()
        .unwrap()
        .get_content_scale()
        .unwrap();

    let mut window = sdl_video
        .window(
            "easy-imgui-sdl3 demo",
            (800.0 * main_scale) as u32,
            (600.0 * main_scale) as u32,
        )
        .opengl()
        .resizable()
        .hidden()
        .high_pixel_density()
        .build()
        .unwrap();
    let sdl_gl = window.gl_create_context().unwrap();
    window.gl_make_current(&sdl_gl).unwrap();
    let _ = sdl_video.gl_set_swap_interval(SwapInterval::VSync);
    window.set_position(WindowPos::Centered, WindowPos::Centered);
    window.show();

    let mut event_pump = sdl.event_pump().unwrap();
    let mut app_handler = easy_imgui_sdl3::AppHandler::<App>::new(
        &easy_imgui::ContextBuilder::new(),
        &sdl_event,
        window,
        sdl_gl,
        (),
    );

    let io = app_handler.imgui_mut().io_mut();
    io.enable_docking(true);
    io.enable_viewports(true);
    app_handler.run(&mut event_pump);
}

struct App;

impl Application for App {
    type UserEvent = ();
    type Data = ();

    fn new(_args: easy_imgui_sdl3::Args<'_, Self>) -> Self {
        App
    }
}

impl easy_imgui::UiBuilder for App {
    fn do_ui(&mut self, ui: &easy_imgui::Ui<Self>) {
        ui.show_demo_window(None);
    }
}
