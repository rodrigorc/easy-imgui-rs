use crate::conv::{from_imgui_cursor, to_imgui_button, to_imgui_key};
use easy_imgui as imgui;
use easy_imgui_sys::*;
use std::time::{Duration, Instant};
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::error::ExternalError;
use winit::event::Ime::Commit;
use winit::event::WindowEvent::{
    CursorLeft, CursorMoved, Focused, Ime, KeyboardInput, ModifiersChanged, MouseInput, MouseWheel,
    RedrawRequested, Resized, ScaleFactorChanged,
};
use winit::event::{Event, WindowEvent};
use winit::keyboard::PhysicalKey;
use winit::window::{CursorIcon, Window};

#[derive(Debug)]
pub struct WinitState {
    hidpi_factor: f64,
    status: WindowStatus,
}

#[derive(Debug)]
pub struct WindowStatus {
    last_frame: Instant,
    current_cursor: Option<CursorIcon>,

    idle_time: Duration,
    idle_frame_count: u32,
    last_input_time: Instant,
    last_input_frame: u32,
}

impl Default for WindowStatus {
    fn default() -> WindowStatus {
        let now = Instant::now();
        WindowStatus {
            last_frame: now,
            current_cursor: Some(CursorIcon::Default),

            idle_time: Duration::from_secs(1),
            idle_frame_count: 60,
            last_input_time: now,
            last_input_frame: 0,
        }
    }
}

impl WindowStatus {
    pub fn set_idle_time(&mut self, time: Duration) {
        self.idle_time = time;
    }
    pub fn set_idle_frame_count(&mut self, frame_count: u32) {
        self.idle_frame_count = frame_count;
    }
    pub fn ping_user_input(&mut self) {
        self.last_input_time = Instant::now();
        self.last_input_frame = 0;
    }
}

impl WinitState {
    pub fn init() -> WinitState {
        Self {
            hidpi_factor: 1.0,
            status: Default::default(),
        }
    }

    pub fn attach_window(&mut self, io: &mut ImGuiIO, window: &Window) {
        let hidpi_factor = window.scale_factor();
        self.hidpi_factor = hidpi_factor;
        io.DisplayFramebufferScale = imgui::im_vec2(hidpi_factor as f32, hidpi_factor as f32);
        let logical_size = window.inner_size().to_logical(hidpi_factor);
        let logical_size = self.scale_size_from_winit(window, logical_size);
        io.DisplaySize = imgui::im_vec2(logical_size.width as f32, logical_size.height as f32);
    }

    pub fn scale_size_from_winit(
        &self,
        window: &Window,
        logical_size: LogicalSize<f64>,
    ) -> LogicalSize<f64> {
        logical_size
            .to_physical::<f64>(window.scale_factor())
            .to_logical(self.hidpi_factor)
    }

    pub fn hidpi_factor(&self) -> f64 {
        self.hidpi_factor
    }

    /// Scales a logical position coming from winit.
    pub fn scale_pos_from_winit(
        &self,
        window: &Window,
        logical_pos: LogicalPosition<f64>,
    ) -> LogicalPosition<f64> {
        logical_pos
            .to_physical::<f64>(window.scale_factor())
            .to_logical(self.hidpi_factor)
    }

    /// Scales a logical position for winit.
    pub fn scale_pos_for_winit(
        &self,
        window: &Window,
        logical_pos: LogicalPosition<f64>,
    ) -> LogicalPosition<f64> {
        logical_pos
            .to_physical::<f64>(self.hidpi_factor)
            .to_logical(window.scale_factor())
    }

    pub fn handle_event<T>(&mut self, io: &mut ImGuiIO, window: &Window, event: &Event<T>) {
        match *event {
            Event::WindowEvent {
                window_id,
                ref event,
            } if window_id == window.id() => {
                self.handle_window_event(io, window, event);
            }
            Event::NewEvents(_) => {
                let now = Instant::now();
                io.DeltaTime = now.duration_since(self.status.last_frame).as_secs_f32();
                self.status.last_frame = now;
            }
            Event::AboutToWait => {
                self.status.last_input_frame += 1;
            }
            // Track key release events outside our window. If we don't do this,
            // we might never see the release event if some other window gets focus.
            // Event::DeviceEvent {
            //     event:
            //         DeviceEvent::Key(RawKeyEvent {
            //             physical_key,
            //             state: ElementState::Released,
            //         }),
            //     ..
            // } => {
            //     if let Some(key) = to_imgui_key(key) {
            //         io.add_key_event(key, false);
            //     }
            // }
            _ => (),
        }
    }

    pub fn handle_window_event(&mut self, io: &mut ImGuiIO, window: &Window, event: &WindowEvent) {
        match event {
            RedrawRequested => self.prepare_render(io, window),
            Resized(physical_size) => {
                self.status.ping_user_input();
                let logical_size = physical_size.to_logical(window.scale_factor());
                let logical_size = self.scale_size_from_winit(window, logical_size);
                io.DisplaySize =
                    imgui::im_vec2(logical_size.width as f32, logical_size.height as f32);
            }
            ScaleFactorChanged { scale_factor, .. } => {
                self.status.ping_user_input();
                let scale_factor = *scale_factor;
                let old_scale_factor = io.DisplayFramebufferScale.x as f64;
                if io.MousePos.x.is_finite() && io.MousePos.y.is_finite() {
                    let scale = (scale_factor / old_scale_factor) as f32;
                    io.MousePos.x *= scale;
                    io.MousePos.y *= scale;
                }
                self.hidpi_factor = scale_factor;
                io.DisplayFramebufferScale =
                    imgui::im_vec2(scale_factor as f32, scale_factor as f32);
                let logical_size = window.inner_size().to_logical(scale_factor);
                let logical_size = self.scale_size_from_winit(window, logical_size);
                io.DisplaySize =
                    imgui::im_vec2(logical_size.width as f32, logical_size.height as f32);
            }
            ModifiersChanged(mods) => unsafe {
                self.status.ping_user_input();
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
            },
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
                self.status.ping_user_input();
                let pressed = *state == winit::event::ElementState::Pressed;
                if let Some(key) = to_imgui_key(*physical_key) {
                    unsafe {
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
                            for c in text.chars() {
                                ImGuiIO_AddInputCharacter(io, c as u32);
                            }
                        }
                    }
                }
            }
            Ime(Commit(text)) => unsafe {
                self.status.ping_user_input();
                for c in text.chars() {
                    ImGuiIO_AddInputCharacter(io, c as u32);
                }
            },
            CursorMoved { position, .. } => unsafe {
                self.status.ping_user_input();
                let scale = window.scale_factor();
                let position = position.to_logical(scale);
                ImGuiIO_AddMousePosEvent(io, position.x, position.y);
            },
            MouseWheel {
                delta,
                phase: winit::event::TouchPhase::Moved,
                ..
            } => {
                self.status.ping_user_input();
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
            MouseInput { state, button, .. } => unsafe {
                self.status.ping_user_input();
                if let Some(btn) = to_imgui_button(*button) {
                    let pressed = *state == winit::event::ElementState::Pressed;
                    ImGuiIO_AddMouseButtonEvent(io, btn.bits(), pressed);
                }
            },
            CursorLeft { .. } => unsafe {
                self.status.ping_user_input();
                ImGuiIO_AddMousePosEvent(io, f32::MAX, f32::MAX);
            },
            Focused(focused) => unsafe {
                self.status.ping_user_input();
                ImGuiIO_AddFocusEvent(io, *focused);
            },
            _ => {}
        }
    }

    pub fn prepare_frame(&self, io: &mut ImGuiIO, window: &Window) -> Result<(), ExternalError> {
        if io.WantSetMousePos {
            let logical_pos = self.scale_pos_for_winit(
                window,
                LogicalPosition::new(f64::from(io.MousePos.x), f64::from(io.MousePos.y)),
            );
            window.set_cursor_position(logical_pos)
        } else {
            Ok(())
        }
    }

    pub fn prepare_render(&mut self, io: &ImGuiIO, window: &Window) {
        let config_flags = imgui::ConfigFlags::from_bits_truncate(io.ConfigFlags);
        if !config_flags.contains(imgui::ConfigFlags::NoMouseCursorChange) {
            let cursor = if io.MouseDrawCursor {
                None
            } else {
                let cursor = unsafe {
                    imgui::MouseCursor::from_bits(ImGui_GetMouseCursor())
                        .unwrap_or(imgui::MouseCursor::Arrow)
                };
                from_imgui_cursor(cursor)
            };
            if cursor != self.status.current_cursor {
                self.set_cursor(window, cursor);
                self.status.current_cursor = cursor;
            }
        }
    }

    fn set_cursor(&self, w: &Window, cursor: Option<CursorIcon>) {
        match cursor {
            None => w.set_cursor_visible(false),
            Some(c) => {
                w.set_cursor_icon(c);
                w.set_cursor_visible(true);
            }
        }
    }
}

impl WinitState {
    pub fn set_idle_time(&mut self, time: Duration) {
        self.status.idle_time = time;
    }
    pub fn set_idle_frame_count(&mut self, frame_count: u32) {
        self.status.idle_frame_count = frame_count;
    }
    pub fn ping_user_input(&mut self) {
        self.status.last_input_time = Instant::now();
        self.status.last_input_frame = 0;
    }

    pub fn status(&self) -> &WindowStatus {
        &self.status
    }

    pub fn status_mut(&mut self) -> &mut WindowStatus {
        &mut self.status
    }

    pub fn can_request_redraw(&self) -> bool {
        let status = &self.status;
        let now = Instant::now();
        now.duration_since(status.last_input_time) < status.idle_time
            || status.last_input_frame < status.idle_frame_count
    }
}
