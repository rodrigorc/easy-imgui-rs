use std::time::Duration;

use easy_imgui::{future::FutureHandle, lbl_id};
use easy_imgui_sdl3::Application;
use sdl3::video::{GLProfile, SwapInterval, WindowPos};

fn main() {
    let sdl = sdl3::init().unwrap();
    sdl3::hint::set(sdl3::hint::names::QUIT_ON_LAST_WINDOW_CLOSE, "0");

    let sdl_video = sdl.video().unwrap();
    let sdl_event = sdl.event().unwrap();
    let _sdl_gamepad = sdl.gamepad().unwrap();

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
    unsafe {
        io.inner().ConfigViewportsNoDefaultParent = false;
    }

    app_handler.run(&mut event_pump);
}

struct App {
    local_proxy: easy_imgui_sdl3::LocalProxy<App>,
    counter: i32,
    fut: Option<FutureHandle<()>>,
}

impl Application for App {
    type UserEvent = u32;

    type Data = ();

    fn new(args: easy_imgui_sdl3::Args<'_, Self>) -> Self {
        App {
            counter: 0,
            local_proxy: args.local_proxy(),
            fut: None,
        }
    }
}

impl easy_imgui::UiBuilder for App {
    fn do_ui(&mut self, ui: &easy_imgui::Ui<Self>) {
        ui.show_demo_window(None);
        ui.window_config(lbl_id("SDL3 example", "example"))
            .with(|| {
                if ui.button(lbl_id("Future", "future")) {
                    let local_proxy = self.local_proxy.clone();
                    let fut = self.local_proxy.spawn_idle(async move {
                        println!("start");
                        local_proxy.ping_user_input();
                        for i in 0..10 {
                            let counter = local_proxy
                                .future_back()
                                .run(|app, _args| {
                                    app.counter += 1;
                                    app.counter
                                })
                                .unwrap();
                            println!("do {i} {counter}");

                            smol::Timer::after(Duration::from_millis(250)).await;
                        }
                        println!("end");
                    });
                    if let Some(fut) = self.fut.take() {
                        fut.cancel();
                    }
                    self.fut = Some(fut);
                }
                ui.text(&format!("Counter: {}", self.counter));
            });
    }
}
