#![allow(unused_variables, unused_mut)]

use std::rc::Rc;
use cstr::cstr;
use winit::event_loop::EventLoopBuilder;
use glutin::display::{GetGlDisplay, GlDisplay};
use glow::HasContext;
use dear_imgui as imgui;
use imgui::{FontId, CustomRectIndex, UiBuilder, SelectableFlags, SliderFlags, FontAtlasMut};
use dear_imgui_renderer::{window::{MainWindow, MainWindowWithRenderer}, renderer::{Renderer, Application}};

static KARLA_TTF: &[u8] = include_bytes!("Karla-Regular.ttf");
static UBUNTU_TTF: &[u8] = include_bytes!("Ubuntu-R.ttf");

fn main() {
    let event_loop = EventLoopBuilder::new().build();
    let mut window = MainWindow::new(&event_loop, "Example").unwrap();

    let dsp = window.gl_context().display();
    let gl = unsafe { glow::Context::from_loader_function_cstr(|s| dsp.get_proc_address(s)) };
    let gl = Rc::new(gl);

    let mut renderer = Renderer::new(gl.clone()).unwrap();

    let imgui = renderer.imgui();
    //let f1 = imgui.add_font(imgui::FontInfo::new(KARLA_TTF, 18.0));
    //imgui.merge_font(imgui::FontInfo::new(UBUNTU_TTF, 18.0).char_range(0x20ac, 0x20ac));
    //let f2 = imgui.add_font(imgui::FontInfo::new(KARLA_TTF, 36.0));

    let my = MyData {
        gl,
        f1: FontId::default(),
        f2: FontId::default(),
        rr: CustomRectIndex::default(),
        z: 0,
        sel: 0,
        checked: false,
        drags: [0.0, 0.0, 0.0],
        input: String::with_capacity(10),
        x: 0.0,
    };

    let mut window = MainWindowWithRenderer::new(window, renderer, my);

    let mut x = 0;
    let mut y = 0;
    event_loop.run(move |event, _w, control_flow| {
        x += 1;
        //window.ping_user_input();
        window.do_event_with_data(&event, control_flow, &mut x);
    });
}

static mut X: i32 = 0;

struct MyData {
    gl: Rc<glow::Context>,
    f1: FontId,
    f2: FontId,
    rr: CustomRectIndex,
    z: i32,
    checked: bool,
    sel: usize,
    drags: [f32; 3],
    input: String,
    x: f32,
}

impl UiBuilder for MyData {
    type Data = i32;

    fn do_custom_atlas<'ctx>(&'ctx mut self, atlas: &mut FontAtlasMut<'ctx, '_>) {
        self.f1 = atlas.add_font_collection([
            imgui::FontInfo::new(KARLA_TTF, 18.0),
            imgui::FontInfo::new(UBUNTU_TTF, 18.0).char_range(0x20ac, 0x20ac),
        ]);
        self.f2 = atlas.add_font(imgui::FontInfo::new(KARLA_TTF, 36.0));

        self.rr = atlas.add_custom_rect_regular(42, 42,
            |pixels| {
                for (y, row) in pixels.iter_mut().enumerate() {
                    for (x, color) in row.iter_mut().enumerate() {
                        *color = [
                            (x * y) as u8,
                            (x * x) as u8,
                            (y * y) as u8,
                            0xff,
                        ];
                    }
                }
                dbg!(self.f2);
            });
        atlas.add_custom_rect_font_glyph(self.f1, '💩', 16, 16, 20.0, [2.0, 0.0].into(),
            |pixels| {
                for (y, row) in pixels.iter_mut().enumerate() {
                    for (x, color) in row.iter_mut().enumerate() {
                        *color = [
                            (x * y) as u8,
                            (x * x) as u8,
                            (y * y) as u8,
                            0xff,
                        ];
                    }
                }
            }
        );
        atlas.get_custom_rect(self.rr);
    }

    fn do_ui<'s>(&'s mut self, ui: &mut imgui::Ui<Self::Data>) {
        let mut y = 0;
        {
            *ui.data() += 1;
            self.z += 1;
            ui.set_next_window_size_constraints_callback([20.0, 20.0].into(), [520.0, 520.0].into(), |_data, mut d| {
                let mut sz = d.desired_size();
                sz.x = (sz.x / 100.0).round() * 100.0;
                sz.y = (sz.y / 100.0).round() * 100.0;
                d.set_desired_size(sz);
                //self.z += 1;
                //y += 1;
                //let _ = *x;
                unsafe { X += 1 };
                *_data += 1;
            });
            //println!("<<<<<<<<< {X}");
        }
        ui.set_next_window_size([300.0, 300.0].into(), imgui::Cond::Once);
        ui.set_next_window_pos([0.0, 0.0].into(), imgui::Cond::Once, [0.0, 0.0].into());
        ui.do_window(cstr!("Yo"))
            .open(&mut true)
            .flags(imgui::WindowFlags::MenuBar)
            .push_for_begin((imgui::StyleVar::WindowPadding, imgui::StyleValue::Vec2([20.0, 20.0].into())))
            .with(|ui: &mut imgui::Ui<Self::Data>| {

                ui.with_menu_bar(|ui| {
                    ui.do_menu("File").with(|ui| {
                        if ui.do_menu_item("Exit").shortcut("Ctrl-X").build() {
                            let st = ui.style();
                            println!("{:#?}", st);
                        }
                    });
                });
                ui.do_child("T").border(true).with(|ui| {
                    ui.window_draw_list().add_callback({
                        let gl = self.gl.clone();
                        move |data| {
                            //println!("callback!");
                            //let _ = *x;
                            //y += 1;
                            *data += 1;
                            unsafe {
                                gl.clear_color(0.2, 0.2, 0.0, 1.0);
                                gl.clear(glow::COLOR_BUFFER_BIT);
                            }
                        }
                    });
                    ui.text("Test #1");
                    ui.separator();
                    ui.separator_text("Hala");

                    ui.do_image_button_with_custom_rect("Image", self.rr, 2.0)
                        .build();
                    ui.with_push(
                        (
                            self.f2,
                            [
                                (imgui::ColorId::Text, imgui::Color::from([1.0, 0.0, 0.0, 1.0])),
                                (imgui::ColorId::WindowBg, imgui::Color::from([0.5, 0.0, 0.0, 1.0])),
                            ],
                            [
                                (imgui::StyleVar::Alpha, imgui::StyleValue::F32(0.25)),
                            ],
                        ),
                        |ui| {
                            ui.text("Test #2");
                            ui.with_item_tooltip(|ui| {
                                ui.with_push(self.f1, |ui| {
                                    ui.text("ok...");
                                });
                            })
                        }
                    );
                    ui.checkbox("Click me!", &mut self.checked);
                    ui.do_combo("Combo").preview_value("One").with(|ui| {
                        ui.text("ha");
                        ui.do_selectable("One").flags(SelectableFlags::DontClosePopups).build();
                        ui.do_selectable("Two").build();
                        ui.do_selectable("Three").build();
                    });
                    let mut sel = (self.sel, "");
                    if ui.combo("Other", ["One", "Two", "Three", "Two"].into_iter().enumerate(), |(_, n)| n, &mut sel) {
                        self.sel = sel.0;
                    }
                    ui.do_drag_float_2("Drag x 2##d1", (&mut self.drags[0..2]).try_into().unwrap())
                        .speed(0.01)
                        .range(0.0, 1.0)
                        .flags(SliderFlags::AlwaysClamp)
                        .build();
                    ui.do_input_float("Float", &mut self.x)
                        .step(1.0)
                        .step_fast(10.0)
                        .build();
                    ui.do_input_text_hint("Input", "Aquí", &mut self.input).build();

                    ui.foreground_draw_list().add_circle([50.0, 50.0].into(), 25.0, [1.0, 1.0, 0.0, 1.0].into(), 32, 2.0);
                    ui.background_draw_list().add_circle([150.0, 150.0].into(), 25.0, [1.0, 0.0, 0.0, 1.0].into(), 32, 2.0);
                });
            });
        ui.show_demo_window(None);
        //println!("{}", *ui.data());

        //my_frame(ui, self.f2, &mut self.z);
        self.z += 1;
    }

}

impl Application for MyData {

    fn do_background(&mut self) {
        unsafe {
            self.gl.clear_color(0.45, 0.55, 0.60, 1.0);
            self.gl.clear(glow::COLOR_BUFFER_BIT);
        }
    }

}

