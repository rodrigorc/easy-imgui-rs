use easy_imgui_window::{AppHandler, Application, Args, EventResult, easy_imgui as imgui, winit};
use winit::{event::WindowEvent, event_loop::EventLoop};

fn main() {
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Debug)
        .parse_default_env()
        .init();

    let event_loop = EventLoop::with_user_event().build().unwrap();

    let mut main = AppHandler::<App>::new(&event_loop, ());
    main.imgui_builder().set_docking(true);
    main.attributes().title = String::from("Example");

    event_loop.run_app(&mut main).unwrap();
}

struct App;

static KARLA_TTF: &[u8] = include_bytes!("Karla-Regular.ttf");
static UBUNTU_TTF: &[u8] = include_bytes!("Ubuntu-R.ttf");

impl Application for App {
    type UserEvent = ();
    type Data = ();
    fn new(args: Args<Self>) -> App {
        // Dear ImGui by default uses "imgui.ini", but easy_imgui sets it to None.
        let imgui = args.window.renderer().imgui();
        imgui.set_ini_file_name(Some("imgui.ini"));
        let font_atlas = imgui.io_mut().font_atlas_mut();
        font_atlas.add_font(imgui::FontInfo::default_font());
        font_atlas.add_font(imgui::FontInfo::new(KARLA_TTF).set_name("karla"));
        font_atlas.add_font(imgui::FontInfo::new(UBUNTU_TTF).set_name("ubuntu"));
        //imgui.style_mut().FontSizeBase = 16.0;
        App
    }
    fn window_event(&mut self, args: Args<Self>, _event: WindowEvent, res: EventResult) {
        if res.window_closed {
            args.event_loop.exit();
        }
    }
}

impl imgui::UiBuilder for App {
    fn do_ui(&mut self, ui: &imgui::Ui<Self>) {
        ui.dock_space_over_viewport(0, ui.get_main_viewport(), imgui::DockNodeFlags::None);
        ui.show_demo_window(None);
    }
}
