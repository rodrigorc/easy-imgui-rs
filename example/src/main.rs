#![allow(unused_variables, unused_mut)]

use std::rc::Rc;
use cstr::cstr;
use winit::event_loop::EventLoopBuilder;
use glutin::display::{GetGlDisplay, GlDisplay};
use glow::HasContext;
use dear_imgui as imgui;
use imgui::{FontId, UiBuilder};
use dear_imgui_renderer::{window::{MainWindow, MainWindowWithRenderer}, renderer::{Renderer, Application}};

static KARLA_TTF: &[u8] = include_bytes!("Karla-Regular.ttf");
static UBUNTU_TTF: &[u8] = include_bytes!("Ubuntu-R.ttf");

fn main() {
    let event_loop = EventLoopBuilder::new().build();
    let mut window = MainWindow::new(&event_loop).unwrap();

    let dsp = window.gl_context().display();
    let gl = unsafe { glow::Context::from_loader_function_cstr(|s| dsp.get_proc_address(s)) };
    let gl = Rc::new(gl);

    let mut renderer = Renderer::new(gl.clone()).unwrap();

    let imgui = renderer.imgui();
    let f1 = imgui.add_font(imgui::FontInfo::new(KARLA_TTF, 18.0));
    imgui.merge_font(imgui::FontInfo::new(UBUNTU_TTF, 18.0).char_range(0x20ac, 0x20ac));
    let f2 = imgui.add_font(imgui::FontInfo::new(KARLA_TTF, 36.0));

    let my = MyData {
        gl,
        _f1: f1,
        f2,
        z: 0,
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
    _f1: FontId,
    f2: FontId,
    z: i32,
}

impl UiBuilder for MyData {
    type Data = i32;
    fn do_ui<'s>(&'s mut self, ui: &mut imgui::Ui<Self::Data>) {
        let mut y = 0;
        {
            *ui.data() += 1;
            self.z += 1;
            ui.set_next_window_size_constraints_callback(&[20.0, 20.0].into(), &[520.0, 520.0].into(), |_data, mut d| {
                let mut sz = d.desired_size();
                sz.x = (sz.x / 100.0).round() * 100.0;
                sz.y = (sz.y / 100.0).round() * 100.0;
                d.set_desired_size(&sz);
                //self.z += 1;
                //y += 1;
                //let _ = *x;
                unsafe { X += 1 };
                *_data += 1;
            });
            //println!("<<<<<<<<< {X}");
        }
        ui.set_next_window_size(&[100.0, 100.0].into(), imgui::Cond::ImGuiCond_Once);
        ui.set_next_window_pos(&[0.0, 0.0].into(), imgui::Cond::ImGuiCond_Once, &[0.0, 0.0].into());
        ui.with_window(cstr!("Yo"), Some(&mut true), 0, |ui| {
            ui.with_child("T", &[0.0, 0.0].into(), true, 0, |ui| {
                ui.window_draw_list().add_callback({
                    let gl = self.gl.clone();
                    move |data| {
                        //println!("callback!");
                        //let _ = *x;
                        //y += 1;
                        *data += 1;
                        unsafe {
                            gl.clear_color(1.0, 1.0, 0.0, 1.0);
                            gl.clear(glow::COLOR_BUFFER_BIT);
                        }
                    }
                });
                ui.text_unformatted("Test #1");
                ui.with_font(self.f2, |ui| {
                    ui.text_unformatted("Test #2");
                });
                ui.foreground_draw_list().add_circle(&[50.0, 50.0].into(), 25.0, [0xff, 0xff, 0, 0xff], 32, 2.0);
                ui.background_draw_list().add_circle(&[150.0, 150.0].into(), 25.0, [0xff, 0, 0, 0xff], 32, 2.0);
            });
        });
        ui.show_demo_window(&mut true);
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

