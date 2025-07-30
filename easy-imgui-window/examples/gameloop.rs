use std::rc::Rc;
use std::sync::Arc;

use easy_imgui_window::{
    EventFlags, MainWindow, MainWindowStatus,
    easy_imgui_renderer::{
        Renderer,
        glow::{self, Context, HasContext},
    },
    glutin::prelude::GlSurface,
    winit,
};
use winit::{
    application::ApplicationHandler,
    event_loop::{ActiveEventLoop, EventLoop},
    window::Window,
};

type GameLoop = game_loop::GameLoop<Game, game_loop::Time, (Arc<Window>, MainWindowStatus)>;

struct Init {
    game_loop: Option<GameLoop>,
}

impl ApplicationHandler for Init {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let wattrs = Window::default_attributes().with_title("game_loop");
        let window = MainWindow::new(event_loop, wattrs).unwrap();
        let gl = Rc::new(window.create_gl_context());
        let (gl_context, surface, window) = unsafe { window.into_pieces() };
        let window = Arc::new(window);

        let mut renderer = Renderer::new(gl.clone()).unwrap();
        renderer.set_background_color(None);
        let scale = window.scale_factor();
        let size = window.inner_size().to_logical::<f32>(scale);
        let size = easy_imgui::Vector2::new(size.width, size.height);
        renderer.set_size(size, scale as f32);

        let window_status = MainWindowStatus::default();

        let game = Game {
            gl: gl.clone(),
            surface,
            gl_context,
            renderer,
            app: App {
                r: 0.1,
                g: 0.1,
                b: 0.1,
                show_demo: true,
            },
        };
        let game_loop = game_loop::GameLoop::new(game, 60, 0.1, (window, window_status));

        self.game_loop = Some(game_loop);
    }
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let Some(game_loop) = self.game_loop.as_mut() else {
            return;
        };
        if game_loop.window.0.id() != window_id {
            return;
        }
        game_handle(game_loop, window_id, &event);
        match event {
            winit::event::WindowEvent::RedrawRequested => {
                if !game_loop.next_frame(game_update, game_render) {
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let Some(game_loop) = self.game_loop.as_mut() else {
            return;
        };
        game_loop.window.0.request_redraw();
    }
}

fn game_update(_g: &mut GameLoop) {}

fn game_render(g: &mut GameLoop) {
    unsafe {
        // GL preparation
        use glutin::context::PossiblyCurrentGlContext;

        // This should be the game scene render
        g.game.gl_context.make_current(&g.game.surface).unwrap();
        g.game
            .gl
            .clear_color(g.game.app.r, g.game.app.g, g.game.app.b, 1.0);
        g.game.gl.clear(glow::COLOR_BUFFER_BIT);

        // And this is the ImGui render, optional
        g.game.renderer.do_frame(&mut g.game.app);

        // GL presentation
        g.window.0.pre_present_notify();
        g.game.surface.swap_buffers(&g.game.gl_context).unwrap();
    }
}

fn game_handle(
    g: &mut GameLoop,
    window_id: winit::window::WindowId,
    event: &winit::event::WindowEvent,
) {
    use winit::{
        event::{ElementState, KeyEvent, WindowEvent},
        keyboard::{KeyCode, PhysicalKey},
    };
    let mut wr =
        easy_imgui_window::MainWindowPieces::new(&g.window.0, &g.game.surface, &g.game.gl_context);
    // game_loop renders in the other callback, not here
    let imgui_wants = easy_imgui_window::window_event(
        &mut wr,
        &mut g.game.renderer,
        &mut g.window.1,
        &mut g.game.app,
        window_id,
        event,
        None,
        EventFlags::DoNotRender,
    );
    if imgui_wants.window_closed {
        g.exit();
        return;
    }
    match event {
        WindowEvent::KeyboardInput {
            event:
                KeyEvent {
                    physical_key: PhysicalKey::Code(code),
                    state: ElementState::Pressed,
                    ..
                },
            ..
        } if !imgui_wants.want_capture_keyboard => match code {
            KeyCode::ArrowLeft => {
                g.game.app.b = (g.game.app.b - 0.1).max(0.0);
            }
            KeyCode::ArrowRight => {
                g.game.app.b = (g.game.app.b + 0.1).min(1.0);
            }
            KeyCode::Escape => {
                g.game.app.show_demo ^= true;
            }
            _ => {}
        },
        WindowEvent::CursorMoved { position, .. } if !imgui_wants.want_capture_mouse => {
            let scale = g.window.0.scale_factor();
            let size = g.window.0.inner_size();
            let size = size.to_logical::<f32>(scale);
            let position = position.to_logical::<f32>(scale);
            g.game.app.r = (position.x / size.width).clamp(0.0, 1.0);
            g.game.app.g = (position.y / size.height).clamp(0.0, 1.0);
        }
        _ => {}
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();

    let mut init = Init { game_loop: None };
    event_loop.run_app(&mut init).unwrap();

    /*
        let window = init.window.unwrap();
        let gl = Rc::new(window.create_gl_context());

        let (gl_context, surface, window) = unsafe { window.into_pieces() };
        let window = Arc::new(window);

        let mut renderer = Renderer::new(gl.clone()).unwrap();
        renderer.set_background_color(None);
        let scale = window.scale_factor();
        let size = window.inner_size().to_logical::<f32>(scale);
        let size = easy_imgui::Vector2::new(size.width, size.height);
        renderer.set_size(size, scale as f32);

        let game = Game {
            gl: gl.clone(),
            surface,
            gl_context,
            renderer,
            app: App {
                r: 0.1,
                g: 0.1,
                b: 0.1,
                show_demo: true,
            },
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
            |g| {
                unsafe {
                    // GL preparation
                    use glutin::context::PossiblyCurrentGlContext;

                    // This should be the game scene render
                    g.game.gl_context.make_current(&g.game.surface).unwrap();
                    g.game
                        .gl
                        .clear_color(g.game.app.r, g.game.app.g, g.game.app.b, 1.0);
                    g.game.gl.clear(glow::COLOR_BUFFER_BIT);

                    // And this is the ImGui render, optional
                    g.game.renderer.do_frame(&mut g.game.app);

                    // GL presentation
                    g.window.pre_present_notify();
                    g.game.surface.swap_buffers(&g.game.gl_context).unwrap();
                }
            },
            //handle
            move |g, ev| {
                use winit::{
                    event::{ElementState, Event, KeyEvent, WindowEvent},
                    keyboard::{KeyCode, PhysicalKey},
                };
                let mut wr = easy_imgui_window::MainWindowPieces::new(
                    &g.window,
                    &g.game.surface,
                    &g.game.gl_context,
                );
                // game_loop renders in the other callback, not here
                let imgui_wants = easy_imgui_window::do_event(
                    &mut wr,
                    &mut g.game.renderer,
                    &mut window_status,
                    &mut g.game.app,
                    ev,
                    EventFlags::DoNotRender,
                );
                if imgui_wants.window_closed {
                    g.exit();
                    return;
                }
                match ev {
                    Event::WindowEvent { window_id, event } if g.window.id() == *window_id => {
                        match event {
                            WindowEvent::KeyboardInput {
                                event:
                                    KeyEvent {
                                        physical_key: PhysicalKey::Code(code),
                                        state: ElementState::Pressed,
                                        ..
                                    },
                                ..
                            } if !imgui_wants.want_capture_keyboard => match code {
                                KeyCode::ArrowLeft => {
                                    g.game.app.b = (g.game.app.b - 0.1).max(0.0);
                                }
                                KeyCode::ArrowRight => {
                                    g.game.app.b = (g.game.app.b + 0.1).min(1.0);
                                }
                                KeyCode::Escape => {
                                    g.game.app.show_demo ^= true;
                                }
                                _ => {}
                            },
                            WindowEvent::CursorMoved { position, .. }
                                if !imgui_wants.want_capture_mouse =>
                            {
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
        )
        .unwrap();
    */
}

struct Game {
    gl: Rc<Context>,
    renderer: Renderer,
    surface: glutin::surface::Surface<glutin::surface::WindowSurface>,
    gl_context: glutin::context::PossiblyCurrentContext,
    app: App,
}

struct App {
    r: f32,
    g: f32,
    b: f32,
    show_demo: bool,
}

impl easy_imgui::UiBuilder for App {
    fn do_ui(&mut self, ui: &easy_imgui::Ui<Self>) {
        use easy_imgui::*;
        ui.set_next_window_pos((10.0, 10.0).into(), Cond::Always, (0.0, 0.0).into());
        ui.window_config(lbl("Instructions"))
            .flags(
                WindowFlags::NoMove
                    | WindowFlags::NoResize
                    | WindowFlags::NoMouseInputs
                    | WindowFlags::NoNav
                    | WindowFlags::NoCollapse,
            )
            .with(|| {
                ui.text("Use left-right arrow keys to change blue channel");
                ui.text("Use the mouse to change red and green channels");
                ui.text("Press ESC to show/hide the demo");
            });

        if self.show_demo {
            ui.show_demo_window(Some(&mut self.show_demo));
        }
    }
}
