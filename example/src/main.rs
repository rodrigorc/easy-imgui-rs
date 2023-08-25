#![allow(unused_imports, unused_variables, unused_mut)]

use std::{num::NonZeroU32, ffi::CString, time::{Instant, Duration}, rc::Rc, mem::MaybeUninit};
use cstr::cstr;

use glutin_winit::DisplayBuilder;
use glutin::{prelude::*, config::{ConfigTemplateBuilder, Config}, display::GetGlDisplay, context::{ContextAttributesBuilder, ContextApi}, surface::{SurfaceAttributesBuilder, WindowSurface, Surface}};
use raw_window_handle::HasRawWindowHandle;
use winit::{event::{self, VirtualKeyCode}, event_loop::{EventLoopBuilder, ControlFlow}, window::{WindowBuilder, Window}, dpi::PhysicalSize};

use dear_imgui_sys::*;
use dear_imgui as imgui;

mod glr;
mod w;

static KARLA_TTF: &[u8] = include_bytes!("Karla-Regular.ttf");
static UBUNTU_TTF: &[u8] = include_bytes!("Ubuntu-R.ttf");

fn main() {
    let event_loop = EventLoopBuilder::new().build();

    // We render to FBOs so we do not need depth, stencil buffers or anything fancy.
    let window_builder = WindowBuilder::new();
    let template = ConfigTemplateBuilder::new()
        .prefer_hardware_accelerated(Some(true))
        .with_depth_size(0)
        .with_stencil_size(0)
    ;

    let display_builder = DisplayBuilder::new()
        .with_window_builder(Some(window_builder));

    let (window, gl_config) = display_builder
        .build(&event_loop, template, |configs| {
            configs
                .reduce(|cfg1, cfg2| {
                    let t = |c: &Config| (c.num_samples(), c.depth_size(), c.stencil_size());
                    if t(&cfg2) < t(&cfg1) {
                        cfg2
                    } else {
                        cfg1
                    }
                })
                .unwrap()
        })
        .unwrap();
    dbg!(gl_config.num_samples(), gl_config.depth_size(), gl_config.stencil_size());
    let window = window.unwrap();
    window.set_title("Test ImGui 2");
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
            .unwrap_or_else(|_| {
                gl_display
                    .create_context(&gl_config, &fallback_context_attributes)
                    .expect("failed to create context")
            })
    });
    let gl_window = w::GlWindow::new(window, &gl_config);
    let gl_context = not_current_gl_context
        .take()
        .unwrap()
        .make_current(&gl_window.surface)
        .unwrap();
    // Enable v-sync to avoid consuming too much CPU
    let _ = gl_window.surface.set_swap_interval(&gl_context, glutin::surface::SwapInterval::Wait(NonZeroU32::new(1).unwrap()));

    let dsp = gl_context.display();
    gl::load_with(|s| dsp.get_proc_address(&CString::new(s).unwrap()));

    let mut imgui_context = imgui::Context::new();


    let f1 = imgui_context.add_font(imgui::FontInfo::new(KARLA_TTF, 18.0));
    imgui_context.merge_font(imgui::FontInfo::new(UBUNTU_TTF, 18.0).char_range(0x20ac, 0x20ac));
    let f2 = imgui_context.add_font(imgui::FontInfo::new(KARLA_TTF, 36.0));

    let mut renderer = w::Renderer::new()
        .expect("failed to create renderer");

    let size = gl_window.window.inner_size();
    let size = gl_window.to_logical_size::<_, f32>(size);
    let scale = gl_window.window.scale_factor() as f32;
    renderer.set_size(&mut imgui_context, size.into(), scale);

    // Main loop
    let mut last_frame = Instant::now();
    // The main loop will keep on rendering for some ms or some frames after the last input,
    // whatever is longer. The frames are needed in case a file operation takes a lot of time,
    // we want at least a render just after that.
    let mut last_input_time = Instant::now();
    let mut last_input_frame: u32 = 0;
    let mut current_cursor = Some(winit::window::CursorIcon::Default);
    event_loop.run(move |event, _, control_flow| {
        match event {
            event::Event::NewEvents(_) => {
                let now = Instant::now();
                unsafe {
                    let io = &mut *ImGui_GetIO();
                    io.DeltaTime = now.duration_since(last_frame).as_secs_f32();
                }
                last_frame = now;
            }
            event::Event::MainEventsCleared => {
                unsafe {
                    let io = &*ImGui_GetIO();
                    if io.WantSetMousePos {
                        let pos = io.MousePos;
                        let pos = winit::dpi::LogicalPosition { x: pos.x, y: pos.y };
                        let _ = gl_window.window.set_cursor_position(pos);
                    }
                }
                gl_window.window.request_redraw();
            }
            event::Event::RedrawEventsCleared => {
                let now = Instant::now();
                // If the mouse is down, redraw all the time, maybe the user is dragging.
                let mouse = unsafe { ImGui_IsAnyMouseDown() };
                last_input_frame += 1;
                if mouse || now.duration_since(last_input_time) < Duration::from_millis(1000) || last_input_frame < 60 {
                    *control_flow = ControlFlow::Poll;
                } else {
                    *control_flow = ControlFlow::Wait;
                }
            }
            event::Event::RedrawRequested(_) => {
                unsafe {
                    let io = &*ImGui_GetIO();
                    if (ImGuiConfigFlags_(io.ConfigFlags as u32) & ImGuiConfigFlags_::ImGuiConfigFlags_NoMouseCursorChange) == ImGuiConfigFlags_(0) {
                        let cursor = ImGuiMouseCursor_(ImGui_GetMouseCursor());
                        let cursor = if io.MouseDrawCursor || cursor == ImGuiMouseCursor_::ImGuiMouseCursor_None {
                            None
                        } else {
                            Some(w::from_imgui_cursor(cursor))
                        };
                        if cursor != current_cursor {
                            match cursor {
                                None => gl_window.window.set_cursor_visible(false),
                                Some(c) => {
                                    gl_window.window.set_cursor_icon(c);
                                    gl_window.window.set_cursor_visible(true);
                                }
                            }
                            current_cursor = cursor;
                        }
                    }

                    let mut x = 0;
                    renderer.do_frame(&mut imgui_context, |ui| {
                        my_frame(ui, f2, &mut x);
                    });
                }
                gl_window.surface.swap_buffers(&gl_context).unwrap();
            }
            event::Event::DeviceEvent { .. } => {
                // Ignore DeviceEvents, they are not used and they wake up the loop needlessly
            }
            event::Event::WindowEvent {
                window_id,
                event
            } if window_id == gl_window.window.id() => {
                use winit::event::WindowEvent::*;

                last_input_time = Instant::now();
                last_input_frame = 0;

                match event {
                    CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }
                    Resized(size) => {
                        // GL surface in physical pixels, imgui in logical
                        if let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) {
                            gl_window.surface.resize(&gl_context, w, h);
                        }
                        unsafe {
                            let io = &mut *ImGui_GetIO();
                            let size = gl_window.to_logical_size::<_, f32>(size);
                            io.DisplaySize = size.into();
                        }
                    }
                    ScaleFactorChanged { scale_factor, new_inner_size } => {
                        let scale_factor = scale_factor as f32;
                        unsafe {
                            let io = &mut *ImGui_GetIO();
                            let old_scale_factor = io.DisplayFramebufferScale.x;
                            if io.MousePos.x.is_finite() && io.MousePos.y.is_finite() {
                                io.MousePos.x *= scale_factor / old_scale_factor;
                                io.MousePos.y *= scale_factor / old_scale_factor;
                            }
                        }
                        let new_inner_size = gl_window.to_logical_size::<_, f32>(*new_inner_size);
                        renderer.set_size(&mut imgui_context, new_inner_size.into(), scale_factor);
                    }
                    ModifiersChanged(mods) => {
                        unsafe {
                            let io = &mut *ImGui_GetIO();
                            ImGuiIO_AddKeyEvent(io, ImGuiKey::ImGuiMod_Ctrl, mods.ctrl());
                            ImGuiIO_AddKeyEvent(io, ImGuiKey::ImGuiMod_Shift, mods.shift());
                            ImGuiIO_AddKeyEvent(io, ImGuiKey::ImGuiMod_Alt, mods.alt());
                            ImGuiIO_AddKeyEvent(io, ImGuiKey::ImGuiMod_Super, mods.logo());
                        }
                    }
                    KeyboardInput {
                        input: winit::event::KeyboardInput {
                            virtual_keycode: Some(wkey),
                            state,
                            ..
                        },
                        ..
                    } => {
                        if let Some(key) = w::to_imgui_key(wkey) {
                            let pressed = state == winit::event::ElementState::Pressed;
                            unsafe {
                                let io = &mut *ImGui_GetIO();
                                ImGuiIO_AddKeyEvent(io, key, pressed);

                                let kmod = match wkey {
                                    VirtualKeyCode::LControl |
                                    VirtualKeyCode::RControl => Some(ImGuiKey::ImGuiMod_Ctrl),
                                    VirtualKeyCode::LShift |
                                    VirtualKeyCode::RShift => Some(ImGuiKey::ImGuiMod_Shift),
                                    VirtualKeyCode::LAlt |
                                    VirtualKeyCode::RAlt => Some(ImGuiKey::ImGuiMod_Alt),
                                    VirtualKeyCode::LWin |
                                    VirtualKeyCode::RWin => Some(ImGuiKey::ImGuiMod_Super),
                                    _ => None
                                };
                                if let Some(kmod) = kmod {
                                    ImGuiIO_AddKeyEvent(io, kmod, pressed);
                                }
                            }
                        }
                    }
                    ReceivedCharacter(c) => {
                        unsafe {
                            let io = &mut *ImGui_GetIO();
                            ImGuiIO_AddInputCharacter(io, c as u32);
                        }
                    }
                    CursorMoved { position, .. } => {
                        unsafe {
                            let io = &mut *ImGui_GetIO();
                            let position = gl_window.to_logical_pos(position);
                            ImGuiIO_AddMousePosEvent(io, position.x, position.y);
                        }
                    }
                    MouseWheel {
                        delta,
                        phase: winit::event::TouchPhase::Moved,
                        ..
                    } => {
                        let (h, v) = match delta {
                            winit::event::MouseScrollDelta::LineDelta(h, v) => (h, v),
                            winit::event::MouseScrollDelta::PixelDelta(d) => (d.x as f32, d.y as f32), //scale?
                        };
                        unsafe {
                            let io = &mut *ImGui_GetIO();
                            ImGuiIO_AddMouseWheelEvent(io, h, v);
                        }
                    }
                    MouseInput { state, button, .. } => {
                        unsafe {
                            let io = &mut *ImGui_GetIO();
                            if let Some(btn) = w::to_imgui_button(button) {
                                let pressed = state == winit::event::ElementState::Pressed;
                                ImGuiIO_AddMouseButtonEvent(io, btn.0 as i32, pressed);
                            }
                        }
                    }
                    CursorLeft { .. } => {
                        unsafe {
                            let io = &mut *ImGui_GetIO();
                            ImGuiIO_AddMousePosEvent(io, f32::MAX, f32::MAX);
                        }
                    }
                    Focused(focused) => {
                        unsafe {
                            let io = &mut *ImGui_GetIO();
                            ImGuiIO_AddFocusEvent(io, focused);
                        }
                    }
                    _ => {}
                }
            }
            _ => { }
        }
    });
}

static mut X: i32 = 0;

fn my_frame<'cb, 'd: 'cb>(ui: &mut imgui::Ui<'cb, '_>, f2: imgui::FontId, x: &'d mut i32) {
    let mut y = 0;
    {
        ui.set_next_window_size_constraints_callback([20.0, 20.0], [520.0, 520.0], |mut d| {
            let mut sz = d.desired_size();
            sz.x = (sz.x / 100.0).round() * 100.0;
            sz.y = (sz.y / 100.0).round() * 100.0;
            d.set_desired_size(sz);
            //*x += 1;
            //y += 1;
            let _ = *x;
            unsafe { X += 1 };
        });
        //println!("<<<<<<<<< {X}");
    }
    ui.set_next_window_size([100.0, 100.0], imgui::Cond::ImGuiCond_Once);
    ui.set_next_window_pos([0.0, 0.0], imgui::Cond::ImGuiCond_Once, [0.0, 0.0]);
    ui.with_window(cstr!("Yo"), Some(&mut true), 0, |ui| {
        ui.with_child("T", [0.0, 0.0], true, 0, |ui| {
            ui.text_unformatted("Test #1");
            ui.with_font(f2, |ui| {
                ui.text_unformatted("Test #2");
            });
        });
        let mut dl = ui.get_window_draw_list();
        dl.add_callback(|| {
            //println!("callback!");
            let _ = *x;
            //y += 1;
        });
    });

    ui.show_demo_window(&mut true);
}
