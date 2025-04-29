use crate::conv::{from_imgui_cursor, to_imgui_button, to_imgui_key};
use cgmath::Matrix3;
use easy_imgui::{self as imgui, cgmath, mint, HasImGuiContext, Vector2};
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
    event::Ime::Commit,
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
    /// Transform the given `pos` by using the current scale factor.
    fn transform_position(&self, pos: Vector2) -> Vector2 {
        pos / self.scale_factor()
    }
    /// Gets the scale factor of the window, (HiDPI).
    fn scale_factor(&self) -> f32 {
        self.window().scale_factor() as f32
    }
    /// Changes the scale factor.
    ///
    /// Normally there is nothing to be done here, unless you are doing something fancy with HiDPI.
    ///
    /// It returns the real applied scale factor, as it would returned by
    /// `self.scale_factor()` after this change has been applied.
    fn set_scale_factor(&self, scale: f32) -> f32 {
        scale
    }
    /// The window has been resized.
    ///
    /// Takes the new physical size. It should return the new logical size.
    fn resize(&mut self, size: PhysicalSize<u32>) -> LogicalSize<f32> {
        let scale = self.scale_factor();
        size.to_logical(scale as f64)
    }
    /// Changes the mouse cursor.
    fn set_cursor(&mut self, cursor: Option<CursorIcon>) {
        let w = self.window();
        match cursor {
            None => w.set_cursor_visible(false),
            Some(c) => {
                w.set_cursor(c);
                w.set_cursor_visible(true);
            }
        }
    }
}

fn transform_position_with_optional_matrix(
    w: &impl MainWindowRef,
    pos: Vector2,
    mx: &Option<Matrix3<f32>>,
) -> Vector2 {
    use cgmath::{EuclideanSpace as _, Transform};
    match mx {
        Some(mx) => mx.transform_point(cgmath::Point2::from_vec(pos)).to_vec(),
        None => pos / w.scale_factor(),
    }
}

bitflags::bitflags! {
    /// These flags can be used to customize the [`window_event`] function.
    #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
    pub struct EventFlags: u32 {
        /// Do not render the UI
        const DoNotRender = 1;
        /// Do not change the size or scale of the UI
        const DoNotResize = 4;
        /// Do not send mouse positions
        const DoNotMouse = 8;
    }
}

/// Helper struct to call [`window_event`] without owning the Window.
pub struct MainWindowPieces<'a> {
    window: &'a Window,
    surface: &'a Surface<WindowSurface>,
    gl_context: &'a PossiblyCurrentContext,
    matrix: Option<Matrix3<f32>>,
}

impl<'a> MainWindowPieces<'a> {
    /// Creates a value from the pieces.
    pub fn new(
        window: &'a Window,
        surface: &'a Surface<WindowSurface>,
        gl_context: &'a PossiblyCurrentContext,
    ) -> Self {
        MainWindowPieces {
            window,
            surface,
            gl_context,
            matrix: None,
        }
    }
    /// Sets the matrix that transforms the input mouse coordinates into UI space.
    ///
    /// If none, it uses the default transformation.
    pub fn set_matrix(&mut self, matrix: Option<Matrix3<f32>>) {
        self.matrix = matrix;
    }
}

/// Default implementation if you have all the pieces.
impl MainWindowRef for MainWindowPieces<'_> {
    fn window(&self) -> &Window {
        self.window
    }
    fn pre_render(&mut self) {
        let _ = self
            .gl_context
            .make_current(self.surface)
            .inspect_err(|e| log::error!("{e}"));
    }
    fn post_render(&mut self) {
        self.window.pre_present_notify();
        let _ = self
            .surface
            .swap_buffers(self.gl_context)
            .inspect_err(|e| log::error!("{e}"));
    }
    fn resize(&mut self, size: PhysicalSize<u32>) -> LogicalSize<f32> {
        let width = NonZeroU32::new(size.width.max(1)).unwrap();
        let height = NonZeroU32::new(size.height.max(1)).unwrap();
        self.surface.resize(self.gl_context, width, height);
        let scale = self.scale_factor();
        size.to_logical(scale as f64)
    }
    fn transform_position(&self, pos: Vector2) -> Vector2 {
        transform_position_with_optional_matrix(self, pos, &self.matrix)
    }
}

/// Simple implementation if you only have a window, no pre/post render, no resize.
impl MainWindowRef for &Window {
    fn window(&self) -> &Window {
        self
    }
}

/// NewType to disable the HiDPI scaling.
pub struct NoScale<'a>(pub &'a Window);

impl MainWindowRef for NoScale<'_> {
    fn window(&self) -> &Window {
        self.0
    }
    fn scale_factor(&self) -> f32 {
        1.0
    }
    fn set_scale_factor(&self, _scale: f32) -> f32 {
        1.0
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

/// Corresponds to winit's `ApplicationHandler::new_events`.
pub fn new_events(renderer: &mut Renderer, status: &mut MainWindowStatus) {
    let now = Instant::now();
    unsafe {
        renderer.imgui().io_mut().DeltaTime = now.duration_since(status.last_frame).as_secs_f32();
    }
    status.last_frame = now;
}

/// Corresponds to winit's `ApplicationHandler::about_to_wait`.
pub fn about_to_wait(main_window: &mut impl MainWindowRef, renderer: &mut Renderer) {
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

/// Corresponds to winit's `ApplicationHandler::window_event`.
pub fn window_event(
    main_window: &mut impl MainWindowRef,
    renderer: &mut Renderer,
    status: &mut MainWindowStatus,
    app: &mut impl imgui::UiBuilder,
    event: &winit::event::WindowEvent,
    flags: EventFlags,
) -> EventResult {
    use winit::event::WindowEvent::*;
    let mut window_closed = false;
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
            // Do not skip this line or the gl surface may be wrong in Wayland
            // GL surface in physical pixels, imgui in logical
            let size = main_window.resize(*size);
            if !flags.contains(EventFlags::DoNotResize) {
                main_window.ping_user_input();
                // GL surface in physical pixels, imgui in logical
                let size = Vector2::from(mint::Vector2::from(size));
                unsafe {
                    renderer.imgui().io_mut().DisplaySize = imgui::v2_to_im(size);
                }
            }
        }
        ScaleFactorChanged { scale_factor, .. } => {
            if !flags.contains(EventFlags::DoNotResize) {
                main_window.ping_user_input();
                let scale_factor = main_window.set_scale_factor(*scale_factor as f32);
                unsafe {
                    let io = renderer.imgui().io_mut();
                    // Keep the mouse in the same relative position: maybe it is wrong, but it is
                    // the best guess we can do.
                    let old_scale_factor = io.DisplayFramebufferScale.x;
                    if io.MousePos.x.is_finite() && io.MousePos.y.is_finite() {
                        io.MousePos.x *= scale_factor / old_scale_factor;
                        io.MousePos.y *= scale_factor / old_scale_factor;
                    }
                }
                let size = renderer.size();
                renderer.set_size(size, scale_factor);
            }
        }
        ModifiersChanged(mods) => {
            main_window.ping_user_input();
            unsafe {
                let io = renderer.imgui().io_mut();
                io.AddKeyEvent(imgui::Key::ModCtrl.bits(), mods.state().control_key());
                io.AddKeyEvent(imgui::Key::ModShift.bits(), mods.state().shift_key());
                io.AddKeyEvent(imgui::Key::ModAlt.bits(), mods.state().alt_key());
                io.AddKeyEvent(imgui::Key::ModSuper.bits(), mods.state().super_key());
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
                    let io = renderer.imgui().io_mut();
                    io.AddKeyEvent(key.bits(), pressed);

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
                            io.AddKeyEvent(kmod.bits(), pressed);
                        }
                    }
                }
            }
            if pressed {
                if let Some(text) = text {
                    unsafe {
                        let io = renderer.imgui().io_mut();
                        for c in text.chars() {
                            io.AddInputCharacter(c as u32);
                        }
                    }
                }
            }
        }
        Ime(Commit(text)) => {
            main_window.ping_user_input();
            unsafe {
                let io = renderer.imgui().io_mut();
                for c in text.chars() {
                    io.AddInputCharacter(c as u32);
                }
            }
        }
        CursorMoved { position, .. } => {
            main_window.ping_user_input();
            unsafe {
                let io = renderer.imgui().io_mut();
                let position = main_window
                    .transform_position(Vector2::new(position.x as f32, position.y as f32));
                io.AddMousePosEvent(position.x, position.y);
            }
        }
        MouseWheel {
            delta,
            phase: winit::event::TouchPhase::Moved,
            ..
        } => {
            main_window.ping_user_input();
            let mut imgui = unsafe { renderer.imgui().set_current() };
            unsafe {
                let io = imgui.io_mut();
                let (h, v) = match delta {
                    winit::event::MouseScrollDelta::LineDelta(h, v) => (*h, *v),
                    winit::event::MouseScrollDelta::PixelDelta(d) => {
                        let scale = io.DisplayFramebufferScale.x;
                        let f_scale = ImGui_GetFontSize();
                        let scale = scale * f_scale;
                        (d.x as f32 / scale, d.y as f32 / scale)
                    }
                };
                io.AddMouseWheelEvent(h, v);
            }
        }
        MouseInput { state, button, .. } => {
            main_window.ping_user_input();
            unsafe {
                let io = renderer.imgui().io_mut();
                if let Some(btn) = to_imgui_button(*button) {
                    let pressed = *state == winit::event::ElementState::Pressed;
                    io.AddMouseButtonEvent(btn.bits(), pressed);
                }
            }
        }
        CursorLeft { .. } => {
            main_window.ping_user_input();
            unsafe {
                let io = renderer.imgui().io_mut();
                io.AddMousePosEvent(f32::MAX, f32::MAX);
            }
        }
        Focused(focused) => {
            main_window.ping_user_input();
            unsafe {
                let io = renderer.imgui().io_mut();
                io.AddFocusEvent(*focused);
            }
        }
        _ => {}
    }
    let imgui = renderer.imgui();
    EventResult {
        window_closed,
        want_capture_mouse: imgui.want_capture_mouse(),
        want_capture_keyboard: imgui.want_capture_keyboard(),
        want_text_input: imgui.want_text_input(),
    }
}

#[cfg(feature = "clipboard")]
/// Easy wrapper for the clipboard functions.
///
/// This module depends on the `clipboard` feature. Usually this is set up automatically just by
/// enabling the faature.
pub mod clipboard {
    use easy_imgui::{self as imgui, HasImGuiContext};
    use std::ffi::{c_char, c_void, CStr, CString};

    /// Sets up the ImGui clipboard using the `arboard` crate.
    pub fn setup(imgui: &mut imgui::Context) {
        if let Ok(ctx) = arboard::Clipboard::new() {
            let clip = MyClipboard {
                ctx,
                text: CString::default(),
            };
            unsafe {
                let pio = imgui.platform_io_mut();
                pio.Platform_ClipboardUserData = Box::into_raw(Box::new(clip)) as *mut c_void;
                pio.Platform_SetClipboardTextFn = Some(set_clipboard_text);
                pio.Platform_GetClipboardTextFn = Some(get_clipboard_text);
            }
        }
    }
    unsafe extern "C" fn set_clipboard_text(
        imgui: *mut easy_imgui_sys::ImGuiContext,
        text: *const c_char,
    ) {
        let user = (*imgui).PlatformIO.Platform_ClipboardUserData;
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
    unsafe extern "C" fn get_clipboard_text(
        imgui: *mut easy_imgui_sys::ImGuiContext,
    ) -> *const c_char {
        let user = (*imgui).PlatformIO.Platform_ClipboardUserData;
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
    use std::future::Future;
    mod fut;
    use anyhow::{anyhow, Result};
    use easy_imgui_renderer::glow;
    pub use fut::{FutureBackCaller, FutureHandle, FutureHandleGuard};
    use glutin::{
        config::{Config, ConfigTemplateBuilder},
        context::{ContextApi, ContextAttributesBuilder},
        display::GetGlDisplay,
        surface::SurfaceAttributesBuilder,
    };
    use glutin_winit::DisplayBuilder;
    use raw_window_handle::HasWindowHandle;
    use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
    use winit::window::WindowAttributes;

    /// This type represents a `winit` window and an OpenGL context.
    pub struct MainWindow {
        gl_context: PossiblyCurrentContext,
        // The surface must be dropped before the window.
        surface: Surface<WindowSurface>,
        window: Window,
        matrix: Option<Matrix3<f32>>,
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
        pub fn new(event_loop: &ActiveEventLoop, wattr: WindowAttributes) -> Result<MainWindow> {
            // For standard UI, we need as few fancy things as available
            let score = |c: &Config| (c.num_samples(), c.depth_size(), c.stencil_size());
            Self::with_gl_chooser(event_loop, wattr, |cfg1, cfg2| {
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
        pub fn with_gl_chooser(
            event_loop: &ActiveEventLoop,
            wattr: WindowAttributes,
            f_choose_cfg: impl FnMut(Config, Config) -> Config,
        ) -> Result<MainWindow> {
            let template = ConfigTemplateBuilder::new()
                .prefer_hardware_accelerated(Some(true))
                .with_depth_size(0)
                .with_stencil_size(0);

            let display_builder = DisplayBuilder::new().with_window_attributes(Some(wattr));
            let (window, gl_config) = display_builder
                .build(event_loop, template, |configs| {
                    configs.reduce(f_choose_cfg).unwrap()
                })
                .map_err(|e| anyhow!("{:#?}", e))?;
            let window = window.unwrap();
            window.set_ime_allowed(true);
            let raw_window_handle = Some(window.window_handle().unwrap().as_raw());
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
            let raw_window_handle = window.window_handle().unwrap().as_raw();
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
                matrix: None,
                idler: MainWindowIdler::default(),
            })
        }
        /// Sets a custom matrix that converts physical mouse coordinates into logical ones.
        pub fn set_matrix(&mut self, matrix: Option<Matrix3<f32>>) {
            self.matrix = matrix;
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
        /// Returns the `glutin` context.
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
        /// Returns the `glutin` surface.
        pub fn surface(&self) -> &Surface<WindowSurface> {
            &self.surface
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
        /// Gets a reference to the ImGui context by the renderer.
        ///
        /// Just like `self.renderer().imgui()`
        pub fn imgui(&mut self) -> &mut imgui::Context {
            self.renderer.imgui()
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

            #[cfg(feature = "clipboard")]
            clipboard::setup(&mut renderer.imgui());

            MainWindowWithRenderer {
                main_window,
                renderer,
                status: MainWindowStatus::default(),
            }
        }
        /// The main event function. Corresponds to winit's `ApplicationHandler::window_event`.
        ///
        /// It returns [`EventResult`]. You can use it to break the main loop, or ignore it, as you see fit.
        /// It also informs of whether ImGui want the monopoly of the user input.
        pub fn window_event(
            &mut self,
            app: &mut impl imgui::UiBuilder,
            event: &winit::event::WindowEvent,
            flags: EventFlags,
        ) -> EventResult {
            window_event(
                &mut self.main_window,
                &mut self.renderer,
                &mut self.status,
                app,
                event,
                flags,
            )
        }

        /// Corresponds to winit's `ApplicationHandler::new_events`.
        pub fn new_events(&mut self) {
            new_events(&mut self.renderer, &mut self.status);
        }
        /// Corresponds to winit's `ApplicationHandler::about_to_wait`.
        pub fn about_to_wait(&mut self) {
            about_to_wait(&mut self.main_window, &mut self.renderer);
        }
    }

    /// Main implementation of the `MainWindowRef` trait for an owned `MainWindow`.
    impl MainWindowRef for MainWindow {
        fn window(&self) -> &Window {
            &self.window
        }
        fn pre_render(&mut self) {
            self.idler.incr_frame();
            let _ = self
                .gl_context
                .make_current(&self.surface)
                .inspect_err(|e| log::error!("{e}"));
        }
        fn post_render(&mut self) {
            self.window.pre_present_notify();
            let _ = self
                .surface
                .swap_buffers(&self.gl_context)
                .inspect_err(|e| log::error!("{e}"));
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
        fn transform_position(&self, pos: Vector2) -> Vector2 {
            transform_position_with_optional_matrix(self, pos, &self.matrix)
        }
    }

    /// This type is an aggregate of values retured by [`AppHandler`].
    ///
    /// With this you don't need to have a buch of `use` that you probably
    /// don't care about.
    ///
    /// Since this is not `Send` it is always used from the main loop, and it can be
    /// used to send non `Send` callbacks to the idle loop.
    #[non_exhaustive]
    pub struct Args<'a, A: Application> {
        /// The main window.
        pub window: &'a mut MainWindowWithRenderer,
        /// The event loop.
        pub event_loop: &'a ActiveEventLoop,
        /// A proxy to send messages to the main loop.
        pub event_proxy: &'a EventLoopProxy<AppEvent<A>>,
        /// The custom application data.
        pub data: &'a mut A::Data,
    }

    /// This type is a wrapper for `EventLoopProxy` that is not `Send`.
    ///
    /// Since it can only be used from the main loop, it can send events that are not `Send`.
    pub struct LocalProxy<A: Application> {
        event_proxy: EventLoopProxy<AppEvent<A>>,
        // !Send + !Sync
        pd: std::marker::PhantomData<*const ()>,
    }

    impl<A: Application> Clone for LocalProxy<A> {
        fn clone(&self) -> Self {
            LocalProxy {
                event_proxy: self.event_proxy.clone(),
                pd: std::marker::PhantomData,
            }
        }
    }

    macro_rules! local_proxy_impl {
        () => {
            /// Registers a future to be run during the idle step of the main loop.
            pub fn spawn_idle<T: 'static, F: Future<Output = T> + 'static>(
                &self,
                f: F,
            ) -> crate::FutureHandle<T> {
                unsafe { fut::spawn_idle(&self.event_proxy, f) }
            }
            /// Registers a callback to be called during the idle step of the main loop.
            pub fn run_idle<F: FnOnce(&mut A, Args<'_, A>) + 'static>(
                &self,
                f: F,
            ) -> Result<(), winit::event_loop::EventLoopClosed<()>> {
                // Self is !Send+!Sync, so this must be in the main loop,
                // and the idle callback will be run in the same loop.
                let f = send_wrapper::SendWrapper::new(f);
                // If it fails, drop the message instead of returing it, because the
                // message is Send but the f inside is not.
                self.event_proxy
                    .run_idle(move |app, args| (f.take())(app, args))
                    .map_err(|_| winit::event_loop::EventLoopClosed(()))
            }
            /// Creates a `FutureBackCaller` for this application.
            pub fn future_back(&self) -> crate::FutureBackCaller<A> {
                fut::future_back_caller_new()
            }
        };
    }

    impl<A: Application> Args<'_, A> {
        /// Creates a `LocalProxy` that is `Clone` but not `Send`.
        pub fn local_proxy(&self) -> LocalProxy<A> {
            LocalProxy {
                event_proxy: self.event_proxy.clone(),
                pd: std::marker::PhantomData,
            }
        }
        /// Helper function to call `ping_user_input` in the main window.
        pub fn ping_user_input(&mut self) {
            self.window.ping_user_input();
        }
        local_proxy_impl! {}
    }

    impl<A: Application> LocalProxy<A> {
        /// Gets the inner real proxy, that is `Send`.
        pub fn event_proxy(&self) -> &EventLoopProxy<AppEvent<A>> {
            &self.event_proxy
        }
        local_proxy_impl! {}
    }

    /// Trait that connects a `UiBuilder` with an `AppHandler`.
    ///
    /// Implement this to manage the main loop of your application.
    pub trait Application: imgui::UiBuilder + Sized + 'static {
        /// The custom event for the `EventLoop`, usually `()`.
        type UserEvent: Send + 'static;
        /// The custom data for your `AppHandler`.
        type Data;

        /// The `EventFlags` for this application. Usually the default is ok.
        const EVENT_FLAGS: EventFlags = EventFlags::empty();

        /// The main window has been created, please create the application.
        fn new(args: Args<'_, Self>) -> Self;

        /// A new window event has been received.
        ///
        /// When this is called the event has already been fed to the ImGui
        /// context. The output is in `res`.
        /// The default impl will end the application when the window is closed.
        fn window_event(
            &mut self,
            args: Args<'_, Self>,
            _event: winit::event::WindowEvent,
            res: EventResult,
        ) {
            if res.window_closed {
                args.event_loop.exit();
            }
        }

        /// Advanced handling for window events.
        ///
        /// The default impl will pass the event to ImGui and then call `window_event`.
        fn window_event_full(&mut self, args: Args<'_, Self>, event: winit::event::WindowEvent) {
            let res = args.window.window_event(self, &event, Self::EVENT_FLAGS);
            self.window_event(args, event, res);
        }

        /// A device event has been received.
        ///
        /// This event is not handled in any way, just passed laong.
        fn device_event(
            &mut self,
            _args: Args<'_, Self>,
            _device_id: winit::event::DeviceId,
            _event: winit::event::DeviceEvent,
        ) {
        }

        /// A custom event has been received.
        fn user_event(&mut self, _args: Args<'_, Self>, _event: Self::UserEvent) {}

        /// Corresponds to `winit` `suspended`` function.
        fn suspended(&mut self, _args: Args<'_, Self>) {}

        /// Corresponds to `winit` `resumed` function.
        fn resumed(&mut self, _args: Args<'_, Self>) {}
    }

    /// The main event type to be used with `winit::EventLoop`.
    ///
    /// It is generic on the `Application` type.
    #[non_exhaustive]
    pub enum AppEvent<A: Application> {
        /// Calls `ping_user_input` on the main window.
        PingUserInput,
        /// Runs the given callback in the main loop idle step, with the regular arguments.
        #[allow(clippy::type_complexity)]
        RunIdle(Box<dyn FnOnce(&mut A, Args<'_, A>) + Send + Sync>),
        /// Runs the given callback in the main loop idle step, without arguments.
        RunIdleSimple(Box<dyn FnOnce() + Send + Sync>),
        /// Sends the custom user event.
        User(A::UserEvent),
    }

    impl<A: Application> std::fmt::Debug for AppEvent<A> {
        fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(fmt, "<AppEvent>")
        }
    }

    /// Helper trait to extend `winit::EventLoopProxy` with useful functions.
    pub trait EventLoopExt<A: Application> {
        /// Sends a `AppEvent::User` event.
        fn send_user(
            &self,
            u: A::UserEvent,
        ) -> Result<(), winit::event_loop::EventLoopClosed<AppEvent<A>>>;
        /// Sends a `AppEvent::PingUserInput` event.
        fn ping_user_input(&self) -> Result<(), winit::event_loop::EventLoopClosed<AppEvent<A>>>;
        /// Sends a `AppEvent::RunIdle` event.
        fn run_idle<F: FnOnce(&mut A, Args<'_, A>) + Send + Sync + 'static>(
            &self,
            f: F,
        ) -> Result<(), winit::event_loop::EventLoopClosed<AppEvent<A>>>;
    }

    impl<A: Application> EventLoopExt<A> for EventLoopProxy<AppEvent<A>> {
        fn send_user(
            &self,
            u: A::UserEvent,
        ) -> Result<(), winit::event_loop::EventLoopClosed<AppEvent<A>>> {
            self.send_event(AppEvent::User(u))
        }
        fn ping_user_input(&self) -> Result<(), winit::event_loop::EventLoopClosed<AppEvent<A>>> {
            self.send_event(AppEvent::PingUserInput)
        }
        fn run_idle<F: FnOnce(&mut A, Args<'_, A>) + Send + Sync + 'static>(
            &self,
            f: F,
        ) -> Result<(), winit::event_loop::EventLoopClosed<AppEvent<A>>> {
            self.send_event(AppEvent::RunIdle(Box::new(f)))
        }
    }

    /// Default implementation for `winit::application::ApplicationHandler`.
    ///
    /// The new `winit` requires an implementation of that trait to be able to use
    /// an `EventLoop`, and everything, including the main window, will be done from
    /// that.
    /// This type implements this trait and does some basic tasks:
    ///  * Creates the `MainWindowWithRenderer` object.
    ///  * Forwards the events to your application object.
    ///
    /// For that it requires that your application object implements `UiBuilder` and
    /// `Application`.
    ///
    /// If you have special needs you can skip this and write your own implementation
    /// of `winit::application::ApplicationHandler`.
    pub struct AppHandler<A: Application> {
        wattrs: WindowAttributes,
        event_proxy: EventLoopProxy<AppEvent<A>>,
        window: Option<MainWindowWithRenderer>,
        app: Option<A>,
        app_data: A::Data,
    }

    impl<A: Application> AppHandler<A> {
        /// Creates a new `AppHandler`.
        ///
        /// It creates an empty handler. It automatically creates an `EventLoopProxy`.
        pub fn new(event_loop: &EventLoop<AppEvent<A>>, app_data: A::Data) -> Self {
            AppHandler {
                wattrs: Window::default_attributes(),
                event_proxy: event_loop.create_proxy(),
                window: None,
                app: None,
                app_data,
            }
        }
        /// Sets the window attributes that will be used to create the main window.
        pub fn set_attributes(&mut self, wattrs: WindowAttributes) {
            self.wattrs = wattrs;
        }
        /// Gets the current window attributes.
        ///
        /// It returns a mutable reference, so you can modify it in-place, which is
        /// sometimes more convenient.
        pub fn attributes(&mut self) -> &mut WindowAttributes {
            &mut self.wattrs
        }
        /// Gets the custom data.
        pub fn data(&self) -> &A::Data {
            &self.app_data
        }
        /// Gets a mutable reference to the custom data.
        pub fn data_mut(&mut self) -> &mut A::Data {
            &mut self.app_data
        }
        /// Gets the inner app object.
        pub fn app(&self) -> Option<&A> {
            self.app.as_ref()
        }
        /// Gets a mutable reference to the inner app object.
        pub fn app_mut(&mut self) -> Option<&mut A> {
            self.app.as_mut()
        }
        /// Extracts the inner values.
        ///
        /// You may need this after the main loop has finished to get
        /// the result of your program execution.
        pub fn into_inner(self) -> (Option<A>, A::Data) {
            (self.app, self.app_data)
        }

        /// Gets the inner `EventLoopProxy`.
        pub fn event_proxy(&self) -> &EventLoopProxy<AppEvent<A>> {
            &self.event_proxy
        }
    }

    impl<A> winit::application::ApplicationHandler<AppEvent<A>> for AppHandler<A>
    where
        A: Application,
    {
        fn suspended(&mut self, event_loop: &ActiveEventLoop) {
            let Some(window) = self.window.as_mut() else {
                return;
            };
            if let Some(app) = &mut self.app {
                let args = Args {
                    window,
                    event_loop,
                    event_proxy: &self.event_proxy,
                    data: &mut self.app_data,
                };
                app.suspended(args);
            }
            self.window = None;
        }
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            let main_window = MainWindow::new(event_loop, self.wattrs.clone()).unwrap();
            let mut window = MainWindowWithRenderer::new(main_window);

            let args = Args {
                window: &mut window,
                event_loop,
                event_proxy: &self.event_proxy,
                data: &mut self.app_data,
            };
            match &mut self.app {
                None => self.app = Some(A::new(args)),
                Some(app) => app.resumed(args),
            }
            self.window = Some(window);
        }
        fn window_event(
            &mut self,
            event_loop: &ActiveEventLoop,
            window_id: winit::window::WindowId,
            event: winit::event::WindowEvent,
        ) {
            let (Some(window), Some(app)) = (self.window.as_mut(), self.app.as_mut()) else {
                return;
            };
            let w = window.main_window();
            if w.window().id() != window_id {
                return;
            }

            let args = Args {
                window,
                event_loop,
                event_proxy: &self.event_proxy,
                data: &mut self.app_data,
            };
            app.window_event_full(args, event);
        }
        fn device_event(
            &mut self,
            event_loop: &ActiveEventLoop,
            device_id: winit::event::DeviceId,
            event: winit::event::DeviceEvent,
        ) {
            let (Some(window), Some(app)) = (self.window.as_mut(), self.app.as_mut()) else {
                return;
            };
            let args = Args {
                window,
                event_loop,
                event_proxy: &self.event_proxy,
                data: &mut self.app_data,
            };
            app.device_event(args, device_id, event);
        }
        fn user_event(&mut self, event_loop: &ActiveEventLoop, event: AppEvent<A>) {
            let (Some(window), Some(app)) = (self.window.as_mut(), self.app.as_mut()) else {
                return;
            };
            let args = Args {
                window,
                event_loop,
                event_proxy: &self.event_proxy,
                data: &mut self.app_data,
            };

            match event {
                AppEvent::PingUserInput => window.ping_user_input(),
                AppEvent::RunIdle(f) => f(app, args),
                AppEvent::RunIdleSimple(f) => fut::future_back_caller_prepare((app, args), f),
                AppEvent::User(uevent) => app.user_event(args, uevent),
            }
        }
        fn new_events(&mut self, _event_loop: &ActiveEventLoop, _cause: winit::event::StartCause) {
            let Some(window) = self.window.as_mut() else {
                return;
            };
            window.new_events();
        }
        fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
            let Some(window) = self.window.as_mut() else {
                return;
            };
            window.about_to_wait();
        }
    }
}

#[cfg(feature = "main-window")]
pub use main_window::*;
