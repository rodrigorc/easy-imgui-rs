/*!
 * An example of how to use external events in an `easy-imgui` loop.
 * This program uses the crate `gilrs` to read gamepad/joystick events.
 */
use std::time::Duration;

use easy_imgui::{UiBuilder, WindowFlags, DrawFlags, Cond, Color, vec2, Vector2};
use easy_imgui_window::{
    MainWindow,
    MainWindowWithRenderer,
    winit::{self, event_loop::{EventLoopBuilder, EventLoopProxy}},
};

use anyhow::Result;

fn main() {

    let event_loop = EventLoopBuilder::with_user_event().build().unwrap();

    let proxy = event_loop.create_proxy();
    std::thread::spawn(move || run_input_events(proxy));

    let window = MainWindow::new(&event_loop, "Gamepad").unwrap();
    let mut window = MainWindowWithRenderer::new(window);

    let mut app = MyApp::new();

    event_loop.run(move |event, w| {
        let res = window.do_event(&mut app, &event, w);
        if res.is_break() {
            w.exit();
        }
        if let winit::event::Event::UserEvent(e) = event {
            window.ping_user_input();
            app.update_gamepad(e);
        }
    }).unwrap();
}

#[derive(Debug)]
struct MyGamepadInfo {
    id: gilrs::GamepadId,
    name: String,
}
#[derive(Debug)]
enum MyEvent {
    GamepadConnected(MyGamepadInfo),
    GamepadEvent(gilrs::Event),
}

#[derive(Debug)]
struct MyApp {
    demo: bool,
    connected: Option<MyGamepadInfo>,
    btn: [bool; 8],
    axis: [f32; 4],
}

impl MyApp {
    fn new() -> Self {
        MyApp {
            demo: false,
            connected: None,
            btn: [false; 8],
            axis: [0.0; 4],
        }
    }
    fn b_idx(b: gilrs::Button) -> Option<usize> {
        let r = match b {
            gilrs::Button::South => 0,
            gilrs::Button::East => 1,
            gilrs::Button::North => 2,
            gilrs::Button::West => 3,
            gilrs::Button::DPadDown => 4,
            gilrs::Button::DPadRight => 5,
            gilrs::Button::DPadUp => 6,
            gilrs::Button::DPadLeft => 7,
            _ => return None,
        };
        Some(r)
    }
    fn a_idx(a: gilrs::Axis) -> Option<usize> {
        let r = match a {
            gilrs::Axis::LeftStickX => 0,
            gilrs::Axis::LeftStickY => 1,
            gilrs::Axis::RightStickX => 2,
            gilrs::Axis::RightStickY => 3,
            _ => return None,
        };
        Some(r)
    }
    fn update_gamepad(&mut self, e: MyEvent) {
        match e {
            MyEvent::GamepadConnected(g) => {
                if self.connected.is_none() {
                    self.connected = Some(g);
                }
            }
            MyEvent::GamepadEvent(e) => {
                match e.event {
                    gilrs::EventType::ButtonPressed(b, _) => {
                        if let Some(i) = Self::b_idx(b) {
                            self.btn[i] = true;
                        }
                        if matches!(b, gilrs::Button::Select | gilrs::Button::Mode | gilrs::Button::Start) {
                            self.demo ^= true;
                        }
                    }
                    gilrs::EventType::ButtonReleased(b, _) => {
                        if let Some(i) = Self::b_idx(b) {
                            self.btn[i] = false;
                        }
                    }
                    gilrs::EventType::AxisChanged(a, v, _) => {
                        if let Some(i) = Self::a_idx(a) {
                            self.axis[i] = v;
                        }
                    }
                    gilrs::EventType::Disconnected => {
                        if matches!(&self.connected, Some(t) if t.id == e.id) {
                            self.connected = None;
                            self.btn = [false; 8];
                            self.axis = [0.0; 4];
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

impl UiBuilder for MyApp {
    fn do_ui(&mut self, ui: &easy_imgui::Ui<Self>) {
        if self.demo {
            ui.show_demo_window(Some(&mut self.demo));
        }

        ui.set_next_window_size(vec2(360.0, 300.0), Cond::Always);
        ui.window_config(&format!("Gamepad: {}###gamepad", self.connected.as_ref().map(|info| info.name.as_str()).unwrap_or("disconnected")))
            .flags(WindowFlags::AlwaysAutoResize)
            .with(|| {
                let p0 = ui.get_cursor_screen_pos();
                let sz = ui.get_content_region_avail();
                let p1 = vec2(p0.x + sz.x, p0.y + sz.y);

                let dr = ui.window_draw_list();
                dr.add_rect_filled(p0, p1, Color::new(1.0, 1.0, 1.0, 1.0), 0.0, DrawFlags::None);
                dr.add_rect(p0, p1, Color::new(0.5, 0.5, 0.5, 1.0), 0.0, DrawFlags::None, 4.0);

                static BUTTONS: &[[f32; 2]] = &[
                    [250.0       , 80.0 + 30.0],
                    [250.0 + 30.0, 80.0       ],
                    [250.0       , 80.0 - 30.0],
                    [250.0 - 30.0, 80.0       ],

                    [80.0       , 180.0 + 30.0],
                    [80.0 + 30.0, 180.0       ],
                    [80.0       , 180.0 - 30.0],
                    [80.0 - 30.0, 180.0       ],
                ];
                let color = if self.connected.is_some() {
                    Color::new(0.75, 0.1, 0.2, 1.0)
                } else {
                    Color::new(0.75, 0.75, 0.75, 1.0)
                };
                let draw_btn = |center: Vector2, color, dpad, filled| {
                    if dpad {
                        let tl = vec2(center.x - 10.0, center.y - 10.0);
                        let br = vec2(center.x + 10.0, center.y + 10.0);
                        if filled {
                            dr.add_rect_filled(tl, br, color, 2.0, DrawFlags::RoundCornersAll);
                        } else {
                            dr.add_rect(tl, br, color, 2.0, DrawFlags::RoundCornersAll, 2.0);
                        }
                    } else if filled {
                        dr.add_circle_filled(center, 10.0, color, 0);
                    } else {
                        dr.add_circle(center, 10.0, color, 0, 2.0);
                    }
                };
                for (idx, pos) in BUTTONS.iter().enumerate() {
                    let dpad = idx >= 4;
                    draw_btn(vec2(p0.x + pos[0], p0.y + pos[1]), color, dpad, self.btn[idx]);
                }
                static AXES: &[[f32; 2]] = &[
                    [80.0, 80.0],
                    [250.0, 180.0],
                ];
                const R1: f32 = 40.0;
                const R2: f32 = 15.0;
                for (idx, pos) in AXES.iter().enumerate() {
                    let x = self.axis[2 * idx];
                    let y = self.axis[2 * idx + 1];
                    dr.add_circle_filled(vec2(p0.x + pos[0], p0.y + pos[1]), R1 + R2, Color::new(0.8, 0.8, 0.8, 1.0), 0);
                    dr.add_circle_filled(vec2(p0.x + pos[0] + x * R1, p0.y + pos[1] - y * R1), R2, color, 0);
                }
            });
    }
}

fn run_input_events(proxy: EventLoopProxy<MyEvent>) -> Result<()> {
    let mut gilrs = gilrs::Gilrs::new().unwrap();
    for (id, gamepad) in gilrs.gamepads() {
        let name = gamepad.name();
        println!("{} is {:?}", name, gamepad.power_info());
        proxy.send_event(MyEvent::GamepadConnected(MyGamepadInfo { id, name: name.to_owned() }))?;
    }
    loop {
        while let Some(e) = gilrs.next_event_blocking(Some(Duration::from_secs(3600))) {
            if e.event == gilrs::EventType::Connected {
                let pad = gilrs.gamepad(e.id);
                let name = pad.name();
                proxy.send_event(MyEvent::GamepadConnected(MyGamepadInfo { id: e.id, name: name.to_owned() }))?;
            } else {
                proxy.send_event(MyEvent::GamepadEvent(e))?;
            }
        }
    }
}
