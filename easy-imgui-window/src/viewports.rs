use std::{
    cell::Cell,
    ffi::{CStr, c_char, c_void},
    num::NonZeroU32,
    ptr::NonNull,
};

use easy_imgui::{self as imgui, ViewportFlags};
use easy_imgui_renderer::{
    Renderer,
    glow::{self, HasContext},
};
use easy_imgui_sys::{ImGuiViewport, ImVec2};
use glutin_winit::finalize_window;
use raw_window_handle::HasWindowHandle;
use winit::{
    dpi::{LogicalPosition, LogicalSize},
    event_loop::ActiveEventLoop,
    window::{WindowAttributes, WindowLevel},
};

use crate::{MainWindow, MainWindowWithRenderer, ViewportRef};

use anyhow::{Result, anyhow};
use glutin::{
    config::GetGlConfig,
    context::PossiblyCurrentContext,
    display::GetGlDisplay,
    prelude::*,
    surface::{Surface, SurfaceAttributesBuilder, WindowSurface},
};
use winit::window::Window;

thread_local! {
    pub static LOOPER: Cell<Option<(NonNull<ActiveEventLoop>, NonNull<MainWindowWithRenderer>)>> = Cell::new(None);
}

pub struct ViewportWindow {
    // The surface must be dropped before the window.
    surface: Surface<WindowSurface>,
    window: Window,
}

impl ViewportWindow {
    fn new(
        event_loop: &ActiveEventLoop,
        main_window: &MainWindow,
        pos: ImVec2,
        size: ImVec2,
        flags: ViewportFlags,
    ) -> Result<ViewportWindow> {
        let gl_ctx = main_window.glutin_context();
        //let gl = main_window.renderer().gl_context().clone();
        dbg!(flags);
        let wattr = WindowAttributes::default()
            .with_visible(false)
            .with_active(false)
            .with_position(LogicalPosition::new(pos.x, pos.y))
            .with_inner_size(LogicalSize::new(size.x, size.y))
            .with_window_level(if flags.contains(ViewportFlags::TopMost) {
                WindowLevel::AlwaysOnTop
            } else {
                WindowLevel::Normal
            })
            .with_decorations(false);

        //.with(main_window.window().window_handle().ok().map(|h| h.as_raw()))
        let gl_config = gl_ctx.config();
        let window =
            finalize_window(event_loop, wattr, &gl_config).map_err(|e| anyhow!("{:#?}", e))?;
        window.set_ime_allowed(true);

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
        // Enable v-sync to avoid consuming too much CPU
        let _ = surface.set_swap_interval(
            &gl_ctx,
            glutin::surface::SwapInterval::Wait(NonZeroU32::new(1).unwrap()),
        );

        Ok(ViewportWindow { surface, window })
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    fn pre_render(
        &mut self,
        gl_ctx: &PossiblyCurrentContext,
        size: ImVec2,
        scale: f32,
        gl: &glow::Context,
    ) {
        let _ = gl_ctx
            .make_current(&self.surface)
            .inspect_err(|e| log::error!("{e}"));

        unsafe { gl.viewport(0, 0, (size.x * scale) as i32, (size.y * scale) as i32) };
    }
    fn post_render(&mut self, gl_ctx: &PossiblyCurrentContext) {
        let _ = self
            .surface
            .swap_buffers(gl_ctx)
            .inspect_err(|e| log::error!("{e}"));
    }
}

pub unsafe fn setup_viewports(imgui: &mut imgui::Context) {
    unsafe {
        imgui
            .io_mut()
            .inner()
            .add_backend_flags(imgui::BackendFlags::PlatformHasViewports);

        let pio = imgui.platform_io_mut();
        pio.Platform_CreateWindow = Some(create_window);
        pio.Platform_DestroyWindow = Some(destroy_window);
        pio.Platform_ShowWindow = Some(show_window);
        pio.Platform_SetWindowPos = Some(set_window_pos);
        pio.Platform_GetWindowPos = Some(get_window_pos);
        pio.Platform_SetWindowSize = Some(set_window_size);
        pio.Platform_GetWindowSize = Some(get_window_size);
        pio.Platform_GetWindowFramebufferScale = Some(get_window_framebuffer_scale);
        pio.Platform_SetWindowFocus = Some(set_window_focus);
        pio.Platform_GetWindowFocus = Some(get_window_focus);
        pio.Platform_GetWindowMinimized = Some(get_window_minimized);
        pio.Platform_SetWindowTitle = Some(set_window_title);
        pio.Platform_SetWindowAlpha = None; // not supported by winit
        pio.Platform_UpdateWindow = None; // set parent?
        pio.Platform_RenderWindow = Some(render_window);
        pio.Platform_SwapBuffers = None; // render does it
        pio.Platform_GetWindowDpiScale = None; // TODO beta
        pio.Platform_OnChangedViewport = None; // TODO beta
        pio.Platform_GetWindowWorkAreaInsets = None; // TODO beta
        pio.Platform_CreateVkSurface = None; // only OpenGl, no Vulkan

        //pio.Renderer_CreateWindow = Some(renderer_create_window);
        //pio.Renderer_DestroyWindow = Some(renderer_destroy_window);
        //pio.Renderer_SetWindowSize = Some(renderer_set_window_size);
    }
}

pub unsafe extern "C" fn create_window(vp: *mut ImGuiViewport) {
    unsafe {
        //let id = (*vp).ID;
        //let flags = easy_imgui::ViewportFlags::from_bits((*vp).Flags);
        //dbg!(flags);
        //println!("create {id}");
        let (looper, mut main_window) = LOOPER.get().unwrap();
        let looper = looper.as_ref();
        let main_window = main_window.as_mut();
        (*vp).FramebufferScale = main_window.imgui().io().DisplayFramebufferScale;
        let main = main_window.main_window();
        let pos = (*vp).Pos;
        let size = (*vp).Size;
        let flags = ViewportFlags::from_bits_truncate((*vp).Flags);
        let w = ViewportWindow::new(looper, main, pos, size, flags).unwrap();
        let w = Box::new(w);
        let w: *mut ViewportWindow = Box::into_raw(w);
        (*vp).PlatformHandle = w as _;
        (*vp).PlatformUserData = 1 as _;
        (*vp).PlatformRequestResize = true;
        (*vp).PlatformRequestMove = true;
    }
}

unsafe fn get_viewport(vp: *mut ImGuiViewport) -> ViewportRef {
    unsafe {
        match (*vp).PlatformUserData as usize {
            0 => ViewportRef::MainWindow,
            1 => {
                let w = (*vp).PlatformHandle as *mut ViewportWindow;
                ViewportRef::Viewport(w, vp)
            }
            _ => ViewportRef::Unknown,
        }
    }
}

pub unsafe extern "C" fn destroy_window(vp: *mut ImGuiViewport) {
    unsafe {
        //let id = (*vp).ID;
        //println!("destroy {id}");
        match get_viewport(vp) {
            ViewportRef::Unknown => {}
            ViewportRef::MainWindow => {}
            ViewportRef::Viewport(w, _) => {
                (*vp).PlatformHandle = std::ptr::null_mut();
                (*vp).PlatformUserData = std::ptr::null_mut();
                let w = Box::from_raw(w);
                drop(w);
            }
        }
    }
}

pub unsafe extern "C" fn show_window(vp: *mut ImGuiViewport) {
    unsafe {
        //let id = (*vp).ID;
        //println!("show {id}");
        match get_viewport(vp) {
            ViewportRef::Unknown => {}
            ViewportRef::MainWindow => {}
            ViewportRef::Viewport(w, _) => {
                let w = &*w;
                w.window.set_visible(true);
                let flags = ViewportFlags::from_bits_truncate((*vp).Flags);
                if !flags.contains(ViewportFlags::NoFocusOnAppearing) {
                    w.window.focus_window();
                }
            }
        }
    }
}

pub unsafe extern "C" fn set_window_pos(vp: *mut ImGuiViewport, pos: ImVec2) {
    unsafe {
        //let id = (*vp).ID;
        //println!("set_pos {id} {pos:?}");
        match get_viewport(vp) {
            ViewportRef::Unknown => {}
            ViewportRef::MainWindow => {
                let w = (*vp).PlatformHandle as *const MainWindow;
                let w = &*w;
                w.window()
                    .set_outer_position(LogicalPosition::new(pos.x, pos.y));
            }
            ViewportRef::Viewport(w, _) => {
                let w = &*w;
                w.window()
                    .set_outer_position(LogicalPosition::new(pos.x, pos.y));
            }
        }
    }
}

pub unsafe extern "C" fn get_window_pos(vp: *mut ImGuiViewport) -> ImVec2 {
    unsafe {
        //let id = (*vp).ID;
        //println!("get_pos {id}");

        let scale = (*vp).FramebufferScale.x;
        let pos = match get_viewport(vp) {
            ViewportRef::Unknown => unreachable!(),
            ViewportRef::MainWindow => {
                let w = (*vp).PlatformHandle as *const MainWindow;
                let w = &*w;
                w.window().inner_position()
            }
            ViewportRef::Viewport(w, _) => {
                let w = &*w;
                w.window.outer_position()
            }
        };
        let Ok(pos) = pos else {
            return ImVec2 { x: 0.0, y: 0.0 };
        };
        let pos: LogicalPosition<u32> = pos.to_logical(scale as f64);
        ImVec2 {
            x: pos.x as f32,
            y: pos.y as f32,
        }
    }
}

pub unsafe extern "C" fn set_window_size(vp: *mut ImGuiViewport, size: ImVec2) {
    unsafe {
        //let id = (*vp).ID;
        //println!("set_size {id} {size:?}");
        match get_viewport(vp) {
            ViewportRef::MainWindow => {
                let w = (*vp).PlatformHandle as *mut MainWindow;
                let w = &*w;
                let _ = w
                    .window()
                    .request_inner_size(LogicalSize::new(size.x, size.y));
            }
            ViewportRef::Viewport(w, _) => {
                let w = &*w;
                let _ = w
                    .window()
                    .request_inner_size(LogicalSize::new(size.x, size.y));
            }
            ViewportRef::Unknown => {}
        }
    }
}

pub unsafe extern "C" fn get_window_size(vp: *mut ImGuiViewport) -> ImVec2 {
    unsafe {
        //let id = (*vp).ID;
        //println!("get_size {id}");
        let scale = (*vp).FramebufferScale.x;
        match get_viewport(vp) {
            ViewportRef::MainWindow => {
                let w = (*vp).PlatformHandle as *mut MainWindow;
                let w = &*w;
                let size: LogicalSize<f32> = w.window().outer_size().to_logical(scale as f64);
                ImVec2 {
                    x: size.width as f32,
                    y: size.height as f32,
                }
            }
            ViewportRef::Viewport(w, _) => {
                let w = &*w;
                let size: LogicalSize<f32> = w.window.inner_size().to_logical(scale as f64);
                ImVec2 {
                    x: size.width as f32,
                    y: size.height as f32,
                }
            }
            ViewportRef::Unknown => ImVec2 { x: 0.0, y: 0.0 },
        }
    }
}

pub unsafe extern "C" fn get_window_framebuffer_scale(vp: *mut ImGuiViewport) -> ImVec2 {
    unsafe {
        //let id = (*vp).ID;
        //println!("get_window_framebuffer_scale {id}");
        let scale = match get_viewport(vp) {
            ViewportRef::MainWindow => {
                let w = (*vp).PlatformHandle as *mut MainWindow;
                let w = &*w;
                w.window().scale_factor() as f32
            }
            ViewportRef::Viewport(w, _) => {
                let w = &*w;
                w.window().scale_factor() as f32
            }
            ViewportRef::Unknown => 1.0,
        };
        ImVec2 { x: scale, y: scale }
    }
}

pub unsafe extern "C" fn set_window_focus(vp: *mut ImGuiViewport) {
    unsafe {
        //let id = (*vp).ID;
        //println!("set_focus {id}");
        match get_viewport(vp) {
            ViewportRef::MainWindow => {
                let w = (*vp).PlatformHandle as *mut MainWindow;
                let w = &*w;
                w.window().focus_window();
            }
            ViewportRef::Viewport(w, _) => {
                let w = &*w;
                w.window().focus_window();
            }
            ViewportRef::Unknown => {}
        }
    }
}

pub unsafe extern "C" fn get_window_focus(vp: *mut ImGuiViewport) -> bool {
    unsafe {
        //let id = (*vp).ID;
        //println!("get_focus {id}");
        match get_viewport(vp) {
            ViewportRef::MainWindow => {
                let w = (*vp).PlatformHandle as *mut MainWindow;
                let w = &*w;
                w.window().has_focus()
            }
            ViewportRef::Viewport(w, _) => {
                let w = &*w;
                w.window().has_focus()
            }
            ViewportRef::Unknown => false,
        }
    }
}

pub unsafe extern "C" fn get_window_minimized(vp: *mut ImGuiViewport) -> bool {
    unsafe {
        //let id = (*vp).ID;
        //println!("get_minimized {id}");
        match get_viewport(vp) {
            ViewportRef::MainWindow => {
                let w = (*vp).PlatformHandle as *mut MainWindow;
                let w = &*w;
                w.window().is_minimized().unwrap_or(false)
            }
            ViewportRef::Viewport(w, _) => {
                let w = &*w;
                w.window().is_minimized().unwrap_or(false)
            }
            ViewportRef::Unknown => false,
        }
    }
}

pub unsafe extern "C" fn set_window_title(vp: *mut ImGuiViewport, title: *const c_char) {
    unsafe {
        //let id = (*vp).ID;
        //println!("set_title {id}");
        let title = CStr::from_ptr(title);
        let title = title.to_string_lossy();
        match get_viewport(vp) {
            ViewportRef::MainWindow => {
                let w = (*vp).PlatformHandle as *mut MainWindow;
                let w = &*w;
                w.window().set_title(&title);
            }
            ViewportRef::Viewport(w, _) => {
                let w = &*w;
                w.window().set_title(&title);
            }
            ViewportRef::Unknown => {}
        }
    }
}

pub unsafe extern "C" fn render_window(vp: *mut ImGuiViewport, _render_arg: *mut c_void) {
    unsafe {
        //let id = vp.ID;
        //println!("render {id}");
        match get_viewport(vp) {
            ViewportRef::MainWindow => {}
            ViewportRef::Unknown => {}
            ViewportRef::Viewport(w, _) => {
                let (_, mut main_window) = LOOPER.get().unwrap();
                let main_window = main_window.as_mut();
                let gl = main_window.renderer().gl_context().clone();
                let w = &mut *w;
                let scale = w.window().scale_factor() as f32;
                w.pre_render(
                    main_window.main_window().glutin_context(),
                    (*vp).Size,
                    scale,
                    &gl,
                );
                let dd = &*(*vp).DrawData;
                let gl_objs = main_window.renderer().gl_objs();
                Renderer::render(&gl, gl_objs, dd, None);
                w.post_render(main_window.main_window().glutin_context());
            }
        }
    }
}

/*
pub unsafe extern "C" fn renderer_create_window(vp: *mut ImGuiViewport) {
    let id = (*vp).ID;
    println!("renderer_create {id}");
}
pub unsafe extern "C" fn renderer_destroy_window(vp: *mut ImGuiViewport) {
    let id = (*vp).ID;
    println!("renderer_destroy {id}");
}
pub unsafe extern "C" fn renderer_set_window_size(vp: *mut ImGuiViewport, _size: ImVec2) {
    let id = (*vp).ID;
    println!("renderer_set_size {id}");
}
*/
