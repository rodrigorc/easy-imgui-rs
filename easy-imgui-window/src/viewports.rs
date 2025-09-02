use std::{
    cell::Cell,
    ffi::{CStr, c_char, c_void},
    num::NonZeroU32,
    ptr::NonNull,
};

use easy_imgui::{self as imgui, ViewportFlags};
use easy_imgui_renderer::glow::{self, HasContext};
use easy_imgui_sys::{ImGui_UpdatePlatformWindows, ImGuiViewport, ImVec2};
use glutin_winit::finalize_window;
use raw_window_handle::HasWindowHandle;
use winit::{
    dpi::{LogicalPosition, LogicalSize},
    event_loop::ActiveEventLoop,
    window::{WindowAttributes, WindowLevel},
};

use anyhow::{Result, anyhow};
use glutin::{
    config::GetGlConfig,
    context::PossiblyCurrentContext,
    display::GetGlDisplay,
    prelude::*,
    surface::{Surface, SurfaceAttributesBuilder, WindowSurface},
};
use winit::window::Window;

// This thread_local variable is used to send the current loop to `create_window()`.
// Unfortunately there is no easy way to pass this data down from the window loop to the ImGui callbacks.
thread_local! {
    static LOOPER: Cell<Option<(NonNull<ActiveEventLoop>, NonNull<PossiblyCurrentContext>, f32)>> = Cell::new(None);
}

pub unsafe fn update_platform_windows(
    event_loop: &ActiveEventLoop,
    glutin_context: &PossiblyCurrentContext,
    scale: f32,
) {
    LOOPER.set(Some((event_loop.into(), glutin_context.into(), scale)));
    unsafe { ImGui_UpdatePlatformWindows() };
    LOOPER.set(None);
}

pub struct ViewportWindow {
    // The surface must be dropped before the window.
    pub surface: Surface<WindowSurface>,
    pub window: Window,
}

impl ViewportWindow {
    fn new(
        event_loop: &ActiveEventLoop,
        gl_ctx: &PossiblyCurrentContext,
        pos: ImVec2,
        size: ImVec2,
        flags: ViewportFlags,
    ) -> Result<ViewportWindow> {
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

    pub fn pre_render(
        &self,
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
    pub fn post_render(&self, gl_ctx: &PossiblyCurrentContext) {
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
        pio.Platform_RenderWindow = None; // implemented manually
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

// Values for PlatformUserData
const PLATFORM_USER_DATA_MAIN_WINDOW: *mut c_void = 1 as _;
const PLATFORM_USER_DATA_VIEWPORT: *mut c_void = 2 as _;

pub unsafe fn set_viewport_as_main_window(vp: *mut ImGuiViewport, vw: *mut ViewportWindow) {
    unsafe {
        (*vp).PlatformUserData = PLATFORM_USER_DATA_MAIN_WINDOW;
        (*vp).PlatformHandle = vw as _;
    }
}

pub unsafe fn set_viewport_as_vieport(vp: *mut ImGuiViewport, vw: *mut ViewportWindow) {
    unsafe {
        (*vp).PlatformUserData = PLATFORM_USER_DATA_VIEWPORT;
        (*vp).PlatformHandle = vw as _;
    }
}

pub enum RealViewportRef {
    MainWindow(*mut ViewportWindow),
    Viewport(*mut ViewportWindow),
}

impl RealViewportRef {
    pub unsafe fn viewport_window(&mut self) -> &mut ViewportWindow {
        let w = match *self {
            RealViewportRef::MainWindow(w) => w,
            RealViewportRef::Viewport(w) => w,
        };
        unsafe { &mut *w }
    }
    pub unsafe fn window(&mut self) -> &mut Window {
        unsafe { &mut self.viewport_window().window }
    }
}

pub unsafe fn get_real_viewport(vp: *mut ImGuiViewport) -> RealViewportRef {
    unsafe {
        let w = (*vp).PlatformHandle as *mut ViewportWindow;
        match (*vp).PlatformUserData {
            PLATFORM_USER_DATA_MAIN_WINDOW => RealViewportRef::MainWindow(w),
            PLATFORM_USER_DATA_VIEWPORT => RealViewportRef::Viewport(w),
            _ => unreachable!(),
        }
    }
}

pub unsafe extern "C" fn create_window(vp: *mut ImGuiViewport) {
    unsafe {
        let id = (*vp).ID;
        let flags = easy_imgui::ViewportFlags::from_bits_truncate((*vp).Flags);
        log::debug!("create {id} {flags:?}");

        let (looper, gl_ctx, scale) = LOOPER.get().unwrap();
        let looper = looper.as_ref();
        let gl_ctx = gl_ctx.as_ref();
        (*vp).FramebufferScale = ImVec2 { x: scale, y: scale };
        let pos = (*vp).Pos;
        let size = (*vp).Size;
        let w = ViewportWindow::new(looper, gl_ctx, pos, size, flags).unwrap();
        log::debug!("viewport wid {:?}", w.window().id());
        let w = Box::new(w);
        let w: *mut ViewportWindow = Box::into_raw(w);
        set_viewport_as_vieport(&mut *vp, w);
        (*vp).PlatformRequestResize = true;
        (*vp).PlatformRequestMove = true;
    }
}

pub unsafe extern "C" fn destroy_window(vp: *mut ImGuiViewport) {
    unsafe {
        let id = (*vp).ID;
        log::debug!("destroy {id}");
        match get_real_viewport(vp) {
            RealViewportRef::MainWindow(_) => {
                // Do not destroy the main window here, it has its own life.
            }
            RealViewportRef::Viewport(w) => {
                let w = Box::from_raw(w);
                drop(w);
            }
        }
        (*vp).PlatformUserData = std::ptr::null_mut();
        (*vp).PlatformHandle = std::ptr::null_mut();
    }
}

pub unsafe extern "C" fn show_window(vp: *mut ImGuiViewport) {
    unsafe {
        //let id = (*vp).ID;
        //println!("show {id}");
        match get_real_viewport(vp) {
            RealViewportRef::MainWindow(_) => {
                // Main viewport can't be hidden
            }
            RealViewportRef::Viewport(w) => {
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
        get_real_viewport(vp)
            .window()
            .set_outer_position(LogicalPosition::new(pos.x, pos.y));
    }
}

pub unsafe extern "C" fn get_window_pos(vp: *mut ImGuiViewport) -> ImVec2 {
    unsafe {
        //let id = (*vp).ID;
        //println!("get_pos {id}");

        let scale = (*vp).FramebufferScale.x;
        let pos = match get_real_viewport(vp) {
            RealViewportRef::MainWindow(w) => {
                let w = &*w;
                w.window().inner_position().ok()
            }
            RealViewportRef::Viewport(w) => {
                let w = &*w;
                w.window.outer_position().ok()
            }
        };
        let Some(pos) = pos else {
            return ImVec2 { x: 0.0, y: 0.0 };
        };
        // Use u32 instead of f32 to avoid rounding errors in the real window position
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
        let _ = get_real_viewport(vp)
            .window()
            .request_inner_size(LogicalSize::new(size.x, size.y));
    }
}

pub unsafe extern "C" fn get_window_size(vp: *mut ImGuiViewport) -> ImVec2 {
    unsafe {
        //let id = (*vp).ID;
        //println!("get_size {id}");
        let scale = (*vp).FramebufferScale.x;
        // Use u32 instead of f32 to avoid rounding errors in the real window size
        match get_real_viewport(vp) {
            RealViewportRef::MainWindow(w) => {
                let w = &*w;
                let size: LogicalSize<u32> = w.window().outer_size().to_logical(scale as f64);
                ImVec2 {
                    x: size.width as f32,
                    y: size.height as f32,
                }
            }
            RealViewportRef::Viewport(w) => {
                let w = &*w;
                let size: LogicalSize<u32> = w.window.inner_size().to_logical(scale as f64);
                ImVec2 {
                    x: size.width as f32,
                    y: size.height as f32,
                }
            }
        }
    }
}

pub unsafe extern "C" fn get_window_framebuffer_scale(vp: *mut ImGuiViewport) -> ImVec2 {
    unsafe {
        //let id = (*vp).ID;
        //println!("get_window_framebuffer_scale {id}");
        let scale = get_real_viewport(vp).window().scale_factor() as f32;
        ImVec2 { x: scale, y: scale }
    }
}

pub unsafe extern "C" fn set_window_focus(vp: *mut ImGuiViewport) {
    unsafe {
        //let id = (*vp).ID;
        //println!("set_focus {id}");
        get_real_viewport(vp).window().focus_window();
    }
}

pub unsafe extern "C" fn get_window_focus(vp: *mut ImGuiViewport) -> bool {
    unsafe {
        //let id = (*vp).ID;
        //println!("get_focus {id}");
        get_real_viewport(vp).window().has_focus()
    }
}

pub unsafe extern "C" fn get_window_minimized(vp: *mut ImGuiViewport) -> bool {
    unsafe {
        //let id = (*vp).ID;
        //println!("get_minimized {id}");
        get_real_viewport(vp)
            .window()
            .is_minimized()
            .unwrap_or(false)
    }
}

pub unsafe extern "C" fn set_window_title(vp: *mut ImGuiViewport, title: *const c_char) {
    unsafe {
        //let id = (*vp).ID;
        //println!("set_title {id}");
        let title = CStr::from_ptr(title);
        let title = title.to_string_lossy();
        get_real_viewport(vp).window().set_title(&title);
    }
}

/*
pub unsafe fn render_window(vp: *mut ImGuiViewport) {
    unsafe {
        //let id = vp.ID;
        //println!("render {id}");
    }
}
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
