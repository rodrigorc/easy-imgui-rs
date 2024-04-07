use easy_imgui_window::{
    easy_imgui as imgui, easy_imgui_renderer::Renderer, winit::event_loop::EventLoopBuilder,
    MainWindow, MainWindowWithRenderer,
};
use std::rc::Rc;

fn main() {
    let event_loop = EventLoopBuilder::new().build().unwrap();

    let main_window = MainWindow::new(&event_loop, "Example #1").unwrap();
    let mut window = MainWindowWithRenderer::new(main_window);

    // The GL context can be reused, but the imgui context cannot
    let main_window_2 = MainWindow::new(&event_loop, "Example #2").unwrap();
    let mut renderer_2 = Renderer::new(Rc::clone(window.renderer().gl_context())).unwrap();
    renderer_2.set_background_color(Some(imgui::Color::GREEN));
    let mut window_2 = MainWindowWithRenderer::new_with_renderer(main_window_2, renderer_2);

    let mut app = App;

    event_loop
        .run(move |event, w| {
            let res_1 = window.do_event(&mut app, &event);
            let res_2 = window_2.do_event(&mut app, &event);
            if res_1.window_closed || res_2.window_closed {
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
