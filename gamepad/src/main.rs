use std::{rc::Rc, time::Duration};

use dear_imgui::{UiBuilder, WindowFlags, DrawFlags, Cond};
use dear_imgui_renderer::{window::{MainWindow, MainWindowWithRenderer}, renderer::{Renderer, Application}, glr::GlContext};
use glow::HasContext;
use glutin::{display::GetGlDisplay, prelude::GlDisplay};
use winit::event_loop::{EventLoopBuilder, EventLoopProxy};
use anyhow::Result;

fn main() {

    let event_loop = EventLoopBuilder::with_user_event().build();

    let proxy = event_loop.create_proxy();
    std::thread::spawn(move || run_input_events(proxy));

    let window = MainWindow::new(&event_loop, "Gamepad").unwrap();

    let dsp = window.gl_context().display();
    let gl = unsafe { glow::Context::from_loader_function_cstr(|s| dsp.get_proc_address(s)) };
    let gl = Rc::new(gl);

    let renderer = Renderer::new(gl.clone()).unwrap();

    let app = MyApp::new(gl);
    let mut window = MainWindowWithRenderer::new(window, renderer, app);

    event_loop.run(move |event, _w, control_flow| {
        match &event {
            winit::event::Event::UserEvent(e) => {
                //println!("{e:?}");
                window.ping_user_input();
                window.application().update_gamepad(e);
            }
            _ => {}
        }
        window.do_event(&event, control_flow);
    });
}

#[derive(Debug)]
struct MyApp {
    gl: GlContext,
    demo: bool,
    connected: bool,
    axis: [f32; 4],
    abtn: [f32; 2],
    btn: [bool; 15],
}

impl MyApp {
    fn new(gl: GlContext) -> Self {
        MyApp {
            gl,
            demo: true,
            connected: true,
            axis: [0.0; 4],
            abtn: [0.0; 2],
            btn: [false; 15],
        }
    }
    fn b_idx(b: gilrs::Button) -> Option<usize> {
        let r = match b {
            gilrs::Button::South => 0,
            gilrs::Button::East => 1,
            gilrs::Button::North => 2,
            gilrs::Button::West => 3,
            gilrs::Button::LeftTrigger => 4,
            gilrs::Button::RightTrigger => 5,
            gilrs::Button::Select => 6,
            gilrs::Button::Start => 7,
            gilrs::Button::Mode => 8,
            gilrs::Button::LeftThumb => 9,
            gilrs::Button::RightThumb => 10,
            gilrs::Button::DPadUp => 11,
            gilrs::Button::DPadDown => 12,
            gilrs::Button::DPadLeft => 13,
            gilrs::Button::DPadRight => 14,
            _ => return None,
        };
        Some(r)
    }
    fn update_gamepad(&mut self, e: &gilrs::Event) {
        match e.event {
            gilrs::EventType::ButtonPressed(b, _) => {
                if let Some(i) = Self::b_idx(b) {
                    self.btn[i] = true;
                }
            }
            gilrs::EventType::ButtonReleased(b, _) => {
                if let Some(i) = Self::b_idx(b) {
                    self.btn[i] = false;
                }
            }
            gilrs::EventType::ButtonRepeated(_, _) => {}
            gilrs::EventType::ButtonChanged(b, v, _) => {
                match b {
                    gilrs::Button::LeftTrigger2 => self.abtn[0] = v,
                    gilrs::Button::RightTrigger2 => self.abtn[1] = v,
                    _ => {}
                }
            }
            gilrs::EventType::AxisChanged(a, v, _) => {
                match a {
                    gilrs::Axis::LeftStickX => self.axis[0] = v,
                    gilrs::Axis::LeftStickY => self.axis[1] = v,
                    gilrs::Axis::RightStickX => self.axis[2] = v,
                    gilrs::Axis::RightStickY => self.axis[3] = v,
                    _ => {}
                }
            }
            gilrs::EventType::Connected => {
                self.connected = true;
            }
            gilrs::EventType::Disconnected => {
                self.connected = false;
            }
            gilrs::EventType::Dropped => {}
        }
        dbg!(self);
    }
}

impl UiBuilder for MyApp {
    type Data = ();

    fn do_ui(&mut self, ui: &mut dear_imgui::Ui<Self::Data>) {
        if self.demo {
            ui.show_demo_window(Some(&mut self.demo));
        }

        ui.set_next_window_size([400.0, 300.0].into(), Cond::Always);
        ui.do_window("Gamepad", None, WindowFlags::AlwaysAutoResize).with(|ui| {
            /*
            ui.checkbox("A", &mut { self.btn[0] });
            ui.checkbox("B", &mut { self.btn[1] });
            ui.checkbox("Y", &mut { self.btn[2] });
            ui.checkbox("X", &mut { self.btn[3] });
            ui.checkbox("LB", &mut { self.btn[4] });
            ui.checkbox("RT", &mut { self.btn[5] });
            ui.do_slider_float("LT2", &mut { self.abtn[0] }).build();
            ui.do_slider_float("RT2", &mut { self.abtn[1] }).build();
 */

            let p0 = ui.get_cursor_screen_pos();
            let sz = ui.get_content_region_avail();
            let p1 = [p0.x + sz.x, p0.y + sz.y].into();

            let mut dr = ui.window_draw_list();
            dr.add_rect_filled(p0, p1, [1.0, 1.0, 1.0, 1.0].into(), 0.0, DrawFlags::None);
            dr.add_rect(p0, p1, [0.5, 0.5, 0.5, 1.0].into(), 0.0, DrawFlags::None, 4.0);

            static BUTTONS: &[[f32; 2]] = &[
                [300.0, 150.0],
                [350.0, 100.0],
                [300.0,  50.0],
                [250.0, 100.0],
            ];
            for (idx, pos) in BUTTONS.iter().enumerate() {
                if self.btn[idx] {
                    dr.add_circle_filled([p0.x + pos[0], p0.y + pos[1]].into(), 20.0, [0.0, 0.0, 0.0, 1.0].into(), 0);
                }
                dr.add_circle([p0.x + pos[0], p0.y + pos[1]].into(), 20.0, [1.0, 0.0, 0.0, 1.0].into(), 0, 4.0);
            }
        });
    }
}

impl Application for MyApp {
    fn do_background(&mut self) {
        unsafe {
            self.gl.clear_color(0.45, 0.55, 0.60, 1.0);
            self.gl.clear(glow::COLOR_BUFFER_BIT);
        }
    }
}

fn run_input_events(proxy: EventLoopProxy<gilrs::Event>) -> Result<()> {
    use gilrs::Gilrs;
    let mut gilrs = Gilrs::new().unwrap();
    for (_id, gamepad) in gilrs.gamepads() {
        println!("{} is {:?}", gamepad.name(), gamepad.power_info());
    }
    loop {
        // Examine new events
        while let Some(e) = gilrs.next_event_blocking(Some(Duration::from_secs(3600))) {
            match e.event {
                gilrs::EventType::ButtonPressed(b, _) => {
                    println!("{b:?}");
                }
                gilrs::EventType::ButtonRepeated(_, _) => {}
                gilrs::EventType::ButtonReleased(_, _) => {}
                gilrs::EventType::ButtonChanged(b, v, _) => {
                    println!("B {b:?} = {v}");
                }
                gilrs::EventType::AxisChanged(a, v, _) => {
                    println!("A {a:?} = {v}");
                }
                gilrs::EventType::Connected => {}
                gilrs::EventType::Disconnected => {}
                gilrs::EventType::Dropped => {}
            }
            proxy.send_event(e)?;
        }
    }
}
