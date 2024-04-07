use crate::conv::{from_imgui_cursor, to_imgui_button, to_imgui_key};
use easy_imgui::{self as imgui, mint, Vector2};
use easy_imgui_renderer::Renderer;
use easy_imgui_sys::*;
use glutin::{
    context::PossiblyCurrentContext,
    prelude::*,
    surface::{Surface, WindowSurface},
};
use std::num::NonZeroU32;
use std::time::{Duration, Instant};
use winit::{
    dpi::{LogicalSize, PhysicalSize},
    event::{Event, Ime::Commit},
    keyboard::PhysicalKey,
    window::{CursorIcon, Window},
};

// Only used with the main-window feature
#[allow(unused_imports)]
use winit::dpi::{LogicalPosition, PhysicalPosition, Pixel};

/// This struct maintains basic window info to be kept across events.
#[derive(Debug, Clone)]
pub struct MainWindowStatus {
    last_frame: Instant,
    current_cursor: Option<CursorIcon>,
}

impl Default for MainWindowStatus {
    fn default() -> MainWindowStatus {
        let now = Instant::now();
        MainWindowStatus {
            last_frame: now,
            current_cursor: Some(CursorIcon::Default),
        }
    }
}

/// This struct handles the main loop going to idle when there is no user input for a while.
pub struct MainWindowIdler {
    idle_time: Duration,
    idle_frame_count: u32,
    last_input_time: Instant,
    last_input_frame: u32,
}

impl Default for MainWindowIdler {
    fn default() -> MainWindowIdler {
        let now = Instant::now();
        MainWindowIdler {
            idle_time: Duration::from_secs(1),
            idle_frame_count: 60,
            last_input_time: now,
            last_input_frame: 0,
        }
    }
}

impl MainWindowIdler {
    /// Sets the maximum time that the window will be rendered without user input.
    pub fn set_idle_time(&mut self, time: Duration) {
        self.idle_time = time;
    }
    /// Sets the maximum number of frames time that the window will be rendered without user input.
    pub fn set_idle_frame_count(&mut self, frame_count: u32) {
        self.idle_frame_count = frame_count;
    }
    /// Call this when the window is renderer.
    pub fn incr_frame(&mut self) {
        // An u32 incrementing 60 values/second would overflow after about 2 years, better safe
        // than sorry.
        self.last_input_frame = self.last_input_frame.saturating_add(1);
    }
    /// Check whether the window should go to idle or keep on rendering.
    pub fn has_to_render(&self) -> bool {
        self.last_input_frame < self.idle_frame_count
            || Instant::now().duration_since(self.last_input_time) < self.idle_time
    }
    /// Notify this struct that user input happened.
    pub fn ping_user_input(&mut self) {
        self.last_input_time = Instant::now();
        self.last_input_frame = 0;
    }
}

/// This traits grants access to a Window.
///
/// Usually you will have a [`MainWindow`], but if you create the `Window` with an external
/// crate, maybe you don't own it.
pub trait MainWindowRef {
    /// Gets the [`Window`].
    fn window(&self) -> &Window;
    /// This runs just before rendering.
    ///
    /// The intended use is to make the GL context current, if needed.
    /// Clearing the background is usually done in [`easy_imgui::UiBuilder::pre_render`], or by the renderer if it has a background color.
    fn pre_render(&mut self) {}
    /// This runs just after rendering.
    ///
    /// The intended use is to present the screen buffer.
    fn post_render(&mut self) {}
    /// Notifies of a user interaction, for idling purposes.
    fn ping_user_input(&mut self) {}
    /// There are no more messages, going to idle.
    fn about_to_wait(&mut self, _pinged: bool) {}
    /// The window has been resized.
    ///
    /// Takes the new physical size. It should return the new logical size.
    fn resize(&mut self, size: PhysicalSize<u32>) -> LogicalSize<f32> {
        let scale = self.window().scale_factor();
        size.to_logical(scale)
    }
    /// Changes the mouse cursor.
    fn set_cursor(&mut self, cursor: Option<CursorIcon>) {
        let w = self.window();
        match cursor {
            None => w.set_cursor_visible(false),
            Some(c) => {
                w.set_cursor_icon(c);
                w.set_cursor_visible(true);
            }
        }
    }
}

bitflags::bitflags! {
    /// These flags can be used to customize the [`do_event`] function.
    #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
    pub struct EventFlags: u32 {
        /// Do not render the UI
        const DoNotRender = 1;
        /// Do not run the focus gained workaround.
        ///
        /// https://github.com/rust-windowing/winit/issues/2841
        const DoNotFixFocusGainedBug = 2;
    }
}

/// Helper struct to call [`do_event`] without owning the Window.
pub struct MainWindowPieces<'a> {
    pub window: &'a Window,
    pub surface: &'a Surface<WindowSurface>,
    pub gl_context: &'a PossiblyCurrentContext,
}

/// Default implementation if you have all the pieces.
impl<'a> MainWindowRef for MainWindowPieces<'a> {
    fn window(&self) -> &Window {
        self.window
    }
    fn pre_render(&mut self) {
        self.gl_context.make_current(self.surface).unwrap();
    }
    fn post_render(&mut self) {
        self.window.pre_present_notify();
        self.surface.swap_buffers(self.gl_context).unwrap();
    }
    fn resize(&mut self, size: PhysicalSize<u32>) -> LogicalSize<f32> {
        let width = NonZeroU32::new(size.width.max(1)).unwrap();
        let height = NonZeroU32::new(size.height.max(1)).unwrap();
        self.surface.resize(self.gl_context, width, height);
        let scale = self.window.scale_factor();
        size.to_logical(scale)
    }
}

/// Simple implementation if you only have a window, no pre/post render, no resize.
impl<'a> MainWindowRef for &'a Window {
    fn window(&self) -> &Window {
        self
    }
}

/// The result of processing an event in the ImGui loop.
#[derive(Debug, Default, Clone)]
pub struct EventResult {
    /// The user requested to close the window. You can break the loop or ignore it, at will.
    pub window_closed: bool,
    /// ImGui requests handling the mouse, your application should ignore mouse events.
    pub want_capture_mouse: bool,
    /// ImGui requests handling the keyboard, your application should ignore keyboard events.
    pub want_capture_keyboard: bool,
    /// ImGui requests handling text input, your application should ignore text events.
    pub want_text_input: bool,
}

/// Just like [`MainWindowWithRenderer::do_event`] but using all the pieces separately.
pub fn do_event<EventUserType>(
    main_window: &mut impl MainWindowRef,
    renderer: &mut Renderer,
    status: &mut MainWindowStatus,
    app: &mut impl imgui::UiBuilder,
    event: &Event<EventUserType>,
    flags: EventFlags,
) -> EventResult {
    let mut window_closed = false;
    match event {
        Event::NewEvents(x) => {
            if x == &winit::event::StartCause::Init
                && !flags.contains(EventFlags::DoNotFixFocusGainedBug)
            {
                // This fixes "keyboard non-responsive on startup because it doesn't detect FocusGained...":
                // https://github.com/rust-windowing/winit/issues/2841
                use winit::raw_window_handle::{
                    HasWindowHandle,
                    RawWindowHandle::{Xcb, Xlib},
                };
                if let Ok(h) = main_window.window().window_handle() {
                    if matches!(h.as_raw(), Xcb(_) | Xlib(_)) {
                        main_window.window().set_visible(false);
                        main_window.window().set_visible(true);
                    }
                }
            }
            let now = Instant::now();
            let mut imgui = unsafe { renderer.imgui().set_current() };
            let io = imgui.io_mut();
            io.DeltaTime = now.duration_since(status.last_frame).as_secs_f32();
            status.last_frame = now;
        }
        Event::AboutToWait => {
            let imgui = unsafe { renderer.imgui().set_current() };
            let io = imgui.io();
            if io.WantSetMousePos {
                let pos = io.MousePos;
                let pos = winit::dpi::LogicalPosition { x: pos.x, y: pos.y };
                let _ = main_window.window().set_cursor_position(pos);
            }
            // If the mouse is down, redraw all the time, maybe the user is dragging.
            let mouse = unsafe { ImGui_IsAnyMouseDown() };
            main_window.about_to_wait(mouse);
        }
        Event::WindowEvent { window_id, event } if main_window.window().id() == *window_id => {
            use winit::event::WindowEvent::*;
            match event {
                CloseRequested => {
                    window_closed = true;
                }
                RedrawRequested => unsafe {
                    let imgui = renderer.imgui().set_current();
                    let io = imgui.io();
                    let config_flags = imgui::ConfigFlags::from_bits_truncate(io.ConfigFlags);
                    if !config_flags.contains(imgui::ConfigFlags::NoMouseCursorChange) {
                        let cursor = if io.MouseDrawCursor {
                            None
                        } else {
                            let cursor = imgui::MouseCursor::from_bits(ImGui_GetMouseCursor())
                                .unwrap_or(imgui::MouseCursor::Arrow);
                            from_imgui_cursor(cursor)
                        };
                        if cursor != status.current_cursor {
                            main_window.set_cursor(cursor);
                            status.current_cursor = cursor;
                        }
                    }
                    if !flags.contains(EventFlags::DoNotRender) {
                        main_window.pre_render();
                        renderer.do_frame(app);
                        main_window.post_render();
                    }
                },
                Resized(size) => {
                    main_window.ping_user_input();
                    // GL surface in physical pixels, imgui in logical
                    let size = main_window.resize(*size);
                    let mut imgui = unsafe { renderer.imgui().set_current() };
                    let io = imgui.io_mut();
                    let size = Vector2::from(mint::Vector2::from(size));
                    io.DisplaySize = imgui::v2_to_im(size);
                }
                ScaleFactorChanged { scale_factor, .. } => {
                    main_window.ping_user_input();
                    let scale_factor = *scale_factor as f32;
                    let mut imgui = unsafe { renderer.imgui().set_current() };
                    let io = imgui.io_mut();
                    let old_scale_factor = io.DisplayFramebufferScale.x;
                    if io.MousePos.x.is_finite() && io.MousePos.y.is_finite() {
                        io.MousePos.x *= scale_factor / old_scale_factor;
                        io.MousePos.y *= scale_factor / old_scale_factor;
                    }
                    let size = renderer.size();
                    renderer.set_size(size, scale_factor);
                }
                ModifiersChanged(mods) => {
                    main_window.ping_user_input();
                    unsafe {
                        let mut imgui = renderer.imgui().set_current();
                        let io = imgui.io_mut();
                        ImGuiIO_AddKeyEvent(
                            io,
                            ImGuiKey(imgui::Key::ModCtrl.bits()),
                            mods.state().control_key(),
                        );
                        ImGuiIO_AddKeyEvent(
                            io,
                            ImGuiKey(imgui::Key::ModShift.bits()),
                            mods.state().shift_key(),
                        );
                        ImGuiIO_AddKeyEvent(
                            io,
                            ImGuiKey(imgui::Key::ModAlt.bits()),
                            mods.state().alt_key(),
                        );
                        ImGuiIO_AddKeyEvent(
                            io,
                            ImGuiKey(imgui::Key::ModSuper.bits()),
                            mods.state().super_key(),
                        );
                    }
                }
                KeyboardInput {
                    event:
                        winit::event::KeyEvent {
                            physical_key,
                            text,
                            state,
                            ..
                        },
                    is_synthetic: false,
                    ..
                } => {
                    main_window.ping_user_input();
                    let pressed = *state == winit::event::ElementState::Pressed;
                    if let Some(key) = to_imgui_key(*physical_key) {
                        unsafe {
                            let mut imgui = renderer.imgui().set_current();
                            let io = imgui.io_mut();
                            ImGuiIO_AddKeyEvent(io, ImGuiKey(key.bits()), pressed);

                            use winit::keyboard::KeyCode::*;
                            if let PhysicalKey::Code(keycode) = physical_key {
                                let kmod = match keycode {
                                    ControlLeft | ControlRight => Some(imgui::Key::ModCtrl),
                                    ShiftLeft | ShiftRight => Some(imgui::Key::ModShift),
                                    AltLeft | AltRight => Some(imgui::Key::ModAlt),
                                    SuperLeft | SuperRight => Some(imgui::Key::ModSuper),
                                    _ => None,
                                };
                                if let Some(kmod) = kmod {
                                    ImGuiIO_AddKeyEvent(io, ImGuiKey(kmod.bits()), pressed);
                                }
                            }
                        }
                    }
                    if pressed {
                        if let Some(text) = text {
                            unsafe {
                                let mut imgui = renderer.imgui().set_current();
                                let io = imgui.io_mut();
                                for c in text.chars() {
                                    ImGuiIO_AddInputCharacter(io, c as u32);
                                }
                            }
                        }
                    }
                }
                Ime(Commit(text)) => {
                    main_window.ping_user_input();
                    unsafe {
                        let mut imgui = renderer.imgui().set_current();
                        let io = imgui.io_mut();
                        for c in text.chars() {
                            ImGuiIO_AddInputCharacter(io, c as u32);
                        }
                    }
                }
                CursorMoved { position, .. } => {
                    main_window.ping_user_input();
                    unsafe {
                        let mut imgui = renderer.imgui().set_current();
                        let io = imgui.io_mut();
                        let scale = main_window.window().scale_factor();
                        let position = position.to_logical(scale);
                        ImGuiIO_AddMousePosEvent(io, position.x, position.y);
                    }
                }
                MouseWheel {
                    delta,
                    phase: winit::event::TouchPhase::Moved,
                    ..
                } => {
                    main_window.ping_user_input();
                    let mut imgui = unsafe { renderer.imgui().set_current() };
                    let io = imgui.io_mut();
                    let (h, v) = match delta {
                        winit::event::MouseScrollDelta::LineDelta(h, v) => (*h, *v),
                        winit::event::MouseScrollDelta::PixelDelta(d) => {
                            let scale = io.DisplayFramebufferScale.x;
                            let f_scale = unsafe { ImGui_GetFontSize() };
                            let scale = scale * f_scale;
                            (d.x as f32 / scale, d.y as f32 / scale)
                        }
                    };
                    unsafe {
                        ImGuiIO_AddMouseWheelEvent(io, h, v);
                    }
                }
                MouseInput { state, button, .. } => {
                    main_window.ping_user_input();
                    unsafe {
                        let mut imgui = renderer.imgui().set_current();
                        let io = imgui.io_mut();
                        if let Some(btn) = to_imgui_button(*button) {
                            let pressed = *state == winit::event::ElementState::Pressed;
                            ImGuiIO_AddMouseButtonEvent(io, btn.bits(), pressed);
                        }
                    }
                }
                CursorLeft { .. } => {
                    main_window.ping_user_input();
                    unsafe {
                        let mut imgui = renderer.imgui().set_current();
                        let io = imgui.io_mut();
                        ImGuiIO_AddMousePosEvent(io, f32::MAX, f32::MAX);
                    }
                }
                Focused(focused) => {
                    main_window.ping_user_input();
                    unsafe {
                        let mut imgui = renderer.imgui().set_current();
                        let io = imgui.io_mut();
                        ImGuiIO_AddFocusEvent(io, *focused);
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
    let res = unsafe {
        let imgui = renderer.imgui().set_current();
        EventResult {
            window_closed,
            want_capture_mouse: imgui.want_capture_mouse(),
            want_capture_keyboard: imgui.want_capture_keyboard(),
            want_text_input: imgui.want_text_input(),
        }
    };
    res
}

#[cfg(feature = "clipboard")]
pub mod clipboard {
    use easy_imgui as imgui;
    use std::ffi::{c_char, c_void, CStr, CString};

    /// Sets up the ImGui clipboard using the `arboard` crate.
    pub fn setup(imgui: &mut imgui::CurrentContext<'_>) {
        if let Ok(ctx) = arboard::Clipboard::new() {
            let clip = MyClipboard {
                ctx,
                text: CString::default(),
            };
            let io = imgui.io_mut();
            io.ClipboardUserData = Box::into_raw(Box::new(clip)) as *mut c_void;
            io.SetClipboardTextFn = Some(set_clipboard_text);
            io.GetClipboardTextFn = Some(get_clipboard_text);
        }
    }
    unsafe extern "C" fn set_clipboard_text(user: *mut c_void, text: *const c_char) {
        let clip = &mut *(user as *mut MyClipboard);
        if text.is_null() {
            let _ = clip.ctx.clear();
        } else {
            let cstr = CStr::from_ptr(text);
            let str = String::from_utf8_lossy(cstr.to_bytes()).to_string();
            let _ = clip.ctx.set_text(str);
        }
    }

    // The returned pointer should be valid for a while...
    unsafe extern "C" fn get_clipboard_text(user: *mut c_void) -> *const c_char {
        let clip = &mut *(user as *mut MyClipboard);
        let Ok(text) = clip.ctx.get_text() else {
            return std::ptr::null();
        };
        let Ok(text) = CString::new(text) else {
            return std::ptr::null();
        };
        clip.text = text;
        clip.text.as_ptr()
    }

    struct MyClipboard {
        ctx: arboard::Clipboard,
        text: CString,
    }
}

#[cfg(feature = "main-window")]
mod main_window {
    use super::*;
    use anyhow::{anyhow, Result};
    use easy_imgui_renderer::glow;
    use glutin::{
        config::{Config, ConfigTemplateBuilder},
        context::{ContextApi, ContextAttributesBuilder},
        display::GetGlDisplay,
        surface::SurfaceAttributesBuilder,
    };
    use glutin_winit::DisplayBuilder;
    use raw_window_handle::HasRawWindowHandle;
    use winit::{event_loop::EventLoopWindowTarget, window::WindowBuilder};

    /// This type represents a `winit` window and an OpenGL context.
    pub struct MainWindow {
        gl_context: PossiblyCurrentContext,
        // The surface must be dropped before the window.
        surface: Surface<WindowSurface>,
        window: Window,
        idler: MainWindowIdler,
    }

    /// This is a [`MainWindow`] plus a [`Renderer`]. It is the ultimate `easy-imgui` object.
    /// Instead of a literal `MainWindow` you can use any type that implements [`MainWindowRef`].
    pub struct MainWindowWithRenderer {
        main_window: MainWindow,
        renderer: Renderer,
        status: MainWindowStatus,
    }

    impl MainWindow {
        /// Creates a `MainWindow` with default values.
        pub fn new<EventUserType>(
            event_loop: &EventLoopWindowTarget<EventUserType>,
            title: &str,
        ) -> Result<MainWindow> {
            // For standard UI, we need as few fancy things as available
            let score = |c: &Config| (c.num_samples(), c.depth_size(), c.stencil_size());
            Self::with_gl_chooser(event_loop, title, |cfg1, cfg2| {
                if score(&cfg2) < score(&cfg1) {
                    cfg2
                } else {
                    cfg1
                }
            })
        }
        /// Creates a `MainWindow` with your own OpenGL context chooser.
        ///
        /// If you don't have specific OpenGL needs, prefer using [`MainWindow::new`]. If you do,
        /// consider using a _FramebufferObject_ and do an offscreen rendering instead.
        pub fn with_gl_chooser<EventUserType>(
            event_loop: &EventLoopWindowTarget<EventUserType>,
            title: &str,
            f_choose_cfg: impl FnMut(Config, Config) -> Config,
        ) -> Result<MainWindow> {
            let window_builder = WindowBuilder::new();
            let template = ConfigTemplateBuilder::new()
                .prefer_hardware_accelerated(Some(true))
                .with_depth_size(0)
                .with_stencil_size(0);

            let display_builder = DisplayBuilder::new().with_window_builder(Some(window_builder));

            let (window, gl_config) = display_builder
                .build(event_loop, template, |configs| {
                    configs.reduce(f_choose_cfg).unwrap()
                })
                .map_err(|e| anyhow!("{:#?}", e))?;
            let window = window.unwrap();
            window.set_title(title);
            window.set_ime_allowed(true);
            let raw_window_handle = Some(window.raw_window_handle());
            let gl_display = gl_config.display();
            let context_attributes = ContextAttributesBuilder::new().build(raw_window_handle);
            let fallback_context_attributes = ContextAttributesBuilder::new()
                .with_context_api(ContextApi::Gles(None))
                .build(raw_window_handle);

            let mut not_current_gl_context = Some(unsafe {
                gl_display
                    .create_context(&gl_config, &context_attributes)
                    .or_else(|_| {
                        gl_display.create_context(&gl_config, &fallback_context_attributes)
                    })?
            });

            let size = window.inner_size();

            let (width, height): (u32, u32) = size.into();
            let raw_window_handle = window.raw_window_handle();
            let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
                raw_window_handle,
                NonZeroU32::new(width).unwrap(),
                NonZeroU32::new(height).unwrap(),
            );

            let surface = unsafe {
                gl_config
                    .display()
                    .create_window_surface(&gl_config, &attrs)?
            };
            let gl_context = not_current_gl_context
                .take()
                .unwrap()
                .make_current(&surface)?;

            // Enable v-sync to avoid consuming too much CPU
            let _ = surface.set_swap_interval(
                &gl_context,
                glutin::surface::SwapInterval::Wait(NonZeroU32::new(1).unwrap()),
            );

            Ok(MainWindow {
                gl_context,
                window,
                surface,
                idler: MainWindowIdler::default(),
            })
        }
        /// Splits this window into its parts.
        ///
        /// # Safety
        /// Do not drop the `window` without dropping the `surface` first.
        pub unsafe fn into_pieces(
            self,
        ) -> (PossiblyCurrentContext, Surface<WindowSurface>, Window) {
            (self.gl_context, self.surface, self.window)
        }
        pub fn glutin_context(&self) -> &PossiblyCurrentContext {
            &self.gl_context
        }
        /// Creates a new `glow` OpenGL context for this window and the selected configuration.
        pub fn create_gl_context(&self) -> glow::Context {
            let dsp = self.gl_context.display();
            unsafe { glow::Context::from_loader_function_cstr(|s| dsp.get_proc_address(s)) }
        }
        /// Gets a reference to the `winit` window.
        pub fn window(&self) -> &Window {
            &self.window
        }
        /// Converts the given physical size to a logical size, using the window scale factor.
        pub fn to_logical_size<X: Pixel, Y: Pixel>(&self, size: PhysicalSize<X>) -> LogicalSize<Y> {
            let scale = self.window.scale_factor();
            size.to_logical(scale)
        }
        /// Converts the given logical size to a physical size, using the window scale factor.
        pub fn to_physical_size<X: Pixel, Y: Pixel>(
            &self,
            size: LogicalSize<X>,
        ) -> PhysicalSize<Y> {
            let scale = self.window.scale_factor();
            size.to_physical(scale)
        }
        /// Converts the given physical position to a logical position, using the window scale factor.
        pub fn to_logical_pos<X: Pixel, Y: Pixel>(
            &self,
            pos: PhysicalPosition<X>,
        ) -> LogicalPosition<Y> {
            let scale = self.window.scale_factor();
            pos.to_logical(scale)
        }
        /// Converts the given logical position to a physical position, using the window scale factor.
        pub fn to_physical_pos<X: Pixel, Y: Pixel>(
            &self,
            pos: LogicalPosition<X>,
        ) -> PhysicalPosition<Y> {
            let scale = self.window.scale_factor();
            pos.to_physical(scale)
        }
    }

    impl MainWindowWithRenderer {
        /// Creates a new [`Renderer`] and attaches it to the given window.
        pub fn new(main_window: MainWindow) -> Self {
            let gl = main_window.create_gl_context();
            let renderer = Renderer::new(std::rc::Rc::new(gl)).unwrap();
            Self::new_with_renderer(main_window, renderer)
        }
        /// Sets the time after which the UI will stop rendering, if there is no user input.
        pub fn set_idle_time(&mut self, time: Duration) {
            self.main_window.idler.set_idle_time(time);
        }
        /// Sets the frame count after which the UI will stop rendering, if there is no user input.
        ///
        /// Note that by default V-Sync is enabled, and that will affect the frame rate.
        pub fn set_idle_frame_count(&mut self, frame_count: u32) {
            self.main_window.idler.set_idle_frame_count(frame_count);
        }
        /// Forces a rebuild of the UI.
        ///
        /// By default the window will stop rendering the UI after a while without user input. Use this
        /// function to force a redraw because of some external factor.
        pub fn ping_user_input(&mut self) {
            self.main_window.idler.ping_user_input();
        }
        /// Gets a reference to the inner renderer.
        pub fn renderer(&mut self) -> &mut Renderer {
            &mut self.renderer
        }
        /// Gets a reference to the inner window.
        pub fn main_window(&mut self) -> &mut MainWindow {
            &mut self.main_window
        }
        /// Attaches the given window and renderer together.
        pub fn new_with_renderer(main_window: MainWindow, mut renderer: Renderer) -> Self {
            let w = main_window.window();
            let size = w.inner_size();
            let scale = w.scale_factor();
            let size = size.to_logical::<f32>(scale);
            renderer.set_size(Vector2::from(mint::Vector2::from(size)), scale as f32);

            let mut imgui = unsafe { renderer.imgui().set_current() };

            #[cfg(feature = "clipboard")]
            clipboard::setup(&mut imgui);

            MainWindowWithRenderer {
                main_window,
                renderer,
                status: MainWindowStatus::default(),
            }
        }
        /// The main event function, to be called from your event loop.
        ///
        /// It returns [`EventResult`]. You can use it to break the main loop, or ignore it, as you see fit.
        /// It also informs of whether ImGui want the monopoly of the user input.
        pub fn do_event<EventUserType>(
            &mut self,
            app: &mut impl imgui::UiBuilder,
            event: &Event<EventUserType>,
        ) -> EventResult {
            do_event(
                &mut self.main_window,
                &mut self.renderer,
                &mut self.status,
                app,
                event,
                EventFlags::empty(),
            )
        }
    }

    /// Main implementation of the `MainWindowRef` trait for an owned `MainWindow`.
    impl MainWindowRef for MainWindow {
        fn window(&self) -> &Window {
            &self.window
        }
        fn pre_render(&mut self) {
            self.idler.incr_frame();
            self.gl_context.make_current(&self.surface).unwrap();
        }
        fn post_render(&mut self) {
            self.window.pre_present_notify();
            self.surface.swap_buffers(&self.gl_context).unwrap();
        }
        fn resize(&mut self, size: PhysicalSize<u32>) -> LogicalSize<f32> {
            let width = NonZeroU32::new(size.width.max(1)).unwrap();
            let height = NonZeroU32::new(size.height.max(1)).unwrap();
            self.surface.resize(&self.gl_context, width, height);
            self.to_logical_size::<_, f32>(size)
        }
        fn ping_user_input(&mut self) {
            self.idler.ping_user_input();
        }
        fn about_to_wait(&mut self, pinged: bool) {
            if pinged || self.idler.has_to_render() {
                // No need to call set_control_flow(): doing a redraw will force extra Poll.
                // Not doing it will default to Wait.
                self.window.request_redraw();
            }
        }
    }
}

#[cfg(feature = "main-window")]
pub use main_window::*;
