use std::{
    num::NonZeroU32,
    time::{Duration, Instant},
};

use easy_imgui_window::{
    easy_imgui as imgui, winit::event_loop::EventLoopBuilder, EventFlags, MainWindow,
    MainWindowWithRenderer,
};

use easy_imgui_renderer::{
    glow,
    glr::{self, GlContext, UniformField},
};
use glutin::surface::GlSurface;
use imgui::cgmath::SquareMatrix;
const VSH: &str = r"#version 140
uniform mat3 m;
in vec2 pos;
void main(void) {
    gl_Position = vec4(m * vec3(pos, 1.0), 1.0);
}
";
const FSH: &str = r"#version 140
out vec4 out_frag_color;
void main(void) {
    out_frag_color = vec4(1.0, 1.0, 1.0, 1.0);
}
";

type Matrix3 = imgui::cgmath::Matrix3<f32>;
type Vector2 = imgui::cgmath::Vector2<f32>;

easy_imgui_renderer::uniform! {
    struct Uniform {
        m: Matrix3,
    }
}

easy_imgui_renderer::attrib! {
    #[derive(Copy, Clone)]
    struct Vertex {
        pos: Vector2,
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum BallPhase {
    Running,
    Goal(u32), // ticks to count the goal
}

struct Pong {
    input: [bool; UserInput::COUNT as usize],
    pos1: f32,
    pos2: f32,
    score1: u32,
    score2: u32,
    ball: Ball,
}

#[derive(PartialEq, Eq, Copy, Clone)]
enum Menu {
    None,
    Hello,
    Main,
    Options,
}

bitflags::bitflags! {
    #[derive(Default, Copy, Clone)]
    struct UiRequest: u32 {
        const Quit = 1;
        const Fullscreen = 2;
        const VSync = 4;
        const ShowCursor = 8;
        const HideCursor = 0x10;
    }
}

struct App {
    gl: GlContext,
    vao: glr::VertexArray,
    prg: glr::Program,
    u: Uniform,
    window_size: winit::dpi::PhysicalSize<u32>,
    ds: glr::DynamicVertexArray<Vertex>,

    font_normal: imgui::FontId,
    font_medium: imgui::FontId,
    font_big: imgui::FontId,
    last_tick: Instant,

    show_menu: Menu,
    show_demo: bool,
    ui_request: UiRequest,
    ui_status: UiRequest,
    pong: Pong,
}

const BAT_HEIGHT: f32 = 75.0;
const BAT_HEIGHT_EXT: f32 = 100.0;
const BAT_HEIGHT_CENTER: f32 = 50.0;

struct Ball {
    phase: BallPhase,
    pos: Vector2,
    vel: Vector2,
}

impl Ball {
    fn new(p1: bool, pos_y: f32) -> Ball {
        let (pos, vel);
        if p1 {
            pos = Vector2::new(50.0, pos_y);
            vel = Vector2::new(5.0, 3.0);
        } else {
            pos = Vector2::new(800.0 - 50.0, pos_y);
            vel = Vector2::new(-5.0, 3.0);
        }
        Ball {
            phase: BallPhase::Running,
            pos,
            vel,
        }
    }

    fn check_goal(&mut self, dy: f32) {
        let da = dy.abs();
        if da > BAT_HEIGHT_EXT / 2.0 {
            self.phase = BallPhase::Goal(30);
            return;
        }
        self.vel.x *= -1.05;

        if da > BAT_HEIGHT_CENTER / 2.0 {
            let ds = dy.signum();
            self.vel.y += ds * 0.5;
            if da > BAT_HEIGHT / 2.0 {
                self.vel.y += dy.signum() * 2.0;
            }
            self.vel.y = self.vel.y.clamp(-10.0, 10.0);
        }
    }
}

// (0,0) is in the bottom-left,
// x positive to the right
// y positive to the top
fn ortho2d_zero(width: f32, height: f32) -> Matrix3 {
    Matrix3::new(
        2.0 / width,
        0.0,
        0.0,
        0.0,
        -2.0 / height,
        0.0,
        -1.0,
        1.0,
        1.0,
    )
}

fn ratio_ortho(width: f32, height: f32) -> (Matrix3, Vector2, f32) {
    let (x, y, r);
    if width / height > 800.0 / 600.0 {
        r = 600.0 / height;
        x = r * width - 800.0;
        y = 0.0;
    } else {
        r = 800.0 / width;
        x = 0.0;
        y = r * height - 600.0;
    }
    let m = ortho2d_zero(800.0 + x, 600.0 + y);
    let d = Vector2::new((x / 2.0).round(), (y / 2.0).round());
    let t = Matrix3::from_translation(d);
    (m * t, d, r)
}

impl App {
    fn new(gl: &GlContext) -> App {
        let vao = glr::VertexArray::generate(gl).unwrap();
        let prg = glr::Program::from_source(gl, VSH, FSH, None).unwrap();
        let u = Uniform {
            m: ortho2d_zero(1.0, 1.0),
        };
        App {
            vao,
            gl: gl.clone(),
            prg,
            u,
            window_size: (800, 600).into(),
            ds: glr::DynamicVertexArray::new(gl).unwrap(),
            font_normal: imgui::FontId::default(),
            font_medium: imgui::FontId::default(),
            font_big: imgui::FontId::default(),
            last_tick: Instant::now(),
            show_menu: Menu::Hello,
            show_demo: false,
            ui_request: UiRequest::empty(),
            ui_status: UiRequest::empty(),
            pong: Pong::default(),
        }
    }
    fn set_show_menu(&mut self, m: Menu) {
        self.show_menu = m;
        self.last_tick = Instant::now();
        self.ui_request.insert(if m == Menu::None {
            UiRequest::HideCursor
        } else {
            UiRequest::ShowCursor
        });
    }
}

fn add_box(vs: &mut Vec<Vertex>, x: f32, y: f32, w: f32, h: f32) {
    let p1 = Vector2::new(x - w / 2.0, y - h / 2.0);
    let p2 = Vector2::new(x + w / 2.0, y + h / 2.0);
    vs.push(Vertex {
        pos: Vector2::new(p1.x, p1.y),
    });
    vs.push(Vertex {
        pos: Vector2::new(p2.x, p1.y),
    });
    vs.push(Vertex {
        pos: Vector2::new(p1.x, p2.y),
    });

    vs.push(Vertex {
        pos: Vector2::new(p2.x, p1.y),
    });
    vs.push(Vertex {
        pos: Vector2::new(p2.x, p2.y),
    });
    vs.push(Vertex {
        pos: Vector2::new(p1.x, p2.y),
    });
}

/*
   XX XX XX   .. .. XX   XX XX XX   XX XX XX   XX .. ..   XX XX XX   XX .. ..   XX XX XX   XX XX XX   XX XX XX
   XX .. XX   .. .. XX   .. .. XX   .. .. XX   XX .. XX   XX .. ..   XX .. ..   .. .. XX   XX .. XX   XX .. XX
   XX .. XX   .. .. XX   XX XX XX   XX XX XX   XX XX XX   XX XX XX   XX XX XX   .. XX ..   XX XX XX   XX XX XX
   XX .. XX   .. .. XX   XX .. ..   .. .. XX   .. .. XX   .. .. XX   XX .. XX   .. XX ..   XX .. XX   .. .. XX
   XX XX XX   .. .. XX   XX XX XX   XX XX XX   .. .. XX   XX XX XX   XX XX XX   .. XX ..   XX XX XX   .. .. XX
*/
const DIGIT: [[u8; 15]; 10] = [
    [1, 1, 1, 1, 0, 1, 1, 0, 1, 1, 0, 1, 1, 1, 1], //0
    [0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 0, 1], //1
    [1, 1, 1, 0, 0, 1, 1, 1, 1, 1, 0, 0, 1, 1, 1], //2
    [1, 1, 1, 0, 0, 1, 1, 1, 1, 0, 0, 1, 1, 1, 1], //3
    [1, 0, 0, 1, 0, 1, 1, 1, 1, 0, 0, 1, 0, 0, 1], //4
    [1, 1, 1, 1, 0, 0, 1, 1, 1, 0, 0, 1, 1, 1, 1], //5
    [1, 0, 0, 1, 0, 0, 1, 1, 1, 1, 0, 1, 1, 1, 1], //6
    [1, 1, 1, 0, 0, 1, 0, 1, 0, 0, 1, 0, 0, 1, 0], //7
    [1, 1, 1, 1, 0, 1, 1, 1, 1, 1, 0, 1, 1, 1, 1], //8
    [1, 1, 1, 1, 0, 1, 1, 1, 1, 0, 0, 1, 0, 0, 1], //9
];
const DIGIT_SIZE: f32 = 10.0;

fn add_digit(vs: &mut Vec<Vertex>, x: f32, y: f32, n: u32) {
    let x = x - DIGIT_SIZE;
    let digit = &DIGIT[n as usize];
    for iy in 0..5 {
        for ix in 0..3 {
            if digit[ix + 3 * iy] != 0 {
                add_box(
                    vs,
                    x + DIGIT_SIZE * ix as f32,
                    y + DIGIT_SIZE * iy as f32,
                    DIGIT_SIZE,
                    DIGIT_SIZE,
                );
            }
        }
    }
}

fn add_number(vs: &mut Vec<Vertex>, mut x: f32, y: f32, mut n: u32) {
    if n >= 10 {
        x += 2.0 * DIGIT_SIZE;
        if n >= 100 {
            x += 2.0 * DIGIT_SIZE;
        }
    }
    loop {
        add_digit(vs, x, y, n % 10);
        n /= 10;
        if n == 0 {
            break;
        }
        x -= 4.0 * DIGIT_SIZE;
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum UserInput {
    P1Up,
    P1Down,
    P2Up,
    P2Down,

    COUNT,
}

impl Default for Pong {
    fn default() -> Pong {
        Pong {
            input: [false; UserInput::COUNT as usize],
            pos1: 300.0,
            pos2: 300.0,
            score1: 0,
            score2: 0,
            ball: Ball::new(true, 300.0),
        }
    }
}
impl Pong {
    fn game_tick(&mut self) {
        const BAT_LIMIT: f32 = BAT_HEIGHT_EXT / 2.0;

        if self.input[UserInput::P1Up as usize] {
            self.pos1 -= 5.0;
        }
        if self.input[UserInput::P1Down as usize] {
            self.pos1 += 5.0;
        }
        if self.input[UserInput::P2Up as usize] {
            self.pos2 -= 5.0;
        }
        if self.input[UserInput::P2Down as usize] {
            self.pos2 += 5.0;
        }

        self.pos1 = self.pos1.clamp(BAT_LIMIT, 600.0 - BAT_LIMIT);
        self.pos2 = self.pos2.clamp(BAT_LIMIT, 600.0 - BAT_LIMIT);

        self.ball.pos += self.ball.vel;
        // Bounce on top wall
        if self.ball.pos.y < 10.0 {
            self.ball.pos.y = 10.0;
            if self.ball.vel.y < 0.0 {
                self.ball.vel.y *= -1.0;
            }
        }
        // Bounce on bottom wall
        if self.ball.pos.y > 600.0 - 10.0 {
            self.ball.pos.y = 600.0 - 10.0;
            if self.ball.vel.y > 0.0 {
                self.ball.vel.y *= -1.0;
            }
        }

        match &mut self.ball.phase {
            BallPhase::Running => {
                // Only one frame to bounce on the bats, it else bounces or changes the phase to Goal

                if self.ball.pos.x > 800.0 - 15.0 - 15.0 / 2.0 && self.ball.vel.x > 0.0 {
                    let dy = self.ball.pos.y - self.pos2;
                    self.ball.check_goal(dy);
                }
                if self.ball.pos.x < 15.0 + 15.0 / 2.0 && self.ball.vel.x < 0.0 {
                    let dy = self.ball.pos.y - self.pos1;
                    self.ball.check_goal(dy);
                }
            }
            BallPhase::Goal(0) => {
                if self.ball.vel.x > 0.0 {
                    self.score1 = (self.score1 + 1).min(999);
                    self.ball = Ball::new(false, self.pos2);
                } else {
                    self.score2 = (self.score2 + 1).min(999);
                    self.ball = Ball::new(true, self.pos1);
                }
            }
            BallPhase::Goal(tick) => {
                *tick -= 1;
            }
        }
    }
}

impl App {
    fn game_tick(&mut self) {
        self.pong.game_tick();
        let mut vs = Vec::new();

        // Dotted field line
        for i in 1..30 {
            let y = 20.0 * i as f32;
            add_box(&mut vs, 400.0, y, 5.0, 10.0);
        }
        // Top wall
        add_box(&mut vs, 400.0, 5.0, 800.0, 10.0);
        // Bottom wall
        add_box(&mut vs, 400.0, 595.0, 800.0, 10.0);

        add_number(&mut vs, 300.0, 30.0, self.pong.score1);
        add_number(&mut vs, 500.0, 30.0, self.pong.score2);

        // Player 1
        add_box(&mut vs, 15.0, self.pong.pos1, 15.0, BAT_HEIGHT);
        // Player 2
        add_box(&mut vs, 800.0 - 15.0, self.pong.pos2, 15.0, BAT_HEIGHT);

        // Ball
        add_box(
            &mut vs,
            self.pong.ball.pos.x,
            self.pong.ball.pos.y,
            15.0,
            15.0,
        );

        self.ds.set(vs);
    }
    fn user_input(&mut self, input: UserInput, pressed: bool) {
        self.pong.input[input as usize] = pressed;
    }
}

impl imgui::UiBuilder for App {
    fn pre_render(&mut self) {
        use glow::HasContext;

        unsafe {
            self.gl.bind_vertex_array(Some(self.vao.id()));
            self.gl.viewport(
                0,
                0,
                self.window_size.width as i32,
                self.window_size.height as i32,
            );
        }
        self.prg.draw(&self.u, &self.ds, glow::TRIANGLES);
    }
    fn build_custom_atlas(&mut self, atlas: &mut imgui::FontAtlasMut<'_, Self>) {
        self.font_normal = atlas.add_font(imgui::FontInfo::default_font(13.0));
        self.font_medium = atlas.add_font(imgui::FontInfo::default_font(30.0));
        self.font_big = atlas.add_font(imgui::FontInfo::default_font(60.0));
    }
    fn do_ui(&mut self, ui: &imgui::Ui<Self>) {
        if self.show_demo {
            ui.show_demo_window(Some(&mut self.show_demo));
        }
        if self.show_menu == Menu::None {
            return;
        }
        ui.set_next_window_size(Vector2::new(700.0, 500.0), imgui::Cond::Always);
        ui.set_next_window_pos(
            Vector2::new(50.0, 50.0),
            imgui::Cond::Always,
            Vector2::new(0.0, 0.0),
        );
        ui.set_next_window_bg_alpha(0.75);
        ui.with_push(self.font_big, || {
            ui.window_config("menu")
                .flags(
                    imgui::WindowFlags::NoTitleBar
                        | imgui::WindowFlags::NoResize
                        | imgui::WindowFlags::NoMove
                        | imgui::WindowFlags::NoDecoration
                        | imgui::WindowFlags::NoBringToFrontOnFocus,
                )
                .push_for_begin((
                    (
                        imgui::StyleVar::WindowBorderSize,
                        imgui::StyleValue::F32(0.0),
                    ),
                    (
                        imgui::StyleVar::WindowPadding,
                        imgui::StyleValue::Vec2(Vector2::new(50.0, 20.0)),
                    ),
                ))
                .with(|| match self.show_menu {
                    Menu::Hello => {
                        ui.set_cursor_pos_x(285.0);
                        ui.text("Pong");
                        ui.with_push(self.font_medium, || {
                            ui.text("This is a clone of the classic game");
                            ui.text("to demonstrate how to use easy-imgui");
                            ui.text("to build the in-game UI.");
                            ui.set_cursor_pos_y(ui.get_cursor_pos_y() + 20.0);
                            ui.text("Controls:");
                            ui.text("    Player #1: Q/A (A/Q in French)");
                            ui.text("    Player #2: P/; (key below P)");
                        });
                        ui.set_cursor_pos_y(380.0);
                        if ui.button("Ok") {
                            self.set_show_menu(Menu::Main);
                        }
                    }
                    Menu::Main => {
                        if ui.button("New game") {
                            self.pong = Pong::default();
                            self.set_show_menu(Menu::None);
                        }
                        ui.set_cursor_pos_y(ui.get_cursor_pos_y() + 20.0);
                        if ui.button("Continue") {
                            self.set_show_menu(Menu::None);
                        }
                        ui.set_cursor_pos_y(ui.get_cursor_pos_y() + 20.0);
                        if ui.button("Options") {
                            self.set_show_menu(Menu::Options);
                        }
                        ui.set_cursor_pos_y(380.0);
                        if ui.button("Quit") {
                            self.ui_request.insert(UiRequest::Quit);
                        }
                    }
                    Menu::Options => {
                        if ui.checkbox(
                            "Full-screen",
                            &mut self.ui_status.contains(UiRequest::Fullscreen),
                        ) {
                            self.ui_request.insert(UiRequest::Fullscreen);
                        }
                        ui.set_cursor_pos_y(ui.get_cursor_pos_y() + 20.0);
                        if ui.checkbox("V-Sync", &mut self.ui_status.contains(UiRequest::VSync)) {
                            self.ui_request.insert(UiRequest::VSync);
                        }
                        ui.set_cursor_pos_y(ui.get_cursor_pos_y() + 20.0);
                        ui.checkbox("ImGui Demo", &mut self.show_demo);
                        ui.set_cursor_pos_y(380.0);
                        if ui.button("Back") {
                            self.set_show_menu(Menu::Main);
                        }
                    }
                    Menu::None => unreachable!(),
                });
        });
    }
}

fn main() {
    //simple_logger::SimpleLogger::new().init().unwrap();
    let event_loop = EventLoopBuilder::new().build().unwrap();
    let main_window = MainWindow::new(&event_loop, "Example").unwrap();
    let mut window = MainWindowWithRenderer::new(main_window);
    window.renderer().set_background_color(Some(imgui::Color {
        r: 0.2,
        g: 0.2,
        b: 0.2,
        a: 1.0,
    }));

    window.renderer().set_matrix(Some(Matrix3::identity()));
    window.main_window().set_matrix(Some(Matrix3::identity()));

    let mut app = App::new(window.renderer().gl_context());
    app.game_tick();
    app.ui_status.insert(UiRequest::VSync);

    const TICK: Duration = Duration::from_micros(1_000_000 / 60);

    event_loop
        .run(move |event, w| {
            let ui_res = window.do_event_with_flags(&mut app, &event, EventFlags::DoNotResize);
            if ui_res.window_closed || app.ui_request.contains(UiRequest::Quit) {
                w.exit();
            }
            //continuous render
            window.ping_user_input();
            let mut ui_request = std::mem::take(&mut app.ui_request);

            match &event {
                winit::event::Event::WindowEvent { event, .. } => match event {
                    winit::event::WindowEvent::Resized(_) => {
                        let size = window.main_window().window().inner_size();
                        let (mx, offs, scale_inv) =
                            ratio_ortho(size.width as f32, size.height as f32);
                        //let angle = imgui::cgmath::Deg(5.0);
                        //let mx = mx * imgui::cgmath::Matrix3::from_angle_z(-angle);
                        app.u.m = mx;
                        window
                            .renderer()
                            .set_size(Vector2::new(800.0, 600.0), 1.0 / scale_inv);
                        window.renderer().set_matrix(Some(mx));
                        window.main_window().set_matrix(Some(
                            //imgui::cgmath::Matrix3::from_angle_z(angle) *
                            Matrix3::from_translation(-offs) * Matrix3::from_scale(scale_inv),
                        ));
                        app.window_size = size;
                    }
                    winit::event::WindowEvent::KeyboardInput { event, .. } => {
                        if !ui_res.want_capture_keyboard {
                            match event.physical_key {
                                winit::keyboard::PhysicalKey::Code(key) => match key {
                                    winit::keyboard::KeyCode::KeyQ => {
                                        app.user_input(UserInput::P1Up, event.state.is_pressed());
                                    }
                                    winit::keyboard::KeyCode::KeyA => {
                                        app.user_input(UserInput::P1Down, event.state.is_pressed());
                                    }
                                    winit::keyboard::KeyCode::KeyP => {
                                        app.user_input(UserInput::P2Up, event.state.is_pressed());
                                    }
                                    winit::keyboard::KeyCode::Semicolon => {
                                        app.user_input(UserInput::P2Down, event.state.is_pressed());
                                    }
                                    winit::keyboard::KeyCode::Escape
                                        if event.state.is_pressed() =>
                                    {
                                        let next = match app.show_menu {
                                            Menu::None => Menu::Main,
                                            Menu::Main => Menu::None,
                                            Menu::Options => Menu::Main,
                                            Menu::Hello => Menu::Main,
                                        };
                                        app.set_show_menu(next);
                                    }
                                    winit::keyboard::KeyCode::F11 if event.state.is_pressed() => {
                                        ui_request.insert(UiRequest::Fullscreen);
                                    }
                                    _ => {}
                                },
                                winit::keyboard::PhysicalKey::Unidentified(_) => todo!(),
                            }
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
            if ui_request.contains(UiRequest::VSync) {
                let w = window.main_window();
                let interval = if app.ui_status.contains(UiRequest::VSync) {
                    app.ui_status.remove(UiRequest::VSync);
                    glutin::surface::SwapInterval::DontWait
                } else {
                    app.ui_status.insert(UiRequest::VSync);
                    glutin::surface::SwapInterval::Wait(NonZeroU32::new(1).unwrap())
                };
                let _ = w.surface().set_swap_interval(&w.glutin_context(), interval);
            }
            if ui_request.contains(UiRequest::Fullscreen) {
                let w = window.main_window().window();
                if w.fullscreen().is_some() {
                    w.set_fullscreen(None);
                    app.ui_status.remove(UiRequest::Fullscreen);
                } else {
                    w.set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
                    app.ui_status.insert(UiRequest::Fullscreen);
                }
            }
            if ui_request.contains(UiRequest::HideCursor) {
                window.main_window().window().set_cursor_visible(false);
            }
            if ui_request.contains(UiRequest::ShowCursor) {
                window.main_window().window().set_cursor_visible(true);
            }
            if app.show_menu == Menu::None {
                let now = Instant::now();
                if now.duration_since(app.last_tick) > TICK {
                    app.game_tick();
                    app.last_tick += TICK;
                }
            }
        })
        .unwrap();
}
