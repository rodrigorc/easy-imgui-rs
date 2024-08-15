use easy_imgui_window::{easy_imgui as imgui, winit, AppHandler, Application};
use winit::event_loop::EventLoop;

use easy_imgui_filechooser as filechooser;

fn main() {
    if let Some(locale) = sys_locale::get_locale() {
        filechooser::set_locale(&locale);
    }
    let event_loop = EventLoop::new().unwrap();

    let mut main = AppHandler::<App>::default();
    main.attributes().title = String::from("Example");
    event_loop.run_app(&mut main).unwrap();
}

struct App {
    of_atlas: filechooser::CustomAtlas,
    of: Option<filechooser::FileChooser>,
}

impl Application for App {
    type UserEvent = ();
    type Data = ();

    fn new(args: easy_imgui_window::Args<'_, Self::Data>) -> Self {
        unsafe {
            args.window
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
        App {
            of_atlas: Default::default(),
            of: Some(of),
        }
    }
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
