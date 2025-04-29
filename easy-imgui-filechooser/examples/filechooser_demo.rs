use easy_imgui::{id, lbl_id, HasImGuiContext};
use easy_imgui_window::{easy_imgui as imgui, winit, AppHandler, Application};
use winit::event_loop::EventLoop;

use easy_imgui_filechooser as filechooser;

fn main() {
    if let Some(locale) = sys_locale::get_locale() {
        filechooser::set_locale(&locale);
    }
    let event_loop = EventLoop::with_user_event().build().unwrap();

    let mut main = AppHandler::<App>::new(&event_loop, ());
    main.attributes().title = String::from("Example");
    event_loop.run_app(&mut main).unwrap();
}

struct App {
    of_atlas: filechooser::CustomAtlas,
    of_wnd: Option<filechooser::FileChooser>,
    of_popup: Option<filechooser::FileChooser>,
}

impl Application for App {
    type UserEvent = ();
    type Data = ();

    fn new(args: easy_imgui_window::Args<'_, Self>) -> Self {
        args.window.renderer().imgui().set_allow_user_scaling(true);
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
            of_wnd: Some(of),
            of_popup: None,
        }
    }
}
impl imgui::UiBuilder for App {
    fn build_custom_atlas(&mut self, atlas: &mut easy_imgui::FontAtlasMut<'_, Self>) {
        self.of_atlas = filechooser::build_custom_atlas(atlas);
    }
    fn do_ui(&mut self, ui: &imgui::Ui<Self>) {
        let mut in_window = ui.shortcut_ex(imgui::Key::F5, imgui::InputFlags::RouteGlobal);
        let mut in_popup = ui.shortcut_ex(imgui::Key::F6, imgui::InputFlags::RouteGlobal);

        ui.window_config(lbl_id("Test", "test")).with(|| {
            if ui.button(lbl_id("Window (F5)", "window")) {
                in_window = true;
            }
            if ui.button(lbl_id("Pop-up (F6)", "popup")) {
                in_popup = true;
            }
        });

        if in_window {
            if self.of_wnd.is_none() {
                let of = filechooser::FileChooser::new();
                self.of_wnd = Some(of);
            } else {
                self.of_wnd = None;
            }
        }
        if in_popup {
            if self.of_popup.is_none() {
                let of = filechooser::FileChooser::new();
                self.of_popup = Some(of);
                ui.open_popup(id("popup_file"));
            } else {
                self.of_popup = None;
            }
        }

        if let Some(of_wnd) = &mut self.of_wnd {
            let mut opened = true;
            let mut closed = false;
            ui.window_config(lbl_id("Select file...", "select_file"))
                .open(&mut opened)
                .with(|| {
                    let res = of_wnd.do_ui(ui, &self.of_atlas);
                    match res {
                        filechooser::Output::Continue => {}
                        filechooser::Output::Cancel => {
                            closed = true;
                        }
                        filechooser::Output::Ok => {
                            let ext = match of_wnd.active_filter() {
                                Some(filechooser::FilterId(0)) => Some("txt"),
                                Some(filechooser::FilterId(1)) => Some("png"),
                                _ => None,
                            };
                            let path = of_wnd.full_path(ext);
                            closed = true;
                            dbg!(of_wnd, path);
                        }
                    }
                });
            if !opened || closed {
                self.of_wnd = None;
            }
        }

        if let Some(of_popup) = &mut self.of_popup {
            let keep_open = ui.popup_config(id("popup_file")).with(|| {
                let res = of_popup.do_ui(ui, &self.of_atlas);
                match res {
                    filechooser::Output::Continue => true,
                    filechooser::Output::Cancel => {
                        ui.close_current_popup();
                        false
                    }
                    filechooser::Output::Ok => {
                        let ext = match of_popup.active_filter() {
                            Some(filechooser::FilterId(0)) => Some("txt"),
                            Some(filechooser::FilterId(1)) => Some("png"),
                            _ => None,
                        };
                        let path = of_popup.full_path(ext);
                        dbg!(of_popup, path);
                        ui.close_current_popup();
                        false
                    }
                }
            });
            if keep_open != Some(true) {
                self.of_popup = None;
            }
        }
    }
}
