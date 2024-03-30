use crate::conv::{to_imgui_button, to_imgui_key};
use easy_imgui as imgui;
use easy_imgui::{BackendFlags, ConfigFlags};
use easy_imgui_sys::*;
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::error::ExternalError;
use winit::event::Ime::Commit;
use winit::event::WindowEvent::{
    CursorLeft, CursorMoved, Focused, Ime, KeyboardInput, ModifiersChanged, MouseInput, MouseWheel,
    Resized, ScaleFactorChanged,
};
use winit::event::{Event, WindowEvent};
use winit::keyboard::KeyCode::{
    AltLeft, AltRight, ControlLeft, ControlRight, ShiftLeft, ShiftRight, SuperLeft, SuperRight,
};
use winit::keyboard::PhysicalKey;
use winit::window::{CursorIcon, Window};

#[derive(Debug)]
pub struct WinitState {
    hidpi_factor: f64,
}

impl WinitState {
    pub fn init() -> WinitState {
        Self { hidpi_factor: 1.0 }
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

    fn handle_window_event(&mut self, io: &mut ImGuiIO, window: &Window, event: &WindowEvent) {
        match event {
            Resized(physical_size) => {
                let logical_size = physical_size.to_logical(window.scale_factor());
                let logical_size = self.scale_size_from_winit(window, logical_size);
                io.DisplaySize =
                    imgui::im_vec2(logical_size.width as f32, logical_size.height as f32);
            }
            ScaleFactorChanged { scale_factor, .. } => {
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
                for c in text.chars() {
                    ImGuiIO_AddInputCharacter(io, c as u32);
                }
            },
            CursorMoved { position, .. } => unsafe {
                let scale = window.scale_factor();
                let position = position.to_logical(scale);
                ImGuiIO_AddMousePosEvent(io, position.x, position.y);
            },
            MouseWheel {
                delta,
                phase: winit::event::TouchPhase::Moved,
                ..
            } => {
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
                if let Some(btn) = to_imgui_button(*button) {
                    let pressed = *state == winit::event::ElementState::Pressed;
                    ImGuiIO_AddMouseButtonEvent(io, btn.bits(), pressed);
                }
            },
            CursorLeft { .. } => unsafe {
                ImGuiIO_AddMousePosEvent(io, f32::MAX, f32::MAX);
            },
            Focused(focused) => unsafe {
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
        // todo
    }
}
