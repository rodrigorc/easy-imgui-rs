use std::rc::Rc;
use std::sync::Arc;

use easy_imgui_window::{
    MainWindow,
    winit::{event_loop::{EventLoopBuilder}},
    easy_imgui_renderer::{Renderer, glow::{self, Context, HasContext}},
    glutin::prelude::GlSurface,
};

fn main() {
    let event_loop = EventLoopBuilder::new().build().unwrap();
    let window = MainWindow::new(&event_loop, "game_loop").unwrap();
    let gl = Rc::new(window.create_gl_context());

    let (window, surface, gl_context) = unsafe { window.into_pieces() };
    let window = Arc::new(window);

    let mut renderer = Renderer::new(gl.clone()).unwrap();
    renderer.set_background_color(None);
    let scale = window.scale_factor();
    let size = window.inner_size().to_logical::<f32>(scale);
    let size = easy_imgui::Vector2::new(size.width, size.height);
    renderer.set_size(size, scale as f32);

    let mut window_status = easy_imgui_window::MainWindowStatus::default();

    let game = Game {
        gl: gl.clone(),
        renderer,
        app: App { r: 0.1, g: 0.1, b: 0.1 },
    };
    game_loop::game_loop(
        event_loop,
        window,
        game,
        60,
        0.1,
        //update
        |_| {},
        //render
        move |g| {
            unsafe {
                g.game.gl.clear_color(g.game.app.r, g.game.app.g, g.game.app.b, 1.0);
                g.game.gl.clear(glow::COLOR_BUFFER_BIT);
                g.game.renderer.do_frame(&mut g.game.app);
                surface.swap_buffers(&gl_context).unwrap();
            }
        },
        //handle
        move |g, ev| {
            let res = easy_imgui_window::do_event(&*g.window, &mut g.game.renderer, &mut window_status, &mut g.game.app, ev);
            let std::ops::ControlFlow::Continue(imgui_wants) = res else {
                g.exit();
                return;
            };
            use winit::{
                keyboard::{PhysicalKey, KeyCode},
                event::{Event, WindowEvent, KeyEvent},
            };
            match ev {
                Event::WindowEvent {
                    window_id,
                    event
                } if g.window.id() == *window_id => {
                    match event {
                        WindowEvent::KeyboardInput {
                            event: KeyEvent {
                                physical_key: PhysicalKey::Code(code),
                                ..
                            },
                            ..
                        } if !imgui_wants.want_capture_keyboard => {
                            match code {
                                KeyCode::ArrowLeft => {
                                    g.game.app.b = (g.game.app.b - 0.1).max(0.0);
                                }
                                KeyCode::ArrowRight => {
                                    g.game.app.b = (g.game.app.b + 0.1).min(1.0);
                                }
                                _ => {}
                            }
                        }
                        WindowEvent::CursorMoved {
                            position,
                            ..
                        } if !imgui_wants.want_capture_mouse => {
                            let scale = g.window.scale_factor();
                            let size = g.window.inner_size();
                            let size = size.to_logical::<f32>(scale);
                            let position = position.to_logical::<f32>(scale);
                            g.game.app.r = (position.x / size.width).clamp(0.0, 1.0);
                            g.game.app.g = (position.y / size.height).clamp(0.0, 1.0);
                        }
                        _ => {}

                    }
                }
                _ => {}
            }
        },
    ).unwrap();
}

struct Game {
    gl: Rc<Context>,
    renderer: Renderer,
    app: App,
}

struct App {
    r: f32,
    g: f32,
    b: f32,
}

impl easy_imgui::UiBuilder for App {
    fn do_ui(&mut self, ui: &easy_imgui::Ui<Self>) {
        use easy_imgui::*;

        ui.set_next_window_pos((10.0, 10.0).into(), Cond::Always, (0.0, 0.0).into());
        ui.window_config("Instructions")
            .flags(WindowFlags::NoMove | WindowFlags::NoResize | WindowFlags::NoMouseInputs)
            .with(|| {
                ui.text("Use left-right arrow keys to change blue channel");
                ui.text("Use the mouse to change red and green channels");
            });

        ui.show_demo_window(None);
    }
}
