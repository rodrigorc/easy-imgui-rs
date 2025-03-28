use easy_imgui_window::{easy_imgui as imgui, winit, AppHandler, Application, Args, EventResult};
use winit::{event::WindowEvent, event_loop::EventLoop};

fn main() {
    let event_loop = EventLoop::with_user_event().build().unwrap();

    let mut main = AppHandler::<App>::new(&event_loop, ());
    main.attributes().title = String::from("Example");

    event_loop.run_app(&mut main).unwrap();
}

struct App;

impl Application for App {
    type UserEvent = ();
    type Data = ();
    fn new(args: Args<Self>) -> App {
        // Dear ImGui by default uses "imgui.ini", but easy_imgui sets it to None.
        args.window.imgui().set_ini_file_name(Some("imgui.ini"));
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
        #[cfg(feature = "docking")]
        {
            ui.dock_space_over_viewport(0, imgui::DockNodeFlags::None);
        }

        ui.show_demo_window(None);
    }
}
