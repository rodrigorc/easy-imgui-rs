use easy_imgui_window::{
    easy_imgui as imgui, winit::event_loop::EventLoopBuilder, MainWindow, MainWindowWithRenderer,
};

fn main() {
    let event_loop = EventLoopBuilder::new().build().unwrap();
    let main_window = MainWindow::new(&event_loop, "Example").unwrap();
    let mut window = MainWindowWithRenderer::new(main_window);

    let mut app = App;

    event_loop
        .run(move |event, w| {
            let res = window.do_event(&mut app, &event);
            if res.window_closed {
                w.exit();
            }
        })
        .unwrap();
}

struct App;

impl imgui::UiBuilder for App {
    fn do_ui(&mut self, ui: &imgui::Ui<Self>) {
        #[cfg(feature = "docking")]
        {
            ui.dock_space_over_viewport(imgui::DockNodeFlags::None);
        }

        ui.show_demo_window(None);
    }
}
