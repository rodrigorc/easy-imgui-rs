use easy_imgui_window::{
    easy_imgui as imgui, winit::event_loop::EventLoopBuilder, MainWindow, MainWindowWithRenderer,
};

use easy_imgui_filechooser as filechooser;

fn main() {
    let event_loop = EventLoopBuilder::new().build().unwrap();
    let main_window = MainWindow::new(&event_loop, "Example").unwrap();
    let mut window = MainWindowWithRenderer::new(main_window);

    unsafe {
        window
            .renderer()
            .imgui()
            .set_current()
            .set_allow_user_scaling(true);
    }

    let mut of = filechooser::FileChooser::new();
    of.add_flags(filechooser::Flags::SHOW_READ_ONLY);
    of.add_filter(filechooser::Filter {
        id: filechooser::FilterId(0),
        text: "Text".to_string(),
        globs: vec![glob::Pattern::new("*.txt").unwrap()],
    });
    of.add_filter(filechooser::Filter {
        id: filechooser::FilterId(1),
        text: "Image".to_string(),
        globs: vec![
            glob::Pattern::new("*.png").unwrap(),
            glob::Pattern::new("*.jpg").unwrap(),
        ],
    });
    of.add_filter(filechooser::Filter {
        id: filechooser::FilterId(2),
        text: "All".to_string(),
        globs: vec![],
    });
    let mut app = App {
        of_atlas: Default::default(),
        of: Some(of),
    };

    event_loop
        .run(move |event, w| {
            let res = window.do_event(&mut app, &event);
            if res.window_closed {
                w.exit();
            }
        })
        .unwrap();
}

struct App {
    of_atlas: filechooser::CustomAtlas,
    of: Option<filechooser::FileChooser>,
}

impl imgui::UiBuilder for App {
    fn build_custom_atlas(&mut self, atlas: &mut easy_imgui::FontAtlasMut<'_, Self>) {
        self.of_atlas = filechooser::build_custom_atlas(atlas);
    }
    fn do_ui(&mut self, ui: &imgui::Ui<Self>) {
        if ui.shortcut_ex(imgui::Key::F5, imgui::InputFlags::RouteGlobal) {
            if self.of.is_none() {
                let of = filechooser::FileChooser::new();
                self.of = Some(of);
            } else {
                self.of = None;
            }
        }

        if let Some(of) = &mut self.of {
            let res = of.do_ui(ui, &self.of_atlas);
            match res {
                filechooser::Output::Continue => {}
                filechooser::Output::Cancel => {
                    self.of = None;
                }
                filechooser::Output::Ok => {
                    let ext = match of.active_filter() {
                        Some(filechooser::FilterId(0)) => Some("txt"),
                        Some(filechooser::FilterId(1)) => Some("png"),
                        _ => None,
                    };
                    let path = of.full_path(ext);
                    dbg!(&self.of, path);
                    self.of = None;
                }
            }
        }
    }
}
