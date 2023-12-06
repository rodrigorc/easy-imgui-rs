use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::{Instant, Duration};
use glutin_winit::DisplayBuilder;
use winit::keyboard::PhysicalKey;
use winit::event::Ime::Commit;
use winit::{window::{Window, CursorIcon, WindowBuilder}, event::Event, dpi::{PhysicalSize, LogicalSize, Pixel, PhysicalPosition, LogicalPosition}, event_loop::EventLoopWindowTarget};
use easy_imgui_sys::*;
use easy_imgui as imgui;
use glutin::{prelude::*, config::{Config, ConfigTemplateBuilder}, display::GetGlDisplay, surface::{SurfaceAttributesBuilder, WindowSurface, Surface}, context::{ContextAttributesBuilder, ContextApi, PossiblyCurrentContext}};
use raw_window_handle::HasRawWindowHandle;
use anyhow::{Result, anyhow};
use easy_imgui_renderer::{Renderer, glow};
use crate::conv::{from_imgui_cursor, to_imgui_key, to_imgui_button};

struct MainWindowStatus {
    last_frame: Instant,
    last_input_time: Instant,
    last_input_frame: u32,
    current_cursor: Option<CursorIcon>,
}

impl Default for MainWindowStatus {
    fn default() -> MainWindowStatus {
        let now = Instant::now();
        // The main loop will keep on rendering for some ms or some frames after the last input,
        // whatever is longer. The frames are needed in case a file operation takes a lot of time,
        // we want at least a render just after that.
        MainWindowStatus {
            last_frame: now,
            last_input_time: now,
            last_input_frame: 0,
            current_cursor: Some(CursorIcon::Default),
        }
    }
}

pub struct MainWindow {
    gl_context: PossiblyCurrentContext,
    // The surface must be dropped before the window.
    surface: Surface<WindowSurface>,
    window: Window,
}

pub struct MainWindowWithRenderer {
    main_window: MainWindow,
    renderer: Renderer,
    status: MainWindowStatus,
}

impl MainWindow {
    pub fn new<EventUserType>(event_loop: &EventLoopWindowTarget<EventUserType>, title: &str) -> Result<MainWindow> {
        Self::with_gl_chooser(
            event_loop,
            title,
            |cfg1, cfg2| {
                // For standard UI, we need as few fancy things as available
                let t = |c: &Config| (c.num_samples(), c.depth_size(), c.stencil_size());
                if t(&cfg2) < t(&cfg1) {
                    cfg2
                } else {
                    cfg1
                }
            })
    }
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
    pub fn create_gl_context(&self) -> Rc<glow::Context> {
        let dsp = self.gl_context.display();
        let gl = unsafe { glow::Context::from_loader_function_cstr(|s| dsp.get_proc_address(s)) };
        Rc::new(gl)
    }
    pub fn window(&self) -> &Window {
        &self.window
    }
    pub fn to_logical_size<X: Pixel, Y: Pixel>(&self, size: PhysicalSize<X>) -> LogicalSize<Y> {
        let scale = self.window.scale_factor();
        size.to_logical(scale)
    }
    #[allow(dead_code)]
    pub fn to_physical_size<X: Pixel, Y: Pixel>(&self, size: LogicalSize<X>) -> PhysicalSize<Y> {
        let scale = self.window.scale_factor();
        size.to_physical(scale)
    }
    pub fn to_logical_pos<X: Pixel, Y: Pixel>(&self, pos: PhysicalPosition<X>) -> LogicalPosition<Y> {
        let scale = self.window.scale_factor();
        pos.to_logical(scale)
    }
    #[allow(dead_code)]
    pub fn to_physical_pos<X: Pixel, Y: Pixel>(&self, pos: LogicalPosition<X>) -> PhysicalPosition<Y> {
        let scale = self.window.scale_factor();
        pos.to_physical(scale)
    }
}

impl MainWindowWithRenderer {
    pub fn new(main_window: MainWindow) -> MainWindowWithRenderer {
        let gl = main_window.create_gl_context();
        let renderer = Renderer::new(gl).unwrap();
        Self::new_with_renderer(main_window, renderer)
    }
    pub fn new_with_renderer(main_window: MainWindow, mut renderer: Renderer) -> MainWindowWithRenderer {
        let size = main_window.window.inner_size();
        let scale = main_window.window.scale_factor();
        let l_size = size.to_logical::<f32>(scale);
        renderer.set_size(l_size.into(), scale as f32);

        clipboard::maybe_setup_clipboard();

        MainWindowWithRenderer {
            main_window,
            renderer,
            status: MainWindowStatus::default(),
        }
    }
    pub fn renderer(&mut self) -> &mut Renderer {
        &mut self.renderer
    }
    pub fn main_window(&mut self) -> &mut MainWindow {
        &mut self.main_window
    }
    pub fn ping_user_input(&mut self) {
        self.status.last_input_time = Instant::now();
        self.status.last_input_frame = 0;
        self.main_window.window.request_redraw();
    }
    #[must_use]
    pub fn do_event<EventUserType>(&mut self, app: &mut impl imgui::UiBuilder, event: &Event<EventUserType>, _w: &EventLoopWindowTarget<EventUserType>) -> std::ops::ControlFlow<()> {
        match event {
            Event::NewEvents(_) => {
                let now = Instant::now();
                unsafe {
                    let io = &mut *ImGui_GetIO();
                    io.DeltaTime = now.duration_since(self.status.last_frame).as_secs_f32();
                }
                self.status.last_frame = now;
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
                self.status.last_input_frame += 1;
                if mouse || now.duration_since(self.status.last_input_time) < Duration::from_millis(1000) || self.status.last_input_frame < 60 {
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
                                if cursor != self.status.current_cursor {
                                    match cursor {
                                        None => self.main_window.window.set_cursor_visible(false),
                                        Some(c) => {
                                            self.main_window.window.set_cursor_icon(c);
                                            self.main_window.window.set_cursor_visible(true);
                                        }
                                    }
                                    self.status.current_cursor = cursor;
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
                            let size: Vector2 = size.into();
                            io.DisplaySize = size.into();
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
