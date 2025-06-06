use easy_imgui_window::{
    MainWindow, MainWindowWithRenderer, easy_imgui as imgui,
    easy_imgui_renderer::Renderer,
    winit::{
        event_loop::{ActiveEventLoop, EventLoop},
        window::Window,
    },
};
use std::rc::Rc;

fn main() {
    let event_loop = EventLoop::new().unwrap();

    let mut main = AppHandler::new();
    event_loop.run_app(&mut main).unwrap();
}

struct AppHandler {
    windows: Vec<MainWindowWithRenderer>,
    app: App,
}

impl AppHandler {
    fn new() -> AppHandler {
        AppHandler {
            windows: Vec::new(),
            app: App,
        }
    }
}

impl winit::application::ApplicationHandler for AppHandler {
    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.windows.clear();
    }
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let wattr_1 = Window::default_attributes().with_title("Example #1");
        let main_window_1 = MainWindow::new(event_loop, wattr_1).unwrap();
        let mut window_1 = MainWindowWithRenderer::new(main_window_1);

        let wattr_2 = Window::default_attributes().with_title("Example #2");
        let main_window_2 = MainWindow::new(event_loop, wattr_2).unwrap();
        // The GL context can be reused, but the imgui context cannot
        let mut renderer_2 = Renderer::new(Rc::clone(window_1.renderer().gl_context())).unwrap();
        renderer_2.set_background_color(Some(imgui::Color::GREEN));
        let window_2 = MainWindowWithRenderer::new_with_renderer(main_window_2, renderer_2);

        self.windows.push(window_1);
        self.windows.push(window_2);
    }
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        for window in &mut self.windows {
            if window.main_window().window().id() != window_id {
                continue;
            }
            let res = window.window_event(
                &mut self.app,
                window_id,
                &event,
                easy_imgui_window::EventFlags::empty(),
            );
            if res.window_closed {
                event_loop.exit();
            }
            break;
        }
    }
    fn new_events(&mut self, _event_loop: &ActiveEventLoop, _cause: winit::event::StartCause) {
        for window in &mut self.windows {
            window.new_events();
        }
    }
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        for window in &mut self.windows {
            window.about_to_wait();
        }
    }
}

struct App;

impl imgui::UiBuilder for App {
    fn do_ui(&mut self, ui: &imgui::Ui<Self>) {
        ui.dock_space_over_viewport(0, ui.get_main_viewport(), imgui::DockNodeFlags::None);
        ui.show_demo_window(None);
    }
}
