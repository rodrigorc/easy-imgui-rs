use easy_imgui_window::{MainWindow, MainWindowWithRenderer,
    winit::event_loop::EventLoopBuilder,
    easy_imgui as imgui,
};

fn main() {
    let event_loop = EventLoopBuilder::new().build().unwrap();
    let main_window = MainWindow::new(&event_loop, "Example").unwrap();
    let mut window = MainWindowWithRenderer::new(main_window);

    let mut app = App;

    event_loop.run(move |event, w| {
        let res = window.do_event(&mut app, &event, w);
        if res.is_break() {
            w.exit();
        }
    }).unwrap();
}

struct App;

impl imgui::UiBuilder for App {
    fn do_ui(&mut self, ui: &imgui::Ui<Self>) {
        ui.show_demo_window(None);
    }
}
