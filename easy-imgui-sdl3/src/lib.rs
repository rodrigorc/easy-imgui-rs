use std::{
    marker::PhantomData,
    rc::Rc,
    sync::{Arc, Mutex, OnceLock, Weak, mpsc::SendError},
};

use easy_imgui::{
    EventResult, Idler, TextureId, UiBuilder,
    future::{FutureHandle, spawn_idle},
};
use sdl3::{
    EventPump,
    event::{Event, EventSender, WindowEvent},
    video::SwapInterval,
};

use easy_imgui_sys::{ImTextureID, sdl3_sys};
use sdl3_sys::everything::*;

pub mod fut;

// Reexport the main dependencies to simplify dependencies.
pub use easy_imgui;
pub use sdl3;

/// Creates a Glow context from a `sdl3` video subsystem.
/// # Safety
/// Call this once after the window context is current.
pub unsafe fn glow_context(sdl_video: &sdl3::VideoSubsystem) -> glow::Context {
    unsafe {
        glow::Context::from_loader_function(|name| {
            use std::ffi::c_void;

            sdl_video
                .gl_get_proc_address(name)
                .map(|f| f as *const c_void)
                .unwrap_or_default()
        })
    }
}

/// Initializes the backend for OpenGL.
///
/// # Safety
/// Call this once after the window context is current.
pub unsafe fn init_for_opengl(window: &sdl3::video::Window, sdl_gl: &sdl3::video::GLContext) {
    unsafe {
        easy_imgui_sys::ImGui_ImplSDL3_InitForOpenGL(window.raw(), sdl_gl.raw() as *mut _);
        easy_imgui_sys::ImGui_ImplOpenGL3_Init(c"#version 150".as_ptr());
    }
}

/// Deinitializes the backend for OpenGL.
///
/// # Safety
/// Call this just before destroying the ImGui context or the SDL3 window.
pub unsafe fn shutdown_for_opengl() {
    unsafe {
        easy_imgui_sys::ImGui_ImplOpenGL3_Shutdown();
        easy_imgui_sys::ImGui_ImplSDL3_Shutdown();
    }
}

/// Runs an ImGui frame.
///
/// # Safety
/// Same requirements as `easy_imgui::CurrentContext::do_frame`.
pub unsafe fn do_frame<A: UiBuilder>(
    imgui: &mut easy_imgui::CurrentContext,
    window: &sdl3::video::Window,
    sdl_gl: &sdl3::video::GLContext,
    app: &mut A,
) {
    unsafe {
        easy_imgui_sys::ImGui_ImplOpenGL3_NewFrame();
        easy_imgui_sys::ImGui_ImplSDL3_NewFrame();

        let viewports =
            imgui.io().ConfigFlags & easy_imgui::ConfigFlags::ViewportsEnable.bits() != 0;

        imgui.do_frame(
            app,
            |_| {},
            |render_data| {
                easy_imgui_sys::ImGui_ImplOpenGL3_RenderDrawData(
                    (&raw const *render_data).cast_mut(),
                );

                if viewports {
                    easy_imgui_sys::ImGui_UpdatePlatformWindows();
                    easy_imgui_sys::ImGui_RenderPlatformWindowsDefault(
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                    );
                    let _ = window.gl_make_current(sdl_gl);
                }
            },
        );
    }
}

/// Wrapper for `SDL_WaitEvent` without consuming any.
///
/// `sdl3` lacks this function.
pub fn sdl3_wait_event(_pump: &mut sdl3::EventPump) {
    unsafe {
        SDL_WaitEvent(std::ptr::null_mut());
    }
}

/// Gets an event from the event queue returning the low-level version.
///
/// This doesn't convert it to `sdl3::event::Event`.
/// You can do that later with `Event::from_ll`.
pub fn sdl3_poll_event_ll(_pump: &mut sdl3::EventPump) -> Option<SDL_Event> {
    let mut raw = std::mem::MaybeUninit::uninit();
    unsafe {
        let has_pending = SDL_PollEvent(raw.as_mut_ptr());
        if has_pending {
            Some(raw.assume_init())
        } else {
            None
        }
    }
}

//////////////////////////////////////////////////

/// The main application trait.
///
/// Implement this in your main application struct.
pub trait Application: easy_imgui::UiBuilder + Sized + 'static {
    /// The user event type.
    /// This can be used to send events back to the application from anywhere.
    type UserEvent: Send + 'static;
    /// Associated type, usually `()` but you can use it for whatever you want.
    ///
    /// It is available even before the application is created.
    type Data;

    /// The main window has been created, please create the application.
    fn new(args: Args<'_, Self>) -> Self;

    /// Callback for every SDL3 event.
    fn sdl3_event(
        &mut self,
        _args: Args<'_, Self>,
        _event: sdl3::event::Event,
        _event_result: &mut EventResult,
    ) {
    }
    /// A user event has been received.
    fn user_event(&mut self, _args: Args<'_, Self>, _event: Self::UserEvent) {}
    /// This is called after rendering the frame.
    fn post_frame(&mut self, _args: Args<'_, Self>) {}
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
    idler: &'a mut Idler,
    /// The ImGui Context
    pub imgui: &'a mut easy_imgui::RawContext,
    /// The main window.
    pub window: &'a mut sdl3::video::Window,
    /// The GL context
    pub gl: &'a Rc<glow::Context>,
    /// A proxy to send messages to the main loop.
    pub local_proxy: &'a LocalProxy<A>,
    /// The custom application data.
    pub data: &'a mut A::Data,
}

impl<A: Application> Args<'_, A> {
    /// Reborrows `self`.
    pub fn reborrow(&mut self) -> Args<'_, A> {
        Args {
            idler: self.idler,
            imgui: self.imgui,
            window: self.window,
            gl: self.gl,
            local_proxy: self.local_proxy,
            data: self.data,
        }
    }
    /// Creates a `LocalProxy` that is `Clone` but not `Send`.
    pub fn local_proxy(&self) -> LocalProxy<A> {
        self.local_proxy.clone()
    }
    /// Sends a ping to the main loop, but simpler than using the proxy.
    pub fn ping_user_input(&mut self) {
        self.idler.ping_user_input();
    }
}

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

/// Like `LocalProxy` but can be sent between threads.
pub struct Proxy<A: Application> {
    sender: Arc<EventSender>,
    register: u32,
    user_events: Weak<Mutex<Vec<AppEvent<A>>>>,
    pd: PhantomData<A>,
}

unsafe impl<A: Application> Send for Proxy<A> {}
unsafe impl<A: Application> Sync for Proxy<A> {}

impl<A: Application> Clone for Proxy<A> {
    fn clone(&self) -> Self {
        Proxy {
            sender: self.sender.clone(),
            user_events: self.user_events.clone(),
            ..*self
        }
    }
}

impl<A: Application> Proxy<A> {
    fn send_priv(&self, ev: AppEvent<A>) -> Result<(), SendError<AppEvent<A>>> {
        let Some(user_events) = self.user_events.upgrade() else {
            return Err(SendError(ev));
        };
        let mut user_events = user_events.lock().unwrap();
        let ue = Event::User {
            timestamp: 0,
            window_id: 0,
            type_: self.register,
            code: 0,
            data1: std::ptr::null_mut(),
            data2: std::ptr::null_mut(),
        };

        if self.sender.push_event(ue).is_err() {
            return Err(SendError(ev));
        }
        user_events.push(ev);
        Ok(())
    }
    /// Sends a user event to the application.
    pub fn send_user(&self, u: A::UserEvent) -> Result<(), SendError<AppEvent<A>>> {
        self.send_priv(AppEvent::<A>::User(u))
    }
    /// Pings the main loop, to avoid idling.
    pub fn ping_user_input(&self) {
        let _ = self.send_priv(AppEvent::<A>::PingUserInput);
    }
    /// Runs the given function in the main loop.
    pub fn run_idle<F: FnOnce(&mut A, Args<'_, A>) + Send + Sync + 'static>(
        &self,
        f: F,
    ) -> Result<(), SendError<AppEvent<A>>> {
        self.send_priv(AppEvent::RunIdle(Box::new(f)))
    }
}

/// Type to access send things to the main loop.
///
/// This can't be sent between threads, for that use `Proxy`.
pub struct LocalProxy<A: Application> {
    proxy: Proxy<A>,
    // !Send + !Sync
    pd: PhantomData<*const ()>,
}

impl<A: Application> Clone for LocalProxy<A> {
    fn clone(&self) -> Self {
        LocalProxy {
            proxy: self.proxy.clone(),
            pd: PhantomData,
        }
    }
}

impl<A: Application> LocalProxy<A> {
    fn new(sender: EventSender, register: u32, user_events: Weak<Mutex<Vec<AppEvent<A>>>>) -> Self {
        LocalProxy {
            proxy: Proxy {
                sender: Arc::new(sender),
                register,
                user_events,
                pd: PhantomData,
            },
            pd: PhantomData,
        }
    }

    /// Gets a proxy that can be sent between threads.
    pub fn proxy(&self) -> Proxy<A> {
        self.proxy.clone()
    }

    /// Pings the main loop.
    pub fn ping_user_input(&self) {
        self.proxy.ping_user_input();
    }

    /// Sends a user event to the application.
    pub fn send_user(&self, u: A::UserEvent) -> Result<(), SendError<AppEvent<A>>> {
        self.proxy.send_priv(AppEvent::<A>::User(u))
    }

    /// Registers a future to be run during the idle step of the main loop.
    pub fn spawn_idle<T: 'static, F: Future<Output = T> + 'static>(&self, f: F) -> FutureHandle<T> {
        unsafe {
            let runner = fut::MyIdleRunner(self.proxy.clone());
            spawn_idle(runner, f)
        }
    }

    /// Registers a callback to be called during the idle step of the main loop.
    pub fn run_idle<F: FnOnce(&mut A, Args<'_, A>) + 'static>(
        &self,
        f: F,
    ) -> Result<(), SendError<()>> {
        // Self is !Send+!Sync, so this must be in the main loop,
        // and the idle callback will be run in the same loop.
        let f = send_wrapper::SendWrapper::new(f);
        // If it fails, drop the message instead of returning it, because the
        // message is Send but the f inside is not.
        self.proxy
            .run_idle(move |app, args| (f.take())(app, args))
            .map_err(|_| SendError(()))
    }

    /// Creates a `FutureBackCaller` for this application.
    pub fn future_back(&self) -> fut::FutureBackCaller<A> {
        fut::FutureBackCaller::new()
    }
}

/// The main type that handles the main loop.
pub struct AppHandler<A: Application> {
    imgui: easy_imgui::Context,
    clear_color: Option<easy_imgui::Color>,
    gl: Rc<glow::Context>,
    window: sdl3::video::Window,
    sdl_gl: sdl3::video::GLContext,
    user_events: Arc<Mutex<Vec<AppEvent<A>>>>,
    local_proxy: LocalProxy<A>,
    idler: easy_imgui::Idler,
    app_data: A::Data,
    app: A,
}

impl<A: Application> Drop for AppHandler<A> {
    fn drop(&mut self) {
        unsafe {
            let _imgui = self.imgui.set_current();
            shutdown_for_opengl();
        }
    }
}

impl<A: Application> AppHandler<A> {
    /// Creates an `AppHandler`.
    pub fn new(
        imgui_builder: &easy_imgui::ContextBuilder,
        event: &sdl3::EventSubsystem,
        mut window: sdl3::video::Window,
        sdl_gl: sdl3::video::GLContext,
        mut app_data: A::Data,
    ) -> AppHandler<A> {
        unsafe {
            window.gl_make_current(&sdl_gl).unwrap();
            let _ = window.subsystem().gl_set_swap_interval(SwapInterval::VSync);
            let gl = glow_context(window.subsystem());
            let gl = Rc::new(gl);

            let mut imgui = imgui_builder.build();

            let io = imgui.io_mut();
            let io = io.inner();
            io.ConfigDpiScaleFonts = true;
            io.ConfigDpiScaleViewports = true;

            init_for_opengl(&window, &sdl_gl);

            let window_scale = window.display_scale();
            let style = imgui.style_mut();
            style.scale_all_sizes(window_scale);
            style.FontScaleDpi = window_scale;

            let mut idler = easy_imgui::Idler::default();

            // event.register_custom_event<T> is finicky and prompt to leaks.
            // Instead we'll use a custom register event (without data) and our own queue.
            let user_events = Default::default();

            // SDL_RegisterEvents uses a global register, in spite of the Rust EventSubsystem.
            // So we are good registering this just once. Not that you can create two separate event
            // loops at the same time anyway.
            static REGISTER_EVENT: OnceLock<u32> = OnceLock::new();
            let register_event = *REGISTER_EVENT.get_or_init(|| event.register_event().unwrap());

            let local_proxy = LocalProxy::new(
                event.event_sender(),
                register_event,
                Arc::downgrade(&user_events),
            );
            let args = Args {
                idler: &mut idler,
                imgui: &mut imgui,
                window: &mut window,
                gl: &gl,
                local_proxy: &local_proxy,
                data: &mut app_data,
            };

            let app = A::new(args);

            AppHandler {
                imgui,
                clear_color: Some(easy_imgui::Color::new(0.45, 0.55, 0.60, 1.00)),
                gl,
                window,
                sdl_gl,
                user_events,
                local_proxy,
                idler,
                app_data,
                app,
            }
        }
    }

    /// Changes the background color of the `ImGui` window.
    ///
    /// Setting this to `None` will skip clearing the window. This is useful if you
    /// want to render something in the background.
    pub fn set_clear_color(&mut self, color: Option<easy_imgui::Color>) {
        self.clear_color = color;
    }

    /// The `easy_imgui::Context`.
    pub fn imgui(&self) -> &easy_imgui::Context {
        &self.imgui
    }
    /// The mutable `easy_imgui::Context`
    pub fn imgui_mut(&mut self) -> &mut easy_imgui::Context {
        &mut self.imgui
    }
    /// The SDL3 window.
    pub fn window(&self) -> &sdl3::video::Window {
        &self.window
    }
    /// The mutable SDL3 window.
    pub fn window_mut(&mut self) -> &mut sdl3::video::Window {
        &mut self.window
    }
    /// Gets the custom data.
    pub fn data(&self) -> &A::Data {
        &self.app_data
    }
    /// Gets a mutable reference to the custom data.
    pub fn data_mut(&mut self) -> &mut A::Data {
        &mut self.app_data
    }
    /// The user application.
    pub fn app(&self) -> &A {
        &self.app
    }
    /// The mutable user application.
    pub fn app_mut(&mut self) -> &mut A {
        &mut self.app
    }
    /// A reference to the pre-created `glow` GL context.
    ///
    /// It is in a `Rc` so it is cloneable.
    pub fn gl_context(&self) -> &Rc<glow::Context> {
        &self.gl
    }

    /// Runs the main loop, until the main window is closed.
    pub fn run(&mut self, event_pump: &mut EventPump) {
        loop {
            let res = self.pump_events(event_pump);
            if res.window_closed {
                break;
            }
            self.render();
        }
    }

    /// Runs one pump of SDL3 events.
    ///
    /// Then does a ImGui frame and renders it.
    pub fn pump_events(&mut self, event_pump: &mut EventPump) -> EventResult {
        let mut imgui = unsafe { self.imgui.set_current() };

        self.idler.incr_frame();
        if !self.idler.has_to_render() {
            sdl3_wait_event(event_pump);
        }
        let mut ping = false;
        let mut res = EventResult::new(&imgui, false);
        let mut args = Args {
            idler: &mut self.idler,
            imgui: &mut imgui,
            window: &mut self.window,
            gl: &self.gl,
            local_proxy: &self.local_proxy,
            data: &mut self.app_data,
        };
        while let Some(event) = sdl3_poll_event_ll(event_pump) {
            unsafe {
                if easy_imgui_sys::ImGui_ImplSDL3_ProcessEvent(&event) {
                    ping = true;
                }
            }
            let event = Event::from_ll(event);
            match event {
                Event::Quit { .. } => res.window_closed = true,

                Event::Window {
                    win_event: WindowEvent::CloseRequested,
                    window_id,
                    ..
                } if window_id == args.window.id() => res.window_closed = true,

                Event::User { type_, .. } if type_ == args.local_proxy.proxy.register => {
                    let user_events = std::mem::take(&mut *self.user_events.lock().unwrap());
                    for user_event in user_events {
                        match user_event {
                            AppEvent::PingUserInput => ping = true,
                            AppEvent::RunIdle(f) => f(&mut self.app, args.reborrow()),
                            AppEvent::RunIdleSimple(f) => {
                                fut::FutureBackCaller::prepare(&mut self.app, args.reborrow(), f)
                            }
                            AppEvent::User(uev) => self.app.user_event(args.reborrow(), uev),
                        }
                    }
                }

                _ => (),
            };
            self.app.sdl3_event(args.reborrow(), event, &mut res);
        }
        if ping {
            self.idler.ping_user_input();
        }
        res
    }

    /// Renders the ImGui frame.
    pub fn render(&mut self) {
        let mut imgui = unsafe { self.imgui.set_current() };

        unsafe {
            let (display_w, display_h) = self.window.size_in_pixels();

            if let Some(color) = self.clear_color {
                use glow::HasContext;

                self.gl.viewport(0, 0, display_w as i32, display_h as i32);
                self.gl.clear_color(color.r, color.g, color.b, color.a);
                self.gl.clear(glow::COLOR_BUFFER_BIT);
            }
            do_frame(&mut imgui, &self.window, &self.sdl_gl, &mut self.app);
        }
        self.window.gl_swap_window();

        let args = Args {
            idler: &mut self.idler,
            imgui: &mut imgui,
            window: &mut self.window,
            gl: &self.gl,
            local_proxy: &self.local_proxy,
            data: &mut self.app_data,
        };
        self.app.post_frame(args);
    }
}

/// Maps an OpenGL texture to an ImGui texture.
pub fn map_tex(ntex: glow::Texture) -> TextureId {
    unsafe { TextureId::from_id(ntex.0.get() as ImTextureID) }
}

/// Gets an OpenGL texture from an ImGui texture.
pub fn unmap_tex(tex: TextureId) -> Option<glow::Texture> {
    Some(glow::NativeTexture(std::num::NonZeroU32::new(
        tex.id() as u32
    )?))
}

/// Returns the SDL3 WindowId of a viewport.
///
/// If the viewport you pass is not a ImGui SDL3 viewport, the return will be unspecified.
pub fn viewport_window_id(vp: &easy_imgui::Viewport) -> u32 {
    vp.PlatformHandle as u32
}
