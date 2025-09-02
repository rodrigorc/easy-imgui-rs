use easy_imgui_window::{AppHandler, Application, Args, easy_imgui as imgui, winit};
use winit::event_loop::EventLoop;

fn main() {
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Debug)
        .parse_default_env()
        .init();

    let event_loop = EventLoop::with_user_event().build().unwrap();

    let mut main = AppHandler::<App>::new(&event_loop, ());
    main.attributes().title = String::from("Example");

    event_loop.run_app(&mut main).unwrap();
}

struct App;

impl Application for App {
    type UserEvent = ();
    type Data = ();
    fn new(_args: Args<Self>) -> App {
        App
    }
}

impl imgui::UiBuilder for App {
    fn do_ui(&mut self, ui: &imgui::Ui<Self>) {
        ui.show_demo_window(None);
    }
}
