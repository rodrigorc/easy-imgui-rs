#![allow(unused_variables, unused_mut)]

use std::rc::Rc;
use cstr::cstr;
use winit::event_loop::EventLoopBuilder;
use glutin::display::{GetGlDisplay, GlDisplay};
use glow::HasContext;
use imgui::{FontId, CustomRectIndex, UiBuilder, SelectableFlags, SliderFlags, FontAtlasMut};
use easy_imgui_window::{MainWindow, MainWindowWithRenderer, easy_imgui as imgui, easy_imgui_renderer::{Renderer, Application}};
use imgui::image::GenericImage;

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

    fn do_custom_atlas<'ctx>(&'ctx mut self, atlas: &mut FontAtlasMut<'ctx, '_>, _: &mut i32) {
        self.f1 = atlas.add_font_collection([
            imgui::FontInfo::new(KARLA_TTF, 18.0),
            imgui::FontInfo::new(UBUNTU_TTF, 18.0).char_range(0x20ac, 0x20ac),
        ]);
        self.f2 = atlas.add_font(imgui::FontInfo::new(KARLA_TTF, 36.0));

        static POO: &[u8] = include_bytes!("poo.png");
        let poo = image::load_from_memory_with_format(POO, image::ImageFormat::Png).unwrap();
        let poo = Rc::new(poo);

        self.rr = atlas.add_custom_rect_regular([poo.width(), poo.height()],
            { let poo = Rc::clone(&poo); move |pixels| pixels.copy_from(&*poo, 0, 0).unwrap() }
        );

        atlas.add_custom_rect_font_glyph(self.f1, '💩', [poo.width(), poo.height()], 20.0, [2.0, 2.0],
            move |pixels| pixels.copy_from(&*poo, 0, 0).unwrap()
        );
        let rr = atlas.get_custom_rect(self.rr);
    }

    fn do_ui(&mut self, ui: &imgui::Ui<Self::Data>, _data: &mut Self::Data) {
        let mut y = 0;
        {
            //*ui.data() += 1;
            self.z += 1;
            ui.set_next_window_size_constraints_callback([20.0, 20.0], [1000.0, 1000.0], |_data, mut d| {
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
        *_data += 1;
        ui.set_next_window_size([300.0, 300.0], imgui::Cond::Once);
        ui.set_next_window_pos([0.0, 0.0], imgui::Cond::Once, [0.0, 0.0]);
        ui.window_config(cstr!("Yo"))
            .open(&mut true)
            .flags(imgui::WindowFlags::MenuBar)
            .push_for_begin((imgui::StyleVar::WindowPadding, imgui::StyleValue::Vec2([20.0, 20.0].into())))
            .with(|| {
                ui.progress_bar_config(self.x / 6.0).overlay("hola").build();
                ui.slider_angle_config("Angle", &mut self.x).display_format(imgui::FloatFormat::F(1)).build();

                let mut s = 1;
                ui.list_box("List", 3, [1, 2, 3, 4, 5, 6], |i| format!("{i}"), &mut s);

                let x = String::from("x");
                ui.tree_node_config(&x).with(|| {});
                ui.tree_node_ex_config(42, &x).with(|| {});

                ui.with_menu_bar(|| {
                    ui.menu_config("File").with(|| {
                        if ui.menu_item_config("Exit").shortcut("Ctrl-X").build() {
                            let st = ui.style();
                            println!("{:#?}", st);
                        }
                    });
                });
                ui.child_config("T").child_flags(imgui::ChildFlags::Border).size([0.0, 300.0]).with(|| {
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

                    ui.image_button_with_custom_rect_config("Image", self.rr, 2.0)
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
                        || {
                            ui.text("Test #2");
                            ui.with_item_tooltip(|| {
                                ui.with_push(self.f1, || {
                                    ui.text("ok...");
                                });
                            })
                        }
                    );
                    ui.checkbox("Click me!", &mut self.checked);
                    ui.popup_context_void_config().with(|| {
                        ui.selectable_config("hala!").build();
                    });
                    ui.combo_config("Combo").preview_value("One").with(|| {
                        ui.text("ha");
                        ui.selectable_config("One").flags(SelectableFlags::DontClosePopups).build();
                        ui.selectable_config("Two").build();
                        ui.selectable_config("Three").build();
                    });
                    let mut sel = (self.sel, "");
                    if ui.combo("Other", ["One", "Two", "Three", "Two"].into_iter().enumerate(), |(_, n)| n, &mut sel) {
                        self.sel = sel.0;
                    }
                    ui.drag_float_2_config("Drag x 2##d1", (&mut self.drags[0..2]).try_into().unwrap())
                        .speed(0.01)
                        .range(0.0, 1.0)
                        .flags(SliderFlags::AlwaysClamp)
                        .build();
                    ui.input_float_config("Float", &mut self.x)
                        .step(1.0)
                        .step_fast(10.0)
                        .build();
                    ui.input_text_hint_config("Input", "Aquí", &mut self.input).build();

                    ui.foreground_draw_list().add_circle([50.0, 50.0], 25.0, [1.0, 1.0, 0.0, 1.0], 32, 2.0);
                    ui.background_draw_list().add_circle([150.0, 150.0], 25.0, [1.0, 0.0, 0.0, 1.0], 32, 2.0);
                });

                ui.table_config("Table 1", 3)
                    .flags(imgui::TableFlags::Borders)
                    .with(|| {
                        ui.table_setup_column("Hello", imgui::TableColumnFlags::None, 0.0, 0);
                        ui.table_setup_column("World", imgui::TableColumnFlags::None, 0.0, 0);
                        ui.table_setup_column("!!!", imgui::TableColumnFlags::None, 0.0, 0);
                        ui.table_headers_row();

                        ui.table_next_row(imgui::TableRowFlags::None, 0.0);

                        ui.table_next_column();
                        ui.text("Uno");
                        ui.table_next_column();
                        ui.text("Dos");
                        ui.table_next_column();
                        ui.text("Tres");

                        ui.table_next_row(imgui::TableRowFlags::None, 0.0);
                        ui.table_next_column();
                        ui.text("1");
                        ui.table_next_column();
                        ui.text("2");
                        ui.table_next_column();

                        ui.table_config("Table 2", 2)
                            .flags(imgui::TableFlags::Borders)
                            .with(|| {
                                ui.table_next_row(imgui::TableRowFlags::Headers, 0.0);

                                ui.table_next_column();
                                ui.text("X");
                                ui.table_next_column();
                                ui.text("Y");
                            });
                    });
                ui.tab_bar_config("Tab Bar")
                    .with(|| {
                        ui.tab_item_config("Tab1")
                            .with(|| {
                                ui.text("hi!");
                            });
                        ui.tab_item_config("Tab2")
                            .with(|| {
                                ui.text("bye!");
                            });
                    });
            });
        ui.show_demo_window(None);
        //println!("{}", *ui.data());

        //my_frame(ui, self.f2, &mut self.z);
        self.z += 1;
    }

}

impl Application for MyData {

    fn do_background(&mut self, _: &mut i32) {
        unsafe {
            self.gl.clear_color(0.45, 0.55, 0.60, 1.0);
            self.gl.clear(glow::COLOR_BUFFER_BIT);
        }
    }

}

