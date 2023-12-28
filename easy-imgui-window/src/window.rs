use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::{Instant, Duration};
use glutin_winit::DisplayBuilder;
use winit::keyboard::PhysicalKey;
use winit::event::Ime::Commit;
use winit::{window::{Window, CursorIcon, WindowBuilder}, event::Event, dpi::{PhysicalSize, LogicalSize, Pixel, PhysicalPosition, LogicalPosition}, event_loop::EventLoopWindowTarget};
use easy_imgui_sys::*;
use easy_imgui as imgui;
use imgui::{mint, Vector2};
use glutin::{prelude::*, config::{Config, ConfigTemplateBuilder}, display::GetGlDisplay, surface::{SurfaceAttributesBuilder, WindowSurface, Surface}, context::{ContextAttributesBuilder, ContextApi, PossiblyCurrentContext}};
use raw_window_handle::HasRawWindowHandle;
use anyhow::{Result, anyhow};
use easy_imgui_renderer::{Renderer, glow};
use crate::conv::{from_imgui_cursor, to_imgui_key, to_imgui_button};

/// This type represents a `winit` window and an OpenGL context.
pub struct MainWindow {
    gl_context: PossiblyCurrentContext,
    // The surface must be dropped before the window.
    surface: Surface<WindowSurface>,
    window: Window,
}

/// This is a [`MainWindow`] plus a [`Renderer`]. It is the ultimate `easy-imgui` object.
pub struct MainWindowWithRenderer {
    main_window: MainWindow,
    renderer: Renderer,

    idle_time: Duration,
    idle_frame_count: u32,

    last_frame: Instant,
    last_input_time: Instant,
    last_input_frame: u32,
    current_cursor: Option<CursorIcon>,
}

impl MainWindow {
    /// Creates a `MainWindow` with default values.
    pub fn new<EventUserType>(event_loop: &EventLoopWindowTarget<EventUserType>, title: &str) -> Result<MainWindow> {
        // For standard UI, we need as few fancy things as available
        let score = |c: &Config| (c.num_samples(), c.depth_size(), c.stencil_size());
        Self::with_gl_chooser(
            event_loop,
            title,
            |cfg1, cfg2| {
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
    pub fn with_gl_chooser<EventUserType>(event_loop: &EventLoopWindowTarget<EventUserType>, title: &str, f_choose_cfg: impl FnMut(Config, Config) -> Config) -> Result<MainWindow> {
        let window_builder = WindowBuilder::new();
        let template = ConfigTemplateBuilder::new()
            .prefer_hardware_accelerated(Some(true))
            .with_depth_size(0)
            .with_stencil_size(0)
        ;

        let display_builder = DisplayBuilder::new()
            .with_window_builder(Some(window_builder));

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
        let context_attributes = ContextAttributesBuilder::new()
            .build(raw_window_handle);
        let fallback_context_attributes = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::Gles(None))
            .build(raw_window_handle);

        let mut not_current_gl_context = Some(unsafe {
            gl_display
                .create_context(&gl_config, &context_attributes)
                .or_else(|_| {
                    gl_display
                        .create_context(&gl_config, &fallback_context_attributes)
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

        let surface = unsafe { gl_config.display().create_window_surface(&gl_config, &attrs)? };
        let gl_context = not_current_gl_context
            .take()
            .unwrap()
            .make_current(&surface)?;

        // Enable v-sync to avoid consuming too much CPU
        let _ = surface.set_swap_interval(&gl_context, glutin::surface::SwapInterval::Wait(NonZeroU32::new(1).unwrap()));

        Ok(MainWindow {
            gl_context,
            window,
            surface,
        })
    }
    pub fn glutin_context(&self) -> &glutin::context::PossiblyCurrentContext {
        &self.gl_context
    }
    /// Creates a new `glow` OpenGL context withfor this window and the selected configuration.
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
    pub fn to_physical_size<X: Pixel, Y: Pixel>(&self, size: LogicalSize<X>) -> PhysicalSize<Y> {
        let scale = self.window.scale_factor();
        size.to_physical(scale)
    }
    /// Converts the given physical position to a logical position, using the window scale factor.
    pub fn to_logical_pos<X: Pixel, Y: Pixel>(&self, pos: PhysicalPosition<X>) -> LogicalPosition<Y> {
        let scale = self.window.scale_factor();
        pos.to_logical(scale)
    }
    /// Converts the given logical position to a physical position, using the window scale factor.
    pub fn to_physical_pos<X: Pixel, Y: Pixel>(&self, pos: LogicalPosition<X>) -> PhysicalPosition<Y> {
        let scale = self.window.scale_factor();
        pos.to_physical(scale)
    }
}

impl MainWindowWithRenderer {
    /// Creates a new [`Renderer`] and attaches it to the given window.
    pub fn new(main_window: MainWindow) -> MainWindowWithRenderer {
        let gl = main_window.create_gl_context();
        let renderer = Renderer::new(Rc::new(gl)).unwrap();
        Self::new_with_renderer(main_window, renderer)
    }
    /// Attaches the given window an renderer together.
    pub fn new_with_renderer(main_window: MainWindow, mut renderer: Renderer) -> MainWindowWithRenderer {
        let size = main_window.window.inner_size();
        let scale = main_window.window.scale_factor();
        let l_size = size.to_logical::<f32>(scale);
        renderer.set_size(Vector2::from(mint::Vector2::from(l_size)), scale as f32);

        clipboard::maybe_setup_clipboard();
        let now = Instant::now();

        MainWindowWithRenderer {
            main_window,
            renderer,
            idle_time: Duration::from_secs(1),
            idle_frame_count: 60,
            last_frame: now,
            last_input_time: now,
            last_input_frame: 0,
            current_cursor: Some(CursorIcon::Default),
        }
    }
    /// Sets the time after which the UI will stop rendering, if there is no user input.
    pub fn set_idle_time(&mut self, time: Duration) {
        self.idle_time = time;
    }
    /// Sets the frame count after which the UI will stop rendering, if there is no user input.
    ///
    /// Note that by default V-Sync is enabled, and that will affect the frame rate.
    pub fn set_idle_frame_count(&mut self, frame_count: u32) {
        self.idle_frame_count = frame_count;
    }
    /// Gets a reference to the inner renderer.
    pub fn renderer(&mut self) -> &mut Renderer {
        &mut self.renderer
    }
    /// Gets a reference to the inner window.
    pub fn main_window(&mut self) -> &mut MainWindow {
        &mut self.main_window
    }
    /// Forces a rebuild of the UI.
    ///
    /// By default the window will stop rendering the UI after a while without user input. Use this
    /// function to force a redraw because of some external factor.
    pub fn ping_user_input(&mut self) {
        self.last_input_time = Instant::now();
        self.last_input_frame = 0;
        self.main_window.window.request_redraw();
    }
    /// The main event function, to be called from your event loop.
    ///
    /// It returns [`std::ops::ControlFlow::Break`] for the event [`winit::event::WindowEvent::CloseRequested`] as a convenience. You can
    /// use it to break the main loop, or ignore it, as you see fit.
    #[must_use]
    pub fn do_event<EventUserType>(&mut self, app: &mut impl imgui::UiBuilder, event: &Event<EventUserType>, _w: &EventLoopWindowTarget<EventUserType>) -> std::ops::ControlFlow<()> {
        match event {
            Event::NewEvents(_) => {
                let now = Instant::now();
                unsafe {
                    let io = &mut *ImGui_GetIO();
                    io.DeltaTime = now.duration_since(self.last_frame).as_secs_f32();
                }
                self.last_frame = now;
            }
            Event::AboutToWait => {
                unsafe {
                    let io = &*ImGui_GetIO();
                    if io.WantSetMousePos {
                        let pos = io.MousePos;
                        let pos = winit::dpi::LogicalPosition { x: pos.x, y: pos.y };
                        let _ = self.main_window.window.set_cursor_position(pos);
                    }
                }
                let now = Instant::now();
                // If the mouse is down, redraw all the time, maybe the user is dragging.
                let mouse = unsafe { ImGui_IsAnyMouseDown() };
                self.last_input_frame += 1;
                if mouse || now.duration_since(self.last_input_time) < self.idle_time || self.last_input_frame < self.idle_frame_count {
                    // No need to call set_control_flow(): doing a redraw will force extra Poll.
                    // Not doing it will default to Wait.
                    self.main_window.window.request_redraw();
                }
            }
            Event::WindowEvent {
                window_id,
                event
            } if *window_id == self.main_window.window.id() => {
                use winit::event::WindowEvent::*;

                match event {
                    CloseRequested => {
                        return std::ops::ControlFlow::Break(());
                    }
                    RedrawRequested => {
                        unsafe {
                            let io = &*ImGui_GetIO();
                            let config_flags = imgui::ConfigFlags::from_bits_truncate(io.ConfigFlags);
                            if !config_flags.contains(imgui::ConfigFlags::NoMouseCursorChange) {
                                let cursor = if io.MouseDrawCursor {
                                    None
                                } else {
                                    let cursor = imgui::MouseCursor::from_bits(ImGui_GetMouseCursor())
                                        .unwrap_or(imgui::MouseCursor::Arrow);
                                    from_imgui_cursor(cursor)
                                };
                                if cursor != self.current_cursor {
                                    match cursor {
                                        None => self.main_window.window.set_cursor_visible(false),
                                        Some(c) => {
                                            self.main_window.window.set_cursor_icon(c);
                                            self.main_window.window.set_cursor_visible(true);
                                        }
                                    }
                                    self.current_cursor = cursor;
                                }
                            }
                            self.renderer.do_frame(app);
                        }
                        self.main_window.window.pre_present_notify();
                        self.main_window.surface.swap_buffers(&self.main_window.gl_context).unwrap();
                    }
                    Resized(size) => {
                        self.ping_user_input();
                        // GL surface in physical pixels, imgui in logical
                        if let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) {
                            self.main_window.surface.resize(&self.main_window.gl_context, w, h);
                        }
                        unsafe {
                            let io = &mut *ImGui_GetIO();
                            let size = self.main_window.to_logical_size::<_, f32>(*size);
                            let size = Vector2::from(mint::Vector2::from(size));
                            io.DisplaySize = imgui::v2_to_im(size);
                        }
                    }
                    ScaleFactorChanged { scale_factor, .. } => {
                        self.ping_user_input();
                        let scale_factor = *scale_factor as f32;
                        unsafe {
                            let io = &mut *ImGui_GetIO();
                            let old_scale_factor = io.DisplayFramebufferScale.x;
                            if io.MousePos.x.is_finite() && io.MousePos.y.is_finite() {
                                io.MousePos.x *= scale_factor / old_scale_factor;
                                io.MousePos.y *= scale_factor / old_scale_factor;
                            }
                        }
                        let size = self.renderer.size();
                        self.renderer.set_size(size, scale_factor);
                    }
                    ModifiersChanged(mods) => {
                        self.ping_user_input();
                        unsafe {
                            let io = &mut *ImGui_GetIO();
                            ImGuiIO_AddKeyEvent(io, ImGuiKey(imgui::Key::ModCtrl.bits()), mods.state().control_key());
                            ImGuiIO_AddKeyEvent(io, ImGuiKey(imgui::Key::ModShift.bits()), mods.state().shift_key());
                            ImGuiIO_AddKeyEvent(io, ImGuiKey(imgui::Key::ModAlt.bits()), mods.state().alt_key());
                            ImGuiIO_AddKeyEvent(io, ImGuiKey(imgui::Key::ModSuper.bits()), mods.state().super_key());
                        }
                    }
                    KeyboardInput {
                        event: winit::event::KeyEvent {
                            physical_key,
                            text,
                            state,
                            ..
                        },
                        is_synthetic: false,
                        ..
                    } => {
                        self.ping_user_input();
                        let pressed = *state == winit::event::ElementState::Pressed;
                        if let Some(key) = to_imgui_key(*physical_key) {
                            unsafe {
                                let io = &mut *ImGui_GetIO();
                                ImGuiIO_AddKeyEvent(io, ImGuiKey(key.bits()), pressed);

                                use winit::keyboard::KeyCode::*;
                                if let PhysicalKey::Code(keycode) = physical_key {
                                    let kmod = match keycode {
                                        ControlLeft | ControlRight => Some(imgui::Key::ModCtrl),
                                        ShiftLeft | ShiftRight => Some(imgui::Key::ModShift),
                                        AltLeft | AltRight => Some(imgui::Key::ModAlt),
                                        SuperLeft | SuperRight => Some(imgui::Key::ModSuper),
                                        _ => None
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
                                    let io = &mut *ImGui_GetIO();
                                    for c in text.chars() {
                                        ImGuiIO_AddInputCharacter(io, c as u32);
                                    }
                                }
                            }
                        }
                    }
                    Ime(Commit(text)) => {
                        self.ping_user_input();
                        unsafe {
                            let io = &mut *ImGui_GetIO();
                            for c in text.chars() {
                                ImGuiIO_AddInputCharacter(io, c as u32);
                            }
                        }
                    }
                    CursorMoved { position, .. } => {
                        self.ping_user_input();
                        unsafe {
                            let io = &mut *ImGui_GetIO();
                            let position = self.main_window.to_logical_pos(*position);
                            ImGuiIO_AddMousePosEvent(io, position.x, position.y);
                        }
                    }
                    MouseWheel {
                        delta,
                        phase: winit::event::TouchPhase::Moved,
                        ..
                    } => {
                        self.ping_user_input();
                        let io = unsafe {
                            &mut *ImGui_GetIO()
                        };
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
                        self.ping_user_input();
                        unsafe {
                            let io = &mut *ImGui_GetIO();
                            if let Some(btn) = to_imgui_button(*button) {
                                let pressed = *state == winit::event::ElementState::Pressed;
                                ImGuiIO_AddMouseButtonEvent(io, btn.bits(), pressed);
                            }
                        }
                    }
                    CursorLeft { .. } => {
                        self.ping_user_input();
                        unsafe {
                            let io = &mut *ImGui_GetIO();
                            ImGuiIO_AddMousePosEvent(io, f32::MAX, f32::MAX);
                        }
                    }
                    Focused(focused) => {
                        self.ping_user_input();
                        unsafe {
                            let io = &mut *ImGui_GetIO();
                            ImGuiIO_AddFocusEvent(io, *focused);
                        }
                    }
                    _ => {}
                }
            }
            _ => { }
        }
        std::ops::ControlFlow::Continue(())
    }
}

#[cfg(not(feature="clipboard"))]
mod clipboard {
    pub fn maybe_setup_clipboard() { }
}
#[cfg(feature="clipboard")]
mod clipboard {
    use std::ffi::{CString, CStr, c_void, c_char};

    pub fn maybe_setup_clipboard() {
        if let Ok(ctx) = arboard::Clipboard::new() {
            let clip = MyClipboard {
                ctx,
                text: CString::default(),
            };
            unsafe {
                let io = &mut *easy_imgui_sys::ImGui_GetIO();
                io.ClipboardUserData = Box::into_raw(Box::new(clip)) as *mut c_void;
                io.SetClipboardTextFn = Some(set_clipboard_text);
                io.GetClipboardTextFn = Some(get_clipboard_text);
            }
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
