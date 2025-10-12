#![allow(clippy::needless_doctest_main)]

/*!
 * Crate for easy integration of the [Dear ImGui][dearimgui] library.
 *
 * This crate is a bind to the Dear ImGui library only. There is also a matching rendering
 * library, [`easy-imgui-renderer`](https://docs.rs/easy-imgui-renderer), that renders the UI using OpenGl, and a matching
 * window-integrated library, [`easy-imgui-window`](https://docs.rs/easy-imgui-window/), that enables to build a full desktop
 * application in just a few lines.
 *
 * If you don't know where to start, then start with the latter. Take a look at the [examples].
 * The simplest `easy-imgui` program would be something like this:
 *
 * ## A note about labels and ids.
 *
 * In [Dear ImGui][1], many controls take a string argument as both a label and an identifier. You can
 * use `##` or a `###` as a separator between the label and the idenfifier if you want to make them
 * apart.
 *
 * In `easy_imgui`, since version 0.8, this is represented by the [`LblId`] type, not by a plain
 * string.
 *
 * If you want to keep on using the same type just write `lbl("foo")`, `lbl("foo##bar")` or
 * `"foo".into()` and it will behave the same as before. But if you want to use them
 * separately, you can write `lbl_id("Label", "id")` and it will join the two strings for you,
 * separated by `###`. This is particularly nice if you pretend to translate the UI: the labels change,
 * but the ids should remain constant.
 *
 * Some functions only take an id, no label. Those will take an argument of type [`Id`], that you
 * can construct with the function [`id`], that will prepend the `###` or [`raw_id`] to use the
 * string as-is. Like before, you can also write `"id".into()` to behave as previous versions of
 * this crate.
 *
 * [1]: https://github.com/ocornut/imgui/blob/master/docs/FAQ.md#q-about-the-id-stack-system
 *
 * ```rust, no_run
 * use easy_imgui_window::{
 *     easy_imgui as imgui,
 *     MainWindow,
 *     MainWindowWithRenderer,
 *     Application, AppHandler, Args, EventResult,
 *     winit,
 * };
 * use winit::{
 *     event_loop::{EventLoop, ActiveEventLoop},
 *     event::WindowEvent,
 * };
 *
 * // The App type, this will do the actual app. stuff.
 * struct App;
 *
 * // This trait handles the UI building.
 * impl imgui::UiBuilder for App {
 *     // There are other function in this trait, but this is the only one
 *     // mandatory and the most important: it build the UI.
 *     fn do_ui(&mut self, ui: &imgui::Ui<Self>) {
 *         ui.show_demo_window(None);
 *     }
 * }
 *
 * // This trait handles the application & event loop stuff.
 * impl Application for App {
 *     // The user event type, `()` if not needed.
 *     type UserEvent = ();
 *     // Custom data type, `()` if not needed.
 *     type Data = ();
 *
 *     // Create the app object.
 *     fn new(_: Args<()>) -> App {
 *         App
 *     }
 *     // Handle one window event.
 *     // There are a few other functions for other types of events.
 *     fn window_event(&mut self, args: Args<()>, _event: WindowEvent, res: EventResult) {
 *         if res.window_closed {
 *             args.event_loop.exit();
 *         }
 *     }
 * }
 *
 * fn main() {
 *     // Create a `winit` event loop.
 *     let event_loop = EventLoop::new().unwrap();
 *     // Create an application handler.
 *     let mut main = AppHandler::<App>::default();
 *     // Optionally set up the window attributes.
 *     main.attributes().title = String::from("Example");
 *     // Run the loop
 *     event_loop.run_app(&mut main);
 * }
 * ```
 *
 * # Alternatives
 * This `crate` is similar to [`imgui-rs`][imguirs], and it is inpired by it, but with a few key
 * differences:
 *  * It doesn't use any C++-to-C api generator, as `rust-bindgen` is able to import simple C++
 *    libraries directly.
 *  * It is lower level, there are fewer high-level abstractions over the ImGui API. This means
 *    that:
 *      * This API is less Rusty than imgui-rs's.
 *      * If you know how to use Dear ImGui, then you know how to use easy-imgui.
 *      * It is far easier to upgrade to new Dear ImGui versions.
 *
 * # Features
 * These are the available features for this crate:
 *  * `freetype`: Uses an external _freetype_ font loader for Dear ImGui, instead of the embedded
 *    `stb_truetype` library.
 *
 * # Usage
 * It is easier to use one of the higher level crates [`easy-imgui-window`] or [`easy-imgui-renderer`].
 * But if you intend to render the UI yourself, then you can use this directly.
 *
 * These are the main pieces of this crate:
 *  * [`Context`]: It represents the ImGui context. In DearImgui this is a global variable. Here it
 *    is a thread-local variable. Still, since it is implicit in most API calls, most uses of this
 *    type are unsafe. If you use [`easy-imgui-window`] or [`easy-imgui-renderer`] you will rarely
 *    need to touch this type directly.
 *  * [`Ui`]: A frame that is being built. Most ImGui functions are members of `Ui`.
 *  * [`UiBuilder`]: A trait that your application implements do build your user interface.
 *
 * If you want to use this library directly, just create a [`Context`], set up its properties, and
 * when you want to render a frame do [`Context::set_current`] and then [`CurrentContext::do_frame`].
 *
 * If you use one of the helper crates then you will just implement `UiBuilder` and get a `Ui` for
 * free.
 *
 * # Conventions
 * This crate follows a series of naming conventions to make the API more predictable,
 * particularly with the [`Ui`] member functions:
 *  * A [`Pushable`] is any value that can be made active by a _push_ function and inactive by a
 *    corresponding _pop_ function. Examples are styles, colors, fonts...
 *  * A [`Hashable`] is any value that can be used to build an ImGui hash id. Ideally there should
 *    be one of these everywhere, but the Dear ImGui API it not totally othogonal here...
 *  * A function without special prefix or suffix does the same thing as its Dear ImGui
 *    counterpart. For example [`Ui::button`] calls `ImGui_Button`.
 *  * A function name that contains the `with` word takes a function that is called immediately. It
 *    corresponds to a pair of `*Begin` and `*End` functions in Dear ImGui. The function is called
 *    between these two functions. The value returned will be that of the function.
 *      * If the function is called based on some condition, such as with `ImGui_BeginChild`, then there
 *        will be another function with prefix `with_always_` that takes a function with a bool
 *        argument `opened: bool`, that can be used if you need the function to be called even if the
 *        condition is not met.
 *  * A function name that ends as `_config` will create a builder object (with the `must_use`
 *    annotation). This object will have a few properties to be set and a `build` or a
 *    `with` function to create the actual UI element.
 *  * Most builder object have a `push_for_begin` function, that will set up the pushable to be
 *    used only for the `begin` part of the UI. This is useful for example to set up the style for a
 *    window but not for its contents.
 *
 * When a function takes a value of type `String` this crate will usually take a generic `impl IntoCStr`.
 * This is an optimization that allows you to pass either a `String`, a `&str`, a `CString` or a
 * `&CStr`, avoiding an extra allocation if it is not really necessary. If you pass a constant
 * string and have a recent Rust compiler you can pass a literal `CStr` with the new syntax `c"hello"`.
 *
 *
 *
 *
 * [dearimgui]: https://github.com/ocornut/imgui
 * [imguirs]: https://github.com/imgui-rs/imgui-rs
 * [examples]: ../../../easy-imgui/examples
 */

// Too many unsafes ahead
#![allow(clippy::missing_safety_doc, clippy::too_many_arguments)]

pub use cgmath;
use easy_imgui_sys::*;
use std::borrow::Cow;
use std::cell::RefCell;
use std::ffi::{CStr, CString, OsString, c_char, c_void};
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ops::{Deref, DerefMut};
use std::ptr::{NonNull, null, null_mut};
use std::time::Duration;

// Adds `repr(transparent)` and basic conversions
macro_rules! transparent_options {
    ( $($options:ident)* ; $(#[$attr:meta])* $vis:vis struct $outer:ident ( $inner:ident); ) => {
        $(#[$attr])*
        #[repr(transparent)]
        $vis struct $outer($inner);

        $( transparent_options! { @OPTS $options $outer $inner } )*

        impl $outer {
            /// Converts a native reference into a wrapper reference.
            pub fn cast(r: &$inner) -> &$outer {
                unsafe { &*<*const $inner>::cast(r) }
            }

            /// Converts a native reference into a wrapper reference.
            ///
            /// It is safe because if you have a reference to the native reference, you already can change anything.
            pub fn cast_mut(r: &mut $inner) -> &mut $outer {
                unsafe { &mut *<*mut $inner>::cast(r) }
            }
        }
    };

    ( @OPTS Deref $outer:ident $inner:ident) => {
        impl std::ops::Deref for $outer {
            type Target = $inner;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl $outer {
            /// Gets a reference to the native wrapper struct.
            pub fn get(&self) -> &$inner {
                &self.0
            }
        }
    };

    ( @OPTS DerefMut $outer:ident $inner:ident) => {
        impl std::ops::DerefMut for $outer {
            fn deref_mut(&mut self) -> &mut $inner {
                &mut self.0
            }
        }
        impl $outer {
            pub fn get_mut(&mut self) -> &mut $inner {
                &mut self.0
            }
        }

    };
}

// This adds a Defer<Target $inner>.
macro_rules! transparent {
    ( $($tt:tt)* ) => {
         transparent_options! { Deref ; $($tt)* }
    };
}

// This adds a DerefMut in addition to the Defer<Target $inner>.
macro_rules! transparent_mut {
    ( $($tt:tt)* ) => {
         transparent_options! { Deref DerefMut ; $($tt)* }
    };
}

/// A type alias of the `cgmath::Vector2<f32>`.
///
/// Used in this crate to describe a 2D position or size.
/// The equivalent type in Dear ImGui would be [`ImVec2`].
pub type Vector2 = cgmath::Vector2<f32>;

mod enums;
mod fontloader;
mod multisel;
pub mod style;

pub use easy_imgui_sys::{self, ImGuiID, ImGuiSelectionUserData};
pub use enums::*;
pub use fontloader::{GlyphBuildFlags, GlyphLoader, GlyphLoaderArg};
pub use image;
pub use mint;
pub use multisel::*;

use image::GenericImage;

// Here we use a "generation value" to avoid calling stale callbacks. It shouldn't happen, but just
// in case, as it would cause undefined behavior.
// The generation value is taken from the ImGui frame number, that is increased every frame.
// The callback id itself is composed by combining the callback index with the generation value.
// When calling a callback, if the generation value does not match the callback is ignored.
const GEN_BITS: u32 = 8;
const GEN_ID_BITS: u32 = usize::BITS - GEN_BITS;
const GEN_MASK: usize = (1 << GEN_BITS) - 1;
const GEN_ID_MASK: usize = (1 << GEN_ID_BITS) - 1;

fn merge_generation(id: usize, gen_id: usize) -> usize {
    if (id & GEN_ID_MASK) != id {
        panic!("UI callback overflow")
    }
    (gen_id << GEN_ID_BITS) | id
}
fn remove_generation(id: usize, gen_id: usize) -> Option<usize> {
    if (id >> GEN_ID_BITS) != (gen_id & GEN_MASK) {
        None
    } else {
        Some(id & GEN_ID_MASK)
    }
}

/// Helper function to create a `Vector2`.
pub fn to_v2(v: impl Into<mint::Vector2<f32>>) -> Vector2 {
    let v = v.into();
    Vector2 { x: v.x, y: v.y }
}
/// Helper function to create a `Vector2`.
pub const fn vec2(x: f32, y: f32) -> Vector2 {
    Vector2 { x, y }
}
/// Helper function to create a `ImVec2`.
pub const fn im_vec2(x: f32, y: f32) -> ImVec2 {
    ImVec2 { x, y }
}
/// Helper function to create a `ImVec2`.
pub fn v2_to_im(v: impl Into<Vector2>) -> ImVec2 {
    let v = v.into();
    ImVec2 { x: v.x, y: v.y }
}
/// Helper function to create a `Vector2`.
pub fn im_to_v2(v: impl Into<ImVec2>) -> Vector2 {
    let v = v.into();
    Vector2 { x: v.x, y: v.y }
}

/// A color is stored as a `[r, g, b, a]`, each value between 0.0 and 1.0.
#[derive(Debug, Copy, Clone, PartialEq)]
#[repr(C)]
pub struct Color {
    /// Red component
    pub r: f32,
    /// Green component
    pub g: f32,
    /// Blue component
    pub b: f32,
    /// Alpha component
    pub a: f32,
}
impl Color {
    // Primary and secondary colors
    /// Transparent color: rgba(0, 0, 0, 0)
    pub const TRANSPARENT: Color = Color::new(0.0, 0.0, 0.0, 0.0);
    /// White color: rgba(255, 255, 255, 1)
    pub const WHITE: Color = Color::new(1.0, 1.0, 1.0, 1.0);
    /// Black color: rgba(0, 0, 0, 1)
    pub const BLACK: Color = Color::new(0.0, 0.0, 0.0, 1.0);
    /// Red color: rgba(255, 0, 0, 1)
    pub const RED: Color = Color::new(1.0, 0.0, 0.0, 1.0);
    /// Green color: rgba(0, 255, 0, 1)
    pub const GREEN: Color = Color::new(0.0, 1.0, 0.0, 1.0);
    /// Blue color: rgba(0, 0, 255, 1)
    pub const BLUE: Color = Color::new(0.0, 0.0, 1.0, 1.0);
    /// Yellow color: rgba(255, 255, 0, 1)
    pub const YELLOW: Color = Color::new(1.0, 1.0, 0.0, 1.0);
    /// Magenta color: rgba(255, 0, 255, 1)
    pub const MAGENTA: Color = Color::new(1.0, 0.0, 1.0, 1.0);
    /// Cyan color: rgba(0, 255, 255, 1)
    pub const CYAN: Color = Color::new(0.0, 1.0, 1.0, 1.0);

    /// Builds a new color from its components
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Color {
        Color { r, g, b, a }
    }
    /// Converts a `Color` into a packed `u32` value, required by some Dear ImGui functions.
    pub fn as_u32(&self) -> u32 {
        unsafe { ImGui_ColorConvertFloat4ToU32(&(*self).into()) }
    }
}
impl AsRef<[f32; 4]> for Color {
    fn as_ref(&self) -> &[f32; 4] {
        // SAFETY: Self is repr(C) so layout compatible with an array
        unsafe { std::mem::transmute::<&Color, &[f32; 4]>(self) }
    }
}
impl AsMut<[f32; 4]> for Color {
    fn as_mut(&mut self) -> &mut [f32; 4] {
        // SAFETY: Self is repr(C) so layout compatible with an array
        unsafe { std::mem::transmute::<&mut Color, &mut [f32; 4]>(self) }
    }
}
impl From<ImVec4> for Color {
    #[inline]
    fn from(c: ImVec4) -> Color {
        Color::new(c.x, c.y, c.z, c.w)
    }
}
impl From<Color> for ImVec4 {
    #[inline]
    fn from(c: Color) -> ImVec4 {
        ImVec4 {
            x: c.r,
            y: c.g,
            z: c.b,
            w: c.a,
        }
    }
}

/// The main ImGui context.
pub struct Context {
    imgui: NonNull<RawContext>,
    ini_file_name: Option<CString>,
}

/// A context that we are sure is made current.
pub struct CurrentContext<'a> {
    ctx: &'a mut Context,
}

/// Builder for a `Context`.
///
/// Call `build()` to build the context.
#[derive(Debug)]
pub struct ContextBuilder {
    docking: bool,
    viewports: bool,
    debug_highlight_id_conflicts: bool,
    ini_file_name: Option<String>,
}

impl Default for ContextBuilder {
    fn default() -> ContextBuilder {
        ContextBuilder::new()
    }
}

impl ContextBuilder {
    /// Creates a builder with default values.
    ///
    /// Defaults are:
    /// * no docking
    /// * no viewports
    /// * hightlight ids only on debug builds
    /// * ini file name disabled
    pub fn new() -> ContextBuilder {
        ContextBuilder {
            docking: false,
            viewports: false,
            debug_highlight_id_conflicts: cfg!(debug_assertions),
            ini_file_name: None,
        }
    }
    /// Sets the docking feature
    pub fn set_docking(&mut self, docking: bool) -> &mut Self {
        self.docking = docking;
        self
    }
    /// Sets the viewports feature.
    ///
    /// Note that this will only work if your used backend supports viewports, which easy-imgui-window does not.
    pub fn set_viewports(&mut self, viewports: bool) -> &mut Self {
        self.viewports = viewports;
        self
    }
    /// Sets the debug highlight id
    pub fn set_debug_highlight_id_conflicts(
        &mut self,
        debug_highlight_id_conflicts: bool,
    ) -> &mut Self {
        self.debug_highlight_id_conflicts = debug_highlight_id_conflicts;
        self
    }
    /// Sets the ini file name
    pub fn set_ini_file_name(&mut self, ini_file_name: Option<&str>) -> &mut Self {
        self.ini_file_name = ini_file_name.map(|s| s.to_string());
        self
    }
    /// Builds the ImGui context.
    ///
    /// SAFETY: read `Context::new()`.
    #[must_use]
    pub unsafe fn build(&self) -> Context {
        let imgui;
        // Probably not needed but just in case
        unsafe {
            imgui = ImGui_CreateContext(null_mut());
            ImGui_SetCurrentContext(imgui);
        }
        let imgui = NonNull::new(imgui).unwrap();
        let mut ctx = Context {
            imgui: imgui.cast(),
            ini_file_name: None,
        };
        ctx.set_ini_file_name(self.ini_file_name.as_deref());

        let io = ctx.io_mut();
        io.font_atlas_mut().0.TexPixelsUseColors = true;

        let io = unsafe { io.inner() };

        if self.docking {
            io.add_config_flags(ConfigFlags::DockingEnable);
        }
        if self.viewports {
            io.add_config_flags(ConfigFlags::ViewportsEnable);
        }
        io.ConfigDpiScaleFonts = true;
        io.ConfigDebugHighlightIdConflicts = self.debug_highlight_id_conflicts;
        ctx
    }
}

impl Context {
    /// Creates a new ImGui context with default values.
    ///
    /// SAFETY: It is unsafe because it makes the context current, and that may brake the current context
    /// if called at the wrong time.
    pub unsafe fn new() -> Context {
        unsafe { ContextBuilder::new().build() }
    }

    /// Sets the size and scale of the context area.
    pub unsafe fn set_size(&mut self, size: Vector2, scale: f32) {
        unsafe {
            self.io_mut().inner().DisplaySize = v2_to_im(size);
            if self.io().display_scale() != scale {
                self.io_mut().inner().DisplayFramebufferScale = ImVec2 { x: scale, y: scale };
            }
        }
    }

    /// Makes this context the current one.
    ///
    /// SAFETY: Do not make two different contexts current at the same time
    /// in the same thread.
    pub unsafe fn set_current(&mut self) -> CurrentContext<'_> {
        unsafe {
            ImGui_SetCurrentContext(self.imgui.as_mut().inner());
            CurrentContext { ctx: self }
        }
    }

    /// Sets the ini file where ImGui persists its data.
    ///
    /// By default is None, that means no file is saved.
    pub fn set_ini_file_name(&mut self, ini_file_name: Option<&str>) {
        let Some(ini_file_name) = ini_file_name else {
            self.ini_file_name = None;
            unsafe {
                self.io_mut().inner().IniFilename = null();
            }
            return;
        };

        let Ok(ini) = CString::new(ini_file_name) else {
            // NUL in the file namem, ignored
            return;
        };

        let ini = self.ini_file_name.insert(ini);
        unsafe {
            self.io_mut().inner().IniFilename = ini.as_ptr();
        }
    }
    /// Gets the ini file set previously by `set_ini_file_name`.
    pub fn ini_file_name(&self) -> Option<&str> {
        let ini = self.ini_file_name.as_deref()?.to_str().unwrap_or_default();
        Some(ini)
    }
}

impl CurrentContext<'_> {
    /// Builds and renders a UI frame.
    ///
    /// * `app`: `UiBuilder` to be used to build the frame.
    /// * `re_render`: function to be called after `app.do_ui` but before rendering.
    /// * `render`: function to do the actual render.
    pub unsafe fn do_frame<A: UiBuilder>(
        &mut self,
        app: &mut A,
        pre_render: impl FnOnce(&mut Self),
        render: impl FnOnce(&ImDrawData),
    ) {
        unsafe {
            let mut ui = Ui {
                imgui: self.ctx.imgui,
                data: std::ptr::null_mut(),
                generation: ImGui_GetFrameCount() as usize % 1000 + 1, // avoid the 0
                callbacks: RefCell::new(Vec::new()),
            };

            self.io_mut().inner().BackendLanguageUserData =
                (&raw const ui).cast::<c_void>().cast_mut();
            struct UiPtrToNullGuard<'a, 'b>(&'a mut CurrentContext<'b>);
            impl Drop for UiPtrToNullGuard<'_, '_> {
                fn drop(&mut self) {
                    unsafe {
                        self.0.io_mut().inner().BackendLanguageUserData = null_mut();
                    }
                }
            }
            let ctx_guard = UiPtrToNullGuard(self);

            // This guards for panics during the frame.
            struct FrameGuard;
            impl Drop for FrameGuard {
                fn drop(&mut self) {
                    unsafe {
                        ImGui_EndFrame();
                    }
                }
            }

            ImGui_NewFrame();

            let end_frame_guard = FrameGuard;
            app.do_ui(&ui);
            std::mem::drop(end_frame_guard);

            pre_render(ctx_guard.0);
            app.pre_render(ctx_guard.0);

            ImGui_Render();

            ui.data = app;

            // This is the same pointer, but without it, there is something fishy about stacked borrows
            // and the mutable access to `ui` above.
            ctx_guard.0.io_mut().inner().BackendLanguageUserData =
                (&raw const ui).cast::<c_void>().cast_mut();

            let draw_data = ImGui_GetDrawData();
            render(&*draw_data);
        }
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            ImGui_DestroyContext(self.imgui.as_mut().inner());
        }
    }
}

transparent! {
    /// Safe thin wrapper for ImGuiContext.
    ///
    /// This has common read-only functions.
    pub struct RawContext(ImGuiContext);
}

impl RawContext {
    /// Gets the current ImGui context.
    ///
    /// SAFETY: unsafe because the reference lifetime is not well defined.
    #[inline]
    pub unsafe fn current<'a>() -> &'a RawContext {
        unsafe { RawContext::cast(&*ImGui_GetCurrentContext()) }
    }
    /// Converts a raw DearImGui context pointer into a `&RawContext``.
    #[inline]
    pub unsafe fn from_ptr<'a>(ptr: *mut ImGuiContext) -> &'a RawContext {
        unsafe { RawContext::cast(&*ptr) }
    }
    /// Converts a raw DearImGui context pointer into a `&mut RawContext``.
    #[inline]
    pub unsafe fn from_ptr_mut<'a>(ptr: *mut ImGuiContext) -> &'a mut RawContext {
        unsafe { RawContext::cast_mut(&mut *ptr) }
    }
    /// Gets a reference to the actual DearImGui context struct.
    #[inline]
    pub unsafe fn inner(&mut self) -> &mut ImGuiContext {
        &mut self.0
    }
    /// Returns a safe wrapper for the `PlatformIo`.
    #[inline]
    pub fn platform_io(&self) -> &PlatformIo {
        PlatformIo::cast(&self.PlatformIO)
    }
    /// Returns an unsafe mutable wrapper for the `PlatformIo`.
    #[inline]
    pub unsafe fn platform_io_mut(&mut self) -> &mut PlatformIo {
        unsafe { PlatformIo::cast_mut(&mut self.inner().PlatformIO) }
    }

    /// Returns a safe wrapper for the IO.
    #[inline]
    pub fn io(&self) -> &Io {
        Io::cast(&self.IO)
    }
    /// Returns a safe mutable wrapper for the IO.
    ///
    /// Use `io_mut().inner()` to get the unsafe wrapper
    #[inline]
    pub fn io_mut(&mut self) -> &mut IoMut {
        unsafe { IoMut::cast_mut(&mut self.inner().IO) }
    }
    /// Returns a reference for the current style definition.
    #[inline]
    pub fn style(&self) -> &style::Style {
        style::Style::cast(&self.Style)
    }
    /// Gets a mutable reference to the style definition.
    #[inline]
    pub fn style_mut(&mut self) -> &mut style::Style {
        // SAFETY: Changing the style is only unsafe during the frame (use pushables there),
        // but during a frame the context is borrowed inside the `&Ui`, that is immutable.
        unsafe { style::Style::cast_mut(&mut self.inner().Style) }
    }

    /// Gets a reference to the main viewport
    pub fn get_main_viewport(&self) -> &Viewport {
        unsafe {
            let ptr = (*self.Viewports)[0];
            Viewport::cast(&(*ptr)._base)
        }
    }
}

impl Deref for Context {
    type Target = RawContext;
    fn deref(&self) -> &RawContext {
        unsafe { self.imgui.as_ref() }
    }
}
impl DerefMut for Context {
    fn deref_mut(&mut self) -> &mut RawContext {
        unsafe { self.imgui.as_mut() }
    }
}

impl Deref for CurrentContext<'_> {
    type Target = RawContext;
    fn deref(&self) -> &RawContext {
        self.ctx
    }
}
impl DerefMut for CurrentContext<'_> {
    fn deref_mut(&mut self) -> &mut RawContext {
        self.ctx
    }
}

impl<A> Deref for Ui<A> {
    type Target = RawContext;
    fn deref(&self) -> &RawContext {
        unsafe { self.imgui.as_ref() }
    }
}

// No DerefMut for Ui, sorry.

/// The main trait that the user must implement to create a UI.
pub trait UiBuilder {
    /// This function is run after `do_ui` but before rendering.
    ///
    /// It can be used to clear the framebuffer, or prerender something.
    fn pre_render(&mut self, _ctx: &mut CurrentContext<'_>) {}
    /// User the `ui` value to create a UI frame.
    ///
    /// This is equivalent to the Dear ImGui code between `NewFrame` and `EndFrame`.
    fn do_ui(&mut self, ui: &Ui<Self>);
}

enum TtfData {
    Bytes(Cow<'static, [u8]>),
    DefaultFont,
    CustomLoader(fontloader::BoxGlyphLoader),
}

/// A font to be fed to the ImGui atlas.
pub struct FontInfo {
    ttf: TtfData,
    size: f32,
    name: String,
    flags: FontFlags,
}

impl FontInfo {
    /// Creates a new `FontInfo` from a TTF content and a font size.
    pub fn new(ttf: impl Into<Cow<'static, [u8]>>) -> FontInfo {
        FontInfo {
            ttf: TtfData::Bytes(ttf.into()),
            size: 0.0, // Default from DearImGui
            name: String::new(),
            flags: FontFlags::None,
        }
    }
    /// Sets the name of this font.
    ///
    /// It is used only for diagnostics and the "demo" window.
    pub fn set_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }
    /// Sets the legacy size of this font.
    ///
    /// The size of the default font (the first one registered) is saved
    /// as the default font size. Any other font size is not actually used,
    /// although it is visible as `Font:.LegacySize`.
    pub fn set_size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    /// Creates a `FontInfo` using the embedded default Dear ImGui font.
    pub fn default_font() -> FontInfo {
        FontInfo {
            ttf: TtfData::DefaultFont,
            size: 0.0,
            name: String::new(),
            flags: FontFlags::None,
        }
    }
    /// Registers a custom font loader.
    ///
    /// A custom font loader is any static type that implements the trait `GlyphLoader`.
    pub fn custom<GL: GlyphLoader + 'static>(glyph_loader: GL) -> FontInfo {
        let t = fontloader::BoxGlyphLoader::from(Box::new(glyph_loader));
        FontInfo {
            ttf: TtfData::CustomLoader(t),
            size: 0.0,
            name: String::new(),
            flags: FontFlags::None,
        }
    }
}

/// Represents any type that can be converted into something that can be deref'ed to a `&CStr`.
pub trait IntoCStr: Sized {
    /// The type that can actually be converted into a `CStr`.
    type Temp: Deref<Target = CStr>;
    /// Convert this value into a `Temp` that can be converted into a `CStr`.
    fn into(self) -> Self::Temp;
    /// Convert this value directly into a `CString`.
    fn into_cstring(self) -> CString;
    /// Length in bytes of the `CString` within.
    fn len(&self) -> usize;
    /// Checks whether the string is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Adds the bytes of the `CStr` within to the given `Vec`.
    ///
    /// SAFETY: Unsafe because we will not check there are no NULs.
    /// Reimplement this if `fn into()` does extra allocations.
    unsafe fn push_to_non_null_vec(self, bs: &mut Vec<u8>) {
        let c = IntoCStr::into(self);
        let c = c.to_bytes();
        bs.extend(c);
    }
}

impl IntoCStr for &str {
    type Temp = CString;

    fn into(self) -> Self::Temp {
        CString::new(self).unwrap()
    }
    fn into_cstring(self) -> CString {
        IntoCStr::into(self)
    }
    fn len(&self) -> usize {
        str::len(self)
    }
    unsafe fn push_to_non_null_vec(self, bs: &mut Vec<u8>) {
        let c = self.as_bytes();
        if c.contains(&0) {
            panic!("NUL error");
        }
        bs.extend(c);
    }
}
impl IntoCStr for &String {
    type Temp = CString;

    fn into(self) -> Self::Temp {
        CString::new(self.as_str()).unwrap()
    }
    fn into_cstring(self) -> CString {
        IntoCStr::into(self)
    }
    fn len(&self) -> usize {
        self.as_str().len()
    }
    unsafe fn push_to_non_null_vec(self, bs: &mut Vec<u8>) {
        unsafe {
            self.as_str().push_to_non_null_vec(bs);
        }
    }
}
impl IntoCStr for String {
    type Temp = CString;

    fn into(self) -> Self::Temp {
        CString::new(self).unwrap()
    }
    fn into_cstring(self) -> CString {
        IntoCStr::into(self)
    }
    fn len(&self) -> usize {
        self.len()
    }
}
impl IntoCStr for &CStr {
    type Temp = Self;
    fn into(self) -> Self {
        self
    }
    fn into_cstring(self) -> CString {
        self.to_owned()
    }
    fn len(&self) -> usize {
        self.to_bytes().len()
    }
}
impl IntoCStr for CString {
    type Temp = Self;

    fn into(self) -> Self {
        self
    }
    fn into_cstring(self) -> CString {
        self
    }
    fn len(&self) -> usize {
        self.as_bytes().len()
    }
}
impl<'a> IntoCStr for &'a CString {
    type Temp = &'a CStr;

    fn into(self) -> &'a CStr {
        self.as_c_str()
    }
    fn into_cstring(self) -> CString {
        self.clone()
    }
    fn len(&self) -> usize {
        self.as_c_str().len()
    }
}

impl<'a, B> IntoCStr for Cow<'a, B>
where
    B: 'a + ToOwned + ?Sized,
    &'a B: IntoCStr,
    B::Owned: IntoCStr,
    <&'a B as IntoCStr>::Temp: Into<Cow<'a, CStr>>,
{
    type Temp = Cow<'a, CStr>;

    fn into(self) -> Cow<'a, CStr> {
        match self {
            Cow::Owned(o) => Cow::Owned(IntoCStr::into_cstring(o)),
            Cow::Borrowed(b) => IntoCStr::into(b).into(),
        }
    }
    fn into_cstring(self) -> CString {
        match self {
            Cow::Owned(o) => o.into_cstring(),
            Cow::Borrowed(b) => b.into_cstring(),
        }
    }
    fn len(&self) -> usize {
        match self {
            Cow::Owned(o) => o.len(),
            Cow::Borrowed(b) => b.len(),
        }
    }
}

/// A string that works as a widget identifier.
///
/// Think of for example `"###ok"`.
///
/// Use the function `id()` to build it.
pub struct Id<C: IntoCStr>(C);

/// A string that works as both a label and identifier.
///
/// For example `"Enter the data###data1"`.
///
/// Prefer to use function `lbl_id()` function to construct it from two strings.
///
/// Or use function `lbl()` to build it from an old compatible `label###id`.
pub struct LblId<C: IntoCStr>(C);

impl<C: IntoCStr> Id<C> {
    /// Converts this `Id` into a value that can be converted to a `CStr`.
    pub fn into(self) -> C::Temp {
        self.0.into()
    }
    /// Converts this `Id` into a `IntoCStr`.
    pub fn into_inner(self) -> C {
        self.0
    }
}

impl<C: IntoCStr> LblId<C> {
    /// Converts this `LblId` into a value that can be converted to a `CStr`.
    pub fn into(self) -> C::Temp {
        self.0.into()
    }
    /// Converts this `LblId` into a `IntoCStr`.
    pub fn into_inner(self) -> C {
        self.0
    }
}

/// Uses the given string as an ImGui id.
///
/// It prepends `###`, for consistency with `lbl_id()`.
pub fn id<C: IntoCStr>(c: C) -> Id<CString> {
    let mut bs = Vec::with_capacity(c.len() + 4);
    bs.push(b'#');
    bs.push(b'#');
    bs.push(b'#');
    // SAFETY:
    // Converts one CString into another CString with the ### prefix.
    unsafe {
        IntoCStr::push_to_non_null_vec(c, &mut bs);
        Id(CString::from_vec_unchecked(bs))
    }
}

/// Uses the given string as an ImGui id, without prepending `###`.
pub fn raw_id<C: IntoCStr>(c: C) -> Id<C> {
    Id(c)
}

/// Same as the `raw_id()` function, but may be easier to use.
///
/// This will be recommended by the compiler if you do it wrong.
impl<C: IntoCStr> From<C> for Id<C> {
    fn from(c: C) -> Id<C> {
        Id(c)
    }
}

/// Uses the given string directly as an ImGui parameter that contains a label plus an id.
///
/// The usual Dear ImGui syntax applies:
///  * `"hello"`: is both a label and an id.
///  * `"hello##world"`: the label is `hello`, the id is the whole string`.
///  * `"hello###world"`: the label is `hello`, the id is `###world`.
pub fn lbl<C: IntoCStr>(c: C) -> LblId<C> {
    LblId(c)
}

/// Uses the first string as label, the second one as id.
///
/// The id has `###` prepended.
pub fn lbl_id<C1: IntoCStr, C2: IntoCStr>(lbl: C1, id: C2) -> LblId<CString> {
    let lbl = lbl.into_cstring();
    let both = if id.is_empty() {
        lbl
    } else {
        let mut bs = lbl.into_bytes();
        bs.extend(b"###");
        // SAFETY:
        // bs can't have NULs, and the `push_to_non_null_vec` safe requirements forbids extra NULs.
        // We add one NUL at the end, so all is good.
        unsafe {
            IntoCStr::push_to_non_null_vec(id, &mut bs);
            CString::from_vec_unchecked(bs)
        }
    };
    LblId(both)
}

/// Same as the `lbl()` function, but may be easier to use.
///
/// This will be recommended by the compiler if you do it wrong.
impl<C: IntoCStr> From<C> for LblId<C> {
    fn from(c: C) -> LblId<C> {
        LblId(c)
    }
}

// Helper functions

// Take care to not consume the argument before using the pointer
fn optional_str<S: Deref<Target = CStr>>(t: &Option<S>) -> *const c_char {
    t.as_ref().map(|s| s.as_ptr()).unwrap_or(null())
}

fn optional_mut_bool(b: &mut Option<&mut bool>) -> *mut bool {
    b.as_mut().map(|x| *x as *mut bool).unwrap_or(null_mut())
}

/// Helper function that, given a string, returns the start and end pointer.
unsafe fn text_ptrs(text: &str) -> (*const c_char, *const c_char) {
    let btxt = text.as_bytes();
    let start = btxt.as_ptr() as *const c_char;
    let end = unsafe { start.add(btxt.len()) };
    (start, end)
}

unsafe fn current_font_ptr(font: FontId) -> *mut ImFont {
    unsafe {
        let fonts = RawContext::current().io().font_atlas();
        fonts.font_ptr(font)
    }
}

// this is unsafe because it replaces a C binding function that does nothing, and adding `unsafe`
// avoids a warning
unsafe fn no_op() {}

/// `Ui` represents an ImGui frame that is being built.
///
/// Usually you will get a `&mut Ui` when you are expected to build a user interface,
/// as in [`UiBuilder::do_ui`].
pub struct Ui<A>
where
    A: ?Sized,
{
    imgui: NonNull<RawContext>,
    data: *mut A, // only for callbacks, after `do_ui` has finished, do not use directly
    generation: usize,
    callbacks: RefCell<Vec<UiCallback<A>>>,
}

/// Callbacks called during `A::do_ui()` will have the first argument as null, because the app value
/// is already `self`, no need for it.
/// Callbacks called during rendering will not have access to `Ui`, because the frame is finished,
/// but they will get a proper `&mut A` as a first argument.
/// The second is a generic pointer to the real function argument, beware of fat pointers!
/// The second parameter will be consumed by the callback, take care of calling the drop exactly
/// once.
type UiCallback<A> = Box<dyn FnMut(*mut A, *mut c_void)>;

macro_rules! with_begin_end {
    ( $(#[$attr:meta])* $name:ident $begin:ident $end:ident ($($arg:ident ($($type:tt)*) ($pass:expr),)*) ) => {
        paste::paste! {
            $(#[$attr])*
            pub fn [< with_ $name >]<R>(&self, $($arg: $($type)*,)* f: impl FnOnce() -> R) -> R {
                unsafe { $begin( $( $pass, )* ) }
                struct EndGuard;
                impl Drop for EndGuard {
                    fn drop(&mut self) {
                        unsafe { $end() }
                    }
                }
                let _guard = EndGuard;
                f()
            }
        }
    };
}

macro_rules! with_begin_end_opt {
    ( $(#[$attr:meta])* $name:ident $begin:ident $end:ident ($($arg:ident ($($type:tt)*) ($pass:expr),)*) ) => {
        paste::paste! {
            $(#[$attr])*
            pub fn [< with_ $name >]<R>(&self, $($arg: $($type)*,)* f: impl FnOnce() -> R) -> Option<R> {
                self.[< with_always_ $name >]($($arg,)* move |opened| { opened.then(f) })
            }
            pub fn [< with_always_ $name >]<R>(&self, $($arg: $($type)*,)* f: impl FnOnce(bool) -> R) -> R {
                if !unsafe { $begin( $( $pass, )* ) } {
                    return f(false);
                }
                struct EndGuard;
                impl Drop for EndGuard {
                    fn drop(&mut self) {
                        unsafe { $end() }
                    }
                }
                let _guard = EndGuard;
                f(true)
            }
        }
    };
}

macro_rules! decl_builder {
    ( $sname:ident -> $tres:ty, $func:ident ($($life:lifetime),*) ( $( $gen_n:ident : $gen_d:tt ),* )
        (
            $(
                $arg:ident ($($ty:tt)*) ($pass:expr),
            )*
        )
        { $($extra:tt)* }
        { $($constructor:tt)* }
    ) => {
        #[must_use]
        pub struct $sname<'s, $($life,)* $($gen_n : $gen_d, )* > {
            _pd: PhantomData<*const &'s ()>, // !Send + !Sync
            $(
                $arg: $($ty)*,
            )*
        }
        impl <'s, $($life,)* $($gen_n : $gen_d, )* > $sname<'s, $($life,)* $($gen_n, )* > {
            pub fn build(self) -> $tres {
                let $sname { _pd, $($arg, )* } = self;
                unsafe {
                    $func($($pass,)*)
                }
            }
            $($extra)*
        }

        impl<A> Ui<A> {
            $($constructor)*
        }
    };
}

macro_rules! decl_builder_setter_ex {
    ($name:ident: $ty:ty = $expr:expr) => {
        pub fn $name(mut self, $name: $ty) -> Self {
            self.$name = $expr;
            self
        }
    };
}

macro_rules! decl_builder_setter {
    ($name:ident: $ty:ty) => {
        decl_builder_setter_ex! { $name: $ty = $name.into() }
    };
}

macro_rules! decl_builder_setter_vector2 {
    ($name:ident: Vector2) => {
        decl_builder_setter_ex! { $name: Vector2 = v2_to_im($name) }
    };
}

macro_rules! decl_builder_with_maybe_opt {
    ( $always_run_end:literal
      $sname:ident, $func_beg:ident, $func_end:ident ($($life:lifetime),*) ( $( $gen_n:ident : $gen_d:tt ),* )
        (
            $(
                $arg:ident ($($ty:tt)*) ($pass:expr),
            )*
        )
        { $($extra:tt)* }
        { $($constructor:tt)* }
    ) => {
        #[must_use]
        pub struct $sname< $($life,)* $($gen_n : $gen_d, )* P: Pushable = () > {
            $(
                $arg: $($ty)*,
            )*
            push: P,
        }
        impl <$($life,)* $($gen_n : $gen_d, )* P: Pushable > $sname<$($life,)* $($gen_n,)* P > {
            /// Registers this `Pushable` to be called only for the `begin` part of this UI
            /// element.
            ///
            /// This is useful for example to modify the style of a window without changing the
            /// stily of its content.
            pub fn push_for_begin<P2: Pushable>(self, push: P2) -> $sname< $($life,)* $($gen_n,)* (P, P2) > {
                $sname {
                    $(
                        $arg: self.$arg,
                    )*
                    push: (self.push, push),
                }
            }
            /// Calls `f` inside this UI element, but only if it is visible.
            pub fn with<R>(self, f: impl FnOnce() -> R) -> Option<R> {
                self.with_always(move |opened| { opened.then(f) })
            }
            /// Calls `f` inside this UI element, passing `true` if the elements visible, `false`
            /// if it is not.
            pub fn with_always<R>(self, f: impl FnOnce(bool) -> R) -> R {
                // Some uses will require `mut`, some will not`
                #[allow(unused_mut)]
                let $sname { $(mut $arg, )* push } = self;
                let bres = unsafe {
                    let _guard = push_guard(&push);
                    $func_beg($($pass,)*)
                };
                struct EndGuard(bool);
                impl Drop for EndGuard {
                    fn drop(&mut self) {
                        if self.0 {
                            unsafe { $func_end(); }
                        }
                    }
                }
                let _guard_2 = EndGuard($always_run_end || bres);
                f(bres)
            }
            $($extra)*
        }

        impl<A> Ui<A> {
            $($constructor)*
        }
    };
}

macro_rules! decl_builder_with {
    ($($args:tt)*) => {
        decl_builder_with_maybe_opt!{ true $($args)* }
    };
}

macro_rules! decl_builder_with_opt {
    ($($args:tt)*) => {
        decl_builder_with_maybe_opt!{ false $($args)* }
    };
}

decl_builder_with! {Child, ImGui_BeginChild, ImGui_EndChild () (S: IntoCStr)
    (
        name (S::Temp) (name.as_ptr()),
        size (ImVec2) (&size),
        child_flags (ChildFlags) (child_flags.bits()),
        window_flags (WindowFlags) (window_flags.bits()),
    )
    {
        decl_builder_setter_vector2!{size: Vector2}
        decl_builder_setter!{child_flags: ChildFlags}
        decl_builder_setter!{window_flags: WindowFlags}
    }
    {
        pub fn child_config<S: IntoCStr>(&self, name: LblId<S>) -> Child<S> {
            Child {
                name: name.into(),
                size: im_vec2(0.0, 0.0),
                child_flags: ChildFlags::None,
                window_flags: WindowFlags::None,
                push: (),
            }
        }
    }
}

decl_builder_with! {Window, ImGui_Begin, ImGui_End ('v) (S: IntoCStr)
    (
        name (S::Temp) (name.as_ptr()),
        open (Option<&'v mut bool>) (optional_mut_bool(&mut open)),
        flags (WindowFlags) (flags.bits()),
    )
    {
        decl_builder_setter!{open: &'v mut bool}
        decl_builder_setter!{flags: WindowFlags}
    }
    {
        pub fn window_config<S: IntoCStr>(&self, name: LblId<S>) -> Window<'_, S> {
            Window {
                name: name.into(),
                open: None,
                flags: WindowFlags::None,
                push: (),
            }
        }
    }
}

decl_builder! { MenuItem -> bool, ImGui_MenuItem () (S1: IntoCStr, S2: IntoCStr)
    (
        label (S1::Temp) (label.as_ptr()),
        shortcut (Option<S2::Temp>) (optional_str(&shortcut)),
        selected (bool) (selected),
        enabled (bool) (enabled),
    )
    {
        pub fn shortcut_opt<S3: IntoCStr>(self, shortcut: Option<S3>) -> MenuItem<'s, S1, S3> {
            MenuItem {
                _pd: PhantomData,
                label: self.label,
                shortcut: shortcut.map(|s| s.into()),
                selected: self.selected,
                enabled: self.enabled,
            }
        }
        pub fn shortcut<S3: IntoCStr>(self, shortcut: S3) -> MenuItem<'s, S1, S3> {
            self.shortcut_opt(Some(shortcut))
        }
        decl_builder_setter!{selected: bool}
        decl_builder_setter!{enabled: bool}
    }
    {
        pub fn menu_item_config<S: IntoCStr>(&self, label: LblId<S>) -> MenuItem<'_, S, &str> {
            MenuItem {
                _pd: PhantomData,
                label: label.into(),
                shortcut: None,
                selected: false,
                enabled: true,
            }
        }
    }
}

decl_builder! { Button -> bool, ImGui_Button () (S: IntoCStr)
    (
        label (S::Temp) (label.as_ptr()),
        size (ImVec2) (&size),
    )
    {
        decl_builder_setter_vector2!{size: Vector2}
    }
    {
        pub fn button_config<S: IntoCStr>(&self, label: LblId<S>) -> Button<'_, S> {
            Button {
                _pd: PhantomData,
                label: label.into(),
                size: im_vec2(0.0, 0.0),
            }
        }
        pub fn button<S: IntoCStr>(&self, label: LblId<S>) -> bool {
            self.button_config(label).build()
        }
    }
}

decl_builder! { SmallButton -> bool, ImGui_SmallButton () (S: IntoCStr)
    (
        label (S::Temp) (label.as_ptr()),
    )
    {}
    {
        pub fn small_button_config<S: IntoCStr>(&self, label: LblId<S>) -> SmallButton<'_, S> {
            SmallButton {
                _pd: PhantomData,
                label: label.into(),
            }
        }
        pub fn small_button<S: IntoCStr>(&self, label: LblId<S>) -> bool {
            self.small_button_config(label).build()
        }
    }
}

decl_builder! { InvisibleButton -> bool, ImGui_InvisibleButton () (S: IntoCStr)
    (
        id (S::Temp) (id.as_ptr()),
        size (ImVec2) (&size),
        flags (ButtonFlags) (flags.bits()),
    )
    {
        decl_builder_setter_vector2!{size: Vector2}
        decl_builder_setter!{flags: ButtonFlags}
    }
    {
        pub fn invisible_button_config<S: IntoCStr>(&self, id: S) -> InvisibleButton<'_, S> {
            InvisibleButton {
                _pd: PhantomData,
                id: id.into(),
                size: im_vec2(0.0, 0.0),
                flags: ButtonFlags::MouseButtonLeft,
            }
        }
    }
}

decl_builder! { ArrowButton -> bool, ImGui_ArrowButton () (S: IntoCStr)
    (
        id (S::Temp) (id.as_ptr()),
        dir (Dir) (dir.bits()),
    )
    {}
    {
        pub fn arrow_button_config<S: IntoCStr>(&self, id: S, dir: Dir) -> ArrowButton<'_, S> {
            ArrowButton {
                _pd: PhantomData,
                id: id.into(),
                dir,
            }
        }
        pub fn arrow_button<S: IntoCStr>(&self, id: S, dir: Dir) -> bool {
            self.arrow_button_config(id, dir).build()
        }
    }
}

decl_builder! { Checkbox -> bool, ImGui_Checkbox ('v) (S: IntoCStr)
    (
        label (S::Temp) (label.as_ptr()),
        value (&'v mut bool) (value),
    )
    {}
    {
        pub fn checkbox_config<'v, S: IntoCStr>(&self, label: LblId<S>, value: &'v mut bool) -> Checkbox<'_, 'v, S> {
            Checkbox {
                _pd: PhantomData,
                label: label.into(),
                value,
            }
        }
        pub fn checkbox<S: IntoCStr>(&self, label: LblId<S>, value: &mut bool) -> bool {
            self.checkbox_config(label, value).build()
        }
    }
}

decl_builder! { RadioButton -> bool, ImGui_RadioButton () (S: IntoCStr)
    (
        label (S::Temp) (label.as_ptr()),
        active (bool) (active),
    )
    {}
    {
        pub fn radio_button_config<S: IntoCStr>(&self, label: LblId<S>, active: bool) -> RadioButton<'_, S> {
            RadioButton {
                _pd: PhantomData,
                label: label.into(),
                active,
            }
        }
    }
}

decl_builder! { ProgressBar -> (), ImGui_ProgressBar () (S: IntoCStr)
    (
        fraction (f32) (fraction),
        size (ImVec2) (&size),
        overlay (Option<S::Temp>) (optional_str(&overlay)),
    )
    {
        decl_builder_setter_vector2!{size: Vector2}
        pub fn overlay<S2: IntoCStr>(self, overlay: S2) -> ProgressBar<'s, S2> {
            ProgressBar {
                _pd: PhantomData,
                fraction: self.fraction,
                size: self.size,
                overlay: Some(overlay.into()),
            }
        }
    }
    {
        pub fn progress_bar_config<'a>(&self, fraction: f32) -> ProgressBar<'_, &'a str> {
            ProgressBar {
                _pd: PhantomData,
                fraction,
                size: im_vec2(-f32::MIN_POSITIVE, 0.0),
                overlay: None,
            }
        }
    }
}

decl_builder! { Image -> (), ImGui_Image ('t) ()
    (
        texture_ref (TextureRef<'t>) (texture_ref.tex_ref()),
        size (ImVec2) (&size),
        uv0 (ImVec2) (&uv0),
        uv1 (ImVec2) (&uv1),
    )
    {
        decl_builder_setter_vector2!{uv0: Vector2}
        decl_builder_setter_vector2!{uv1: Vector2}
    }
    {
        pub fn image_config<'t>(&self, texture_ref: TextureRef<'t>, size: Vector2) -> Image<'_, 't> {
            Image {
                _pd: PhantomData,
                texture_ref,
                size: v2_to_im(size),
                uv0: im_vec2(0.0, 0.0),
                uv1: im_vec2(1.0, 1.0),
            }
        }
        pub fn image_with_custom_rect_config(&self, ridx: CustomRectIndex, scale: f32) -> Image<'_, '_> {
            let rr = self.get_custom_rect(ridx).unwrap();
            self.image_config(rr.tex_ref, vec2(scale * rr.rect.w as f32, scale * rr.rect.h as f32))
                .uv0(im_to_v2(rr.rect.uv0))
                .uv1(im_to_v2(rr.rect.uv1))
        }
    }
}

decl_builder! { ImageWithBg -> (), ImGui_ImageWithBg ('t) ()
    (
        texture_ref (TextureRef<'t>) (texture_ref.tex_ref()),
        size (ImVec2) (&size),
        uv0 (ImVec2) (&uv0),
        uv1 (ImVec2) (&uv1),
        bg_col (ImVec4) (&bg_col),
        tint_col (ImVec4) (&tint_col),
    )
    {
        decl_builder_setter_vector2!{uv0: Vector2}
        decl_builder_setter_vector2!{uv1: Vector2}
        decl_builder_setter!{bg_col: Color}
        decl_builder_setter!{tint_col: Color}
    }
    {
        pub fn image_with_bg_config<'t>(&self, texture_ref: TextureRef<'t>, size: Vector2) -> ImageWithBg<'_, 't> {
            ImageWithBg {
                _pd: PhantomData,
                texture_ref,
                size: v2_to_im(size),
                uv0: im_vec2(0.0, 0.0),
                uv1: im_vec2(1.0, 1.0),
                bg_col: Color::TRANSPARENT.into(),
                tint_col: Color::WHITE.into(),
            }
        }
        pub fn image_with_bg_with_custom_rect_config(&self, ridx: CustomRectIndex, scale: f32) -> ImageWithBg<'_, '_> {
            let rr = self.get_custom_rect(ridx).unwrap();
            self.image_with_bg_config(self.get_atlas_texture_ref(), vec2(scale * rr.rect.w as f32, scale * rr.rect.h as f32))
                .uv0(im_to_v2(rr.rect.uv0))
                .uv1(im_to_v2(rr.rect.uv1))
        }

    }
}

decl_builder! { ImageButton -> bool, ImGui_ImageButton ('t) (S: IntoCStr)
    (
        str_id (S::Temp) (str_id.as_ptr()),
        texture_ref (TextureRef<'t>) (texture_ref.tex_ref()),
        size (ImVec2) (&size),
        uv0 (ImVec2) (&uv0),
        uv1 (ImVec2) (&uv1),
        bg_col (ImVec4) (&bg_col),
        tint_col (ImVec4) (&tint_col),
    )
    {
        decl_builder_setter_vector2!{uv0: Vector2}
        decl_builder_setter_vector2!{uv1: Vector2}
        decl_builder_setter!{bg_col: Color}
        decl_builder_setter!{tint_col: Color}
    }
    {
        pub fn image_button_config<'t, S: IntoCStr>(&self, str_id: Id<S>, texture_ref: TextureRef<'t>, size: Vector2) -> ImageButton<'_, 't, S> {
            ImageButton {
                _pd: PhantomData,
                str_id: str_id.into(),
                texture_ref,
                size: v2_to_im(size),
                uv0: im_vec2(0.0, 0.0),
                uv1: im_vec2(1.0, 1.0),
                bg_col: Color::TRANSPARENT.into(),
                tint_col: Color::WHITE.into(),
            }
        }
        pub fn image_button_with_custom_rect_config<S: IntoCStr>(&self, str_id: Id<S>, ridx: CustomRectIndex, scale: f32) -> ImageButton<'_, '_, S> {
            let rr = self.get_custom_rect(ridx).unwrap();
            self.image_button_config(str_id, rr.tex_ref, vec2(scale * rr.rect.w as f32, scale * rr.rect.h as f32))
                .uv0(im_to_v2(rr.rect.uv0))
                .uv1(im_to_v2(rr.rect.uv1))
        }
    }
}

decl_builder! { Selectable -> bool, ImGui_Selectable () (S: IntoCStr)
    (
        label (S::Temp) (label.as_ptr()),
        selected (bool) (selected),
        flags (SelectableFlags) (flags.bits()),
        size (ImVec2) (&size),
    )
    {
        decl_builder_setter!{selected: bool}
        decl_builder_setter!{flags: SelectableFlags}
        decl_builder_setter_vector2!{size: Vector2}
    }
    {
        pub fn selectable_config<S: IntoCStr>(&self, label: LblId<S>) -> Selectable<'_, S> {
            Selectable {
                _pd: PhantomData,
                label: label.into(),
                selected: false,
                flags: SelectableFlags::None,
                size: im_vec2(0.0, 0.0),
            }
        }
        pub fn selectable<S: IntoCStr>(&self, label: LblId<S>) -> bool {
            self.selectable_config(label).build()
        }
    }
}

macro_rules! decl_builder_drag {
    ($name:ident $func:ident $cfunc:ident $life:lifetime ($argty:ty) ($ty:ty) ($expr:expr)) => {
        decl_builder! { $name -> bool, $cfunc ($life) (S: IntoCStr)
            (
                label (S::Temp) (label.as_ptr()),
                value ($ty) ($expr(value)),
                speed (f32) (speed),
                min ($argty) (min),
                max ($argty) (max),
                format (Cow<'static, CStr>) (format.as_ptr()),
                flags (SliderFlags) (flags.bits()),
            )
            {
                decl_builder_setter!{speed: f32}
                pub fn range(mut self, min: $argty, max: $argty) -> Self {
                    self.min = min;
                    self.max = max;
                    self
                }
                decl_builder_setter!{flags: SliderFlags}
            }
            {
                pub fn $func<$life, S: IntoCStr>(&self, label: LblId<S>, value: $ty) -> $name<'_, $life, S> {
                    $name {
                        _pd: PhantomData,
                        label: label.into(),
                        value,
                        speed: 1.0,
                        min: <$argty>::default(),
                        max: <$argty>::default(),
                        format: Cow::Borrowed(c"%.3f"),
                        flags: SliderFlags::None,
                    }
                }
            }
        }
    };
}

macro_rules! impl_float_format {
    ($name:ident) => {
        impl_float_format! {$name c"%g" c"%.0f" c"%.3f" "%.{}f"}
    };
    ($name:ident $g:literal $f0:literal $f3:literal $f_n:literal) => {
        impl<S: IntoCStr> $name<'_, '_, S> {
            pub fn display_format(mut self, format: FloatFormat) -> Self {
                self.format = match format {
                    FloatFormat::G => Cow::Borrowed($g),
                    FloatFormat::F(0) => Cow::Borrowed($f0),
                    FloatFormat::F(3) => Cow::Borrowed($f3),
                    FloatFormat::F(n) => Cow::Owned(CString::new(format!($f_n, n)).unwrap()),
                };
                self
            }
        }
    };
}

decl_builder_drag! { DragFloat drag_float_config ImGui_DragFloat 'v (f32) (&'v mut f32) (std::convert::identity)}
decl_builder_drag! { DragFloat2 drag_float_2_config ImGui_DragFloat2 'v (f32) (&'v mut [f32; 2]) (<[f32]>::as_mut_ptr)}
decl_builder_drag! { DragFloat3 drag_float_3_config ImGui_DragFloat3 'v (f32) (&'v mut [f32; 3]) (<[f32]>::as_mut_ptr)}
decl_builder_drag! { DragFloat4 drag_float_4_config ImGui_DragFloat4 'v (f32) (&'v mut [f32; 4]) (<[f32]>::as_mut_ptr)}

impl_float_format! { DragFloat }
impl_float_format! { DragFloat2 }
impl_float_format! { DragFloat3 }
impl_float_format! { DragFloat4 }

decl_builder_drag! { DragInt drag_int_config ImGui_DragInt 'v (i32) (&'v mut i32) (std::convert::identity)}
decl_builder_drag! { DragInt2 drag_int_2_config ImGui_DragInt2 'v (i32) (&'v mut [i32; 2]) (<[i32]>::as_mut_ptr)}
decl_builder_drag! { DragInt3 drag_int_3_config ImGui_DragInt3 'v (i32) (&'v mut [i32; 3]) (<[i32]>::as_mut_ptr)}
decl_builder_drag! { DragInt4 drag_int_4_config ImGui_DragInt4 'v (i32) (&'v mut [i32; 4]) (<[i32]>::as_mut_ptr)}

macro_rules! decl_builder_slider {
    ($name:ident $func:ident $cfunc:ident $life:lifetime ($argty:ty) ($ty:ty) ($expr:expr)) => {
        decl_builder! { $name -> bool, $cfunc ($life) (S: IntoCStr)
            (
                label (S::Temp) (label.as_ptr()),
                value ($ty) ($expr(value)),
                min ($argty) (min),
                max ($argty) (max),
                format (Cow<'static, CStr>) (format.as_ptr()),
                flags (SliderFlags) (flags.bits()),
            )
            {
                pub fn range(mut self, min: $argty, max: $argty) -> Self {
                    self.min = min;
                    self.max = max;
                    self
                }
                decl_builder_setter!{flags: SliderFlags}
            }
            {
                pub fn $func<$life, S: IntoCStr>(&self, label: LblId<S>, value: $ty) -> $name<'_, $life, S> {
                    $name {
                        _pd: PhantomData,
                        label: label.into(),
                        value,
                        min: <$argty>::default(),
                        max: <$argty>::default(),
                        format: Cow::Borrowed(c"%.3f"),
                        flags: SliderFlags::None,
                    }
                }
            }
        }
    };
}

decl_builder_slider! { SliderFloat slider_float_config ImGui_SliderFloat 'v (f32) (&'v mut f32) (std::convert::identity)}
decl_builder_slider! { SliderFloat2 slider_float_2_config ImGui_SliderFloat2 'v (f32) (&'v mut [f32; 2]) (<[f32]>::as_mut_ptr)}
decl_builder_slider! { SliderFloat3 slider_float_3_config ImGui_SliderFloat3 'v (f32) (&'v mut [f32; 3]) (<[f32]>::as_mut_ptr)}
decl_builder_slider! { SliderFloat4 slider_float_4_config ImGui_SliderFloat4 'v (f32) (&'v mut [f32; 4]) (<[f32]>::as_mut_ptr)}

impl_float_format! { SliderFloat }
impl_float_format! { SliderFloat2 }
impl_float_format! { SliderFloat3 }
impl_float_format! { SliderFloat4 }

decl_builder_slider! { SliderInt slider_int_config ImGui_SliderInt 'v (i32) (&'v mut i32) (std::convert::identity)}
decl_builder_slider! { SliderInt2 slider_int_2_config ImGui_SliderInt2 'v (i32) (&'v mut [i32; 2]) (<[i32]>::as_mut_ptr)}
decl_builder_slider! { SliderInt3 slider_int_3_config ImGui_SliderInt3 'v (i32) (&'v mut [i32; 3]) (<[i32]>::as_mut_ptr)}
decl_builder_slider! { SliderInt4 slider_int_4_config ImGui_SliderInt4 'v (i32) (&'v mut [i32; 4]) (<[i32]>::as_mut_ptr)}

decl_builder! { SliderAngle -> bool, ImGui_SliderAngle ('v) (S: IntoCStr)
    (
        label (S::Temp) (label.as_ptr()),
        v_rad (&'v mut f32) (v_rad),
        v_degrees_min (f32) (v_degrees_min),
        v_degrees_max (f32) (v_degrees_max),
        format (Cow<'static, CStr>) (format.as_ptr()),
        flags (SliderFlags) (flags.bits()),
    )
    {
        decl_builder_setter!{v_degrees_max: f32}
        decl_builder_setter!{v_degrees_min: f32}
        decl_builder_setter!{flags: SliderFlags}
    }
    {
        pub fn slider_angle_config<'v, S: IntoCStr>(&self, label: LblId<S>, v_rad: &'v mut f32) -> SliderAngle<'_, 'v, S> {
            SliderAngle {
                _pd: PhantomData,
                label: label.into(),
                v_rad,
                v_degrees_min: -360.0,
                v_degrees_max: 360.0,
                format: Cow::Borrowed(c"%.0f deg"),
                flags: SliderFlags::None,
            }
        }
    }
}

impl_float_format! { SliderAngle c"%g deg" c"%.0f deg" c"%.3f deg" "%.{}f deg"}

decl_builder! { ColorEdit3 -> bool, ImGui_ColorEdit3 ('v) (S: IntoCStr)
    (
        label (S::Temp) (label.as_ptr()),
        color (&'v mut [f32; 3]) (color.as_mut_ptr()),
        flags (ColorEditFlags) (flags.bits()),
    )
    {
        decl_builder_setter!{flags: ColorEditFlags}
    }
    {
        pub fn color_edit_3_config<'v, S: IntoCStr>(&self, label: LblId<S>, color: &'v mut [f32; 3]) -> ColorEdit3<'_, 'v, S> {
            ColorEdit3 {
                _pd: PhantomData,
                label: label.into(),
                color,
                flags: ColorEditFlags::None,
            }
        }
    }
}

decl_builder! { ColorEdit4 -> bool, ImGui_ColorEdit4 ('v) (S: IntoCStr)
    (
        label (S::Temp) (label.as_ptr()),
        color (&'v mut [f32; 4]) (color.as_mut_ptr()),
        flags (ColorEditFlags) (flags.bits()),
    )
    {
        decl_builder_setter!{flags: ColorEditFlags}
    }
    {
        pub fn color_edit_4_config<'v, S: IntoCStr>(&self, label: LblId<S>, color: &'v mut Color) -> ColorEdit4<'_, 'v, S> {
            ColorEdit4 {
                _pd: PhantomData,
                label: label.into(),
                color: color.as_mut(),
                flags: ColorEditFlags::None,
            }
        }
    }
}

decl_builder! { ColorPicker3 -> bool, ImGui_ColorPicker3 ('v) (S: IntoCStr)
    (
        label (S::Temp) (label.as_ptr()),
        color (&'v mut [f32; 3]) (color.as_mut_ptr()),
        flags (ColorEditFlags) (flags.bits()),
    )
    {
        decl_builder_setter!{flags: ColorEditFlags}
    }
    {
        pub fn color_picker_3_config<'v, S: IntoCStr>(&self, label: LblId<S>, color: &'v mut [f32; 3]) -> ColorPicker3<'_, 'v, S> {
            ColorPicker3 {
                _pd: PhantomData,
                label: label.into(),
                color,
                flags: ColorEditFlags::None,
            }
        }
    }
}

decl_builder! { ColorPicker4 -> bool, ImGui_ColorPicker4 ('v) (S: IntoCStr)
    (
        label (S::Temp) (label.as_ptr()),
        color (&'v mut [f32; 4]) (color.as_mut_ptr()),
        flags (ColorEditFlags) (flags.bits()),
        ref_col (Option<Color>) (ref_col.as_ref().map(|x| x.as_ref().as_ptr()).unwrap_or(null())),
    )
    {
        decl_builder_setter!{flags: ColorEditFlags}
        pub fn ref_color(mut self, ref_color: Color) -> Self {
            self.ref_col = Some(ref_color);
            self
        }
    }
    {
        pub fn color_picker_4_config<'v, S: IntoCStr>(&self, label: LblId<S>, color: &'v mut Color) -> ColorPicker4<'_, 'v, S> {
            ColorPicker4 {
                _pd: PhantomData,
                label: label.into(),
                color: color.as_mut(),
                flags: ColorEditFlags::None,
                ref_col: None,
            }
        }
    }
}

unsafe extern "C" fn input_text_callback(data: *mut ImGuiInputTextCallbackData) -> i32 {
    unsafe {
        let data = &mut *data;
        if data.EventFlag == InputTextFlags::CallbackResize.bits() {
            let this = &mut *(data.UserData as *mut String);
            let extra = (data.BufSize as usize).saturating_sub(this.len());
            this.reserve(extra);
            data.Buf = this.as_mut_ptr() as *mut c_char;
        }
        0
    }
}

#[inline]
fn text_pre_edit(text: &mut String) {
    // Ensure a NUL at the end
    text.push('\0');
}

#[inline]
unsafe fn text_post_edit(text: &mut String) {
    unsafe {
        let buf = text.as_mut_vec();
        // Look for the ending NUL that must be there, instead of memchr or iter::position, leverage the standard CStr
        let len = CStr::from_ptr(buf.as_ptr() as *const c_char)
            .to_bytes()
            .len();
        buf.set_len(len);
    }
}

unsafe fn input_text_wrapper(
    label: *const c_char,
    text: &mut String,
    flags: InputTextFlags,
) -> bool {
    unsafe {
        let flags = flags | InputTextFlags::CallbackResize;

        text_pre_edit(text);
        let r = ImGui_InputText(
            label,
            text.as_mut_ptr() as *mut c_char,
            text.capacity(),
            flags.bits(),
            Some(input_text_callback),
            text as *mut String as *mut c_void,
        );
        text_post_edit(text);
        r
    }
}

decl_builder! { InputText -> bool, input_text_wrapper ('v) (S: IntoCStr)
    (
        label (S::Temp) (label.as_ptr()),
        text (&'v mut String) (text),
        flags (InputTextFlags) (flags),
    )
    {
        decl_builder_setter!{flags: InputTextFlags}
    }
    {
        pub fn input_text_config<'v, S: IntoCStr>(&self, label: LblId<S>, text: &'v mut String) -> InputText<'_, 'v, S> {
            InputText {
                _pd: PhantomData,
                label: label.into(),
                text,
                flags: InputTextFlags::None,
            }
        }
    }
}

unsafe fn input_os_string_wrapper(
    label: *const c_char,
    os_string: &mut OsString,
    flags: InputTextFlags,
) -> bool {
    unsafe {
        let s = std::mem::take(os_string).into_string();
        let mut s = match s {
            Ok(s) => s,
            Err(os) => os.to_string_lossy().into_owned(),
        };
        let res = input_text_wrapper(label, &mut s, flags);
        *os_string = OsString::from(s);
        res
    }
}

decl_builder! { InputOsString -> bool, input_os_string_wrapper ('v) (S: IntoCStr)
    (
        label (S::Temp) (label.as_ptr()),
        text (&'v mut OsString) (text),
        flags (InputTextFlags) (flags),
    )
    {
        decl_builder_setter!{flags: InputTextFlags}
    }
    {
        pub fn input_os_string_config<'v, S: IntoCStr>(&self, label: LblId<S>, text: &'v mut OsString) -> InputOsString<'_, 'v, S> {
            InputOsString {
                _pd: PhantomData,
                label: label.into(),
                text,
                flags: InputTextFlags::None,
            }
        }
    }
}

unsafe fn input_text_multiline_wrapper(
    label: *const c_char,
    text: &mut String,
    size: &ImVec2,
    flags: InputTextFlags,
) -> bool {
    unsafe {
        let flags = flags | InputTextFlags::CallbackResize;
        text_pre_edit(text);
        let r = ImGui_InputTextMultiline(
            label,
            text.as_mut_ptr() as *mut c_char,
            text.capacity(),
            size,
            flags.bits(),
            Some(input_text_callback),
            text as *mut String as *mut c_void,
        );
        text_post_edit(text);
        r
    }
}

decl_builder! { InputTextMultiline -> bool, input_text_multiline_wrapper ('v) (S: IntoCStr)
    (
        label (S::Temp) (label.as_ptr()),
        text (&'v mut String) (text),
        size (ImVec2) (&size),
        flags (InputTextFlags) (flags),
    )
    {
        decl_builder_setter!{flags: InputTextFlags}
        decl_builder_setter_vector2!{size: Vector2}
    }
    {
        pub fn input_text_multiline_config<'v, S: IntoCStr>(&self, label: LblId<S>, text: &'v mut String) -> InputTextMultiline<'_, 'v, S> {
            InputTextMultiline {
                _pd: PhantomData,
                label:label.into(),
                text,
                flags: InputTextFlags::None,
                size: im_vec2(0.0, 0.0),
            }
        }
    }
}

unsafe fn input_text_hint_wrapper(
    label: *const c_char,
    hint: *const c_char,
    text: &mut String,
    flags: InputTextFlags,
) -> bool {
    unsafe {
        let flags = flags | InputTextFlags::CallbackResize;
        text_pre_edit(text);
        let r = ImGui_InputTextWithHint(
            label,
            hint,
            text.as_mut_ptr() as *mut c_char,
            text.capacity(),
            flags.bits(),
            Some(input_text_callback),
            text as *mut String as *mut c_void,
        );
        text_post_edit(text);
        r
    }
}

decl_builder! { InputTextHint -> bool, input_text_hint_wrapper ('v) (S1: IntoCStr, S2: IntoCStr)
    (
        label (S1::Temp) (label.as_ptr()),
        hint (S2::Temp) (hint.as_ptr()),
        text (&'v mut String) (text),
        flags (InputTextFlags) (flags),
    )
    {
        decl_builder_setter!{flags: InputTextFlags}
    }
    {
        pub fn input_text_hint_config<'v, S1: IntoCStr, S2: IntoCStr>(&self, label: LblId<S1>, hint: S2, text: &'v mut String) -> InputTextHint<'_, 'v, S1, S2> {
            InputTextHint {
                _pd: PhantomData,
                label:label.into(),
                hint: hint.into(),
                text,
                flags: InputTextFlags::None,
            }
        }
    }
}

/// How to convert a float value to a string.
///
/// It maps to the inner ImGui `sprintf` format parameter.
pub enum FloatFormat {
    /// `F(x)` is like `sprintf("%xf")`
    F(u32),
    /// `G` is like `sprintf("%g")`
    G,
}

decl_builder! { InputFloat -> bool, ImGui_InputFloat ('v) (S: IntoCStr)
    (
        label (S::Temp)  (label.as_ptr()),
        value (&'v mut f32) (value),
        step (f32) (step),
        step_fast (f32) (step_fast),
        format (Cow<'static, CStr>) (format.as_ptr()),
        flags (InputTextFlags) (flags.bits()),
    )
    {
        decl_builder_setter!{flags: InputTextFlags}
        decl_builder_setter!{step: f32}
        decl_builder_setter!{step_fast: f32}
    }
    {
        pub fn input_float_config<'v, S: IntoCStr>(&self, label: LblId<S>, value: &'v mut f32) -> InputFloat<'_, 'v, S> {
            InputFloat {
                _pd: PhantomData,
                label:label.into(),
                value,
                step: 0.0,
                step_fast: 0.0,
                format: Cow::Borrowed(c"%.3f"),
                flags: InputTextFlags::None,
            }
        }
    }
}

decl_builder! { InputInt -> bool, ImGui_InputInt ('v) (S: IntoCStr)
    (
        label (S::Temp) (label.as_ptr()),
        value (&'v mut i32) (value),
        step (i32) (step),
        step_fast (i32) (step_fast),
        flags (InputTextFlags) (flags.bits()),
    )
    {
        decl_builder_setter!{flags: InputTextFlags}
        decl_builder_setter!{step: i32}
        decl_builder_setter!{step_fast: i32}
    }
    {
        pub fn input_int_config<'v, S: IntoCStr>(&self, label: LblId<S>, value: &'v mut i32) -> InputInt<'_, 'v, S> {
            InputInt {
                _pd: PhantomData,
                label:label.into(),
                value,
                step: 1,
                step_fast: 100,
                flags: InputTextFlags::None,
            }
        }
    }
}

macro_rules! decl_builder_input_f {
    ($name:ident $func:ident $cfunc:ident $len:literal) => {
        decl_builder! { $name -> bool, $cfunc ('v) (S: IntoCStr)
        (
            label (S::Temp) (label.as_ptr()),
            value (&'v mut [f32; $len]) (value.as_mut_ptr()),
            format (Cow<'static, CStr>) (format.as_ptr()),
            flags (InputTextFlags) (flags.bits()),
        )
        {
            decl_builder_setter!{flags: InputTextFlags}
        }
        {
            pub fn $func<'v, S: IntoCStr>(&self, label: LblId<S>, value: &'v mut [f32; $len]) -> $name<'_, 'v, S> {
                $name {
                    _pd: PhantomData,
                    label: label.into(),
                    value,
                    format: Cow::Borrowed(c"%.3f"),
                    flags: InputTextFlags::None,
                }
            }
        }
    }

    };
}

decl_builder_input_f! { InputFloat2 input_float_2_config ImGui_InputFloat2 2}
decl_builder_input_f! { InputFloat3 input_float_3_config ImGui_InputFloat3 3}
decl_builder_input_f! { InputFloat4 input_float_4_config ImGui_InputFloat4 4}

impl_float_format! { InputFloat }
impl_float_format! { InputFloat2 }
impl_float_format! { InputFloat3 }
impl_float_format! { InputFloat4 }

macro_rules! decl_builder_input_i {
    ($name:ident $func:ident $cfunc:ident $len:literal) => {
        decl_builder! { $name -> bool, $cfunc ('v) (S: IntoCStr)
        (
            label (S::Temp) (label.as_ptr()),
            value (&'v mut [i32; $len]) (value.as_mut_ptr()),
            flags (InputTextFlags) (flags.bits()),
        )
        {
            decl_builder_setter!{flags: InputTextFlags}
        }
        {
            pub fn $func<'v, S: IntoCStr>(&self, label: LblId<S>, value: &'v mut [i32; $len]) -> $name<'_, 'v, S> {
                $name {
                    _pd: PhantomData,
                    label: label.into(),
                    value,
                    flags: InputTextFlags::None,
                }
            }
        }
    }

    };
}

decl_builder_input_i! { InputInt2 input_int_2_config ImGui_InputInt2 2}
decl_builder_input_i! { InputInt3 input_int_3_config ImGui_InputInt3 3}
decl_builder_input_i! { InputInt4 input_int_4_config ImGui_InputInt4 4}

decl_builder_with_opt! {Menu, ImGui_BeginMenu, ImGui_EndMenu () (S: IntoCStr)
    (
        name (S::Temp) (name.as_ptr()),
        enabled (bool) (enabled),
    )
    {
        decl_builder_setter!{enabled: bool}
    }
    {
        pub fn menu_config<S: IntoCStr>(&self, name: LblId<S>) -> Menu<S> {
            Menu {
                name: name.into(),
                enabled: true,
                push: (),
            }
        }
    }
}

decl_builder_with_opt! {CollapsingHeader, ImGui_CollapsingHeader, no_op () (S: IntoCStr)
    (
        label (S::Temp) (label.as_ptr()),
        flags (TreeNodeFlags) (flags.bits()),
    )
    {
        decl_builder_setter!{flags: TreeNodeFlags}
    }
    {
        pub fn collapsing_header_config<S: IntoCStr>(&self, label: LblId<S>) -> CollapsingHeader<S> {
            CollapsingHeader {
                label: label.into(),
                flags: TreeNodeFlags::None,
                push: (),
            }
        }
    }
}

enum LabelId<'a, S: IntoCStr, H: Hashable> {
    LblId(LblId<S>),
    LabelId(&'a str, H),
}

unsafe fn tree_node_ex_helper<S: IntoCStr, H: Hashable>(
    label_id: LabelId<'_, S, H>,
    flags: TreeNodeFlags,
) -> bool {
    unsafe {
        match label_id {
            LabelId::LblId(lbl) => ImGui_TreeNodeEx(lbl.into().as_ptr(), flags.bits()),
            LabelId::LabelId(lbl, id) => {
                let (start, end) = text_ptrs(lbl);
                // Warning! internal imgui API ahead, the alterative would be to call all the TreeNodeEx* functions without the Hashable generics
                ImGui_TreeNodeBehavior(id.get_id(), flags.bits(), start, end)
            }
        }
    }
}

decl_builder_with_opt! {TreeNode, tree_node_ex_helper, ImGui_TreePop ('a) (S: IntoCStr, H: Hashable)
    (
        label (LabelId<'a, S, H>) (label),
        flags (TreeNodeFlags) (flags),
    )
    {
        decl_builder_setter!{flags: TreeNodeFlags}
    }
    {
        pub fn tree_node_config<S: IntoCStr>(&self, label: LblId<S>) -> TreeNode<'static, S, usize> {
            TreeNode {
                label: LabelId::LblId(label),
                flags: TreeNodeFlags::None,
                push: (),
            }
        }
        pub fn tree_node_ex_config<'a, H: Hashable>(&self, id: H, label: &'a str) -> TreeNode<'a, &'a str, H> {
            TreeNode {
                label: LabelId::LabelId(label, id),
                flags: TreeNodeFlags::None,
                push: (),
            }
        }
    }
}

decl_builder_with_opt! {Popup, ImGui_BeginPopup, ImGui_EndPopup () (S: IntoCStr)
    (
        str_id (S::Temp) (str_id.as_ptr()),
        flags (WindowFlags) (flags.bits()),
    )
    {
        decl_builder_setter!{flags: WindowFlags}
    }
    {
        pub fn popup_config<S: IntoCStr>(&self, str_id: Id<S>) -> Popup<S> {
            Popup {
                str_id: str_id.into(),
                flags: WindowFlags::None,
                push: (),
            }
        }
    }
}

enum PopupOpened<'a> {
    Literal(bool),
    Reference(&'a mut bool),
    None,
}

impl PopupOpened<'_> {
    unsafe fn pointer(&mut self) -> *mut bool {
        match self {
            PopupOpened::Literal(x) => x,
            PopupOpened::Reference(r) => *r,
            PopupOpened::None => std::ptr::null_mut(),
        }
    }
}

decl_builder_with_opt! {PopupModal, ImGui_BeginPopupModal, ImGui_EndPopup ('a) (S: IntoCStr)
    (
        name (S::Temp) (name.as_ptr()),
        opened (PopupOpened<'a>) (opened.pointer()),
        flags (WindowFlags) (flags.bits()),
    )
    {
        decl_builder_setter!{flags: WindowFlags}

        pub fn close_button(mut self, close_button: bool) -> Self {
            self.opened = if close_button { PopupOpened::Literal(true) } else { PopupOpened::None };
            self
        }

        pub fn opened(self, opened: Option<&'a mut bool>) -> PopupModal<'a, S, P> {
            let opened = match opened {
                Some(b) => PopupOpened::Reference(b),
                None => PopupOpened::None,
            };
            PopupModal {
                opened,
                .. self
            }
        }
    }
    {
        pub fn popup_modal_config<S: IntoCStr>(&self, name: LblId<S>) -> PopupModal<'static, S> {
            PopupModal {
                name: name.into(),
                opened: PopupOpened::None,
                flags: WindowFlags::None,
                push: (),
            }
        }
    }
}

macro_rules! decl_builder_popup_context {
    ($struct:ident $begin:ident $do_function:ident) => {
        decl_builder_with_opt! {$struct, $begin, ImGui_EndPopup () (S: IntoCStr)
            (
                str_id (Option<S::Temp>) (optional_str(&str_id)),
                flags (PopupFlags) (flags.bits()),
            )
            {
                decl_builder_setter!{flags: PopupFlags}
                pub fn str_id<S2: IntoCStr>(self, str_id: LblId<S2>) -> $struct<S2, P> {
                    $struct {
                        str_id: Some(str_id.into()),
                        flags: self.flags,
                        push: self.push,
                    }
                }

            }
            {
                pub fn $do_function<'a>(&self) -> $struct<&'a str> {
                    $struct {
                        str_id: None,
                        flags: PopupFlags::MouseButtonRight,
                        push: (),
                    }
                }
            }
        }
    };
}

decl_builder_popup_context! {PopupContextItem ImGui_BeginPopupContextItem popup_context_item_config}
decl_builder_popup_context! {PopupContextWindow ImGui_BeginPopupContextWindow popup_context_window_config}
decl_builder_popup_context! {PopupContextVoid ImGui_BeginPopupContextVoid popup_context_void_config}

decl_builder_with_opt! {Combo, ImGui_BeginCombo, ImGui_EndCombo () (S1: IntoCStr, S2: IntoCStr)
    (
        label (S1::Temp) (label.as_ptr()),
        preview_value (Option<S2::Temp>) (optional_str(&preview_value)),
        flags (ComboFlags) (flags.bits()),
    )
    {
        decl_builder_setter!{flags: ComboFlags}
        pub fn preview_value_opt<S3: IntoCStr>(self, preview_value: Option<S3>) -> Combo<S1, S3> {
            Combo {
                label: self.label,
                preview_value: preview_value.map(|x| x.into()),
                flags: ComboFlags::None,
                push: (),
            }
        }
        pub fn preview_value<S3: IntoCStr>(self, preview_value: S3) -> Combo<S1, S3> {
            self.preview_value_opt(Some(preview_value))
        }
    }
    {
        pub fn combo_config<'a, S: IntoCStr>(&self, label: LblId<S>) -> Combo<S, &'a str> {
            Combo {
                label: label.into(),
                preview_value: None,
                flags: ComboFlags::None,
                push: (),
            }
        }
        // Helper function for simple use cases
        pub fn combo<V: Copy + PartialEq, S1: IntoCStr, S2: IntoCStr>(
            &self,
            label: LblId<S1>,
            values: impl IntoIterator<Item=V>,
            f_name: impl Fn(V) -> S2,
            current: &mut V
        ) -> bool
        {
            let mut changed = false;
            self.combo_config(label)
                .preview_value(f_name(*current))
                .with(|| {
                    for (i, val) in values.into_iter().enumerate() {
                        if self.selectable_config(lbl_id(f_name(val), i.to_string()))
                            .selected(*current == val)
                            .build()
                        {
                            *current = val;
                            changed = true;
                        }
                    }
                });
            changed
        }
    }
}

decl_builder_with_opt! {ListBox, ImGui_BeginListBox, ImGui_EndListBox () (S: IntoCStr)
    (
        label (S::Temp) (label.as_ptr()),
        size (ImVec2) (&size),
    )
    {
        decl_builder_setter_vector2!{size: Vector2}
    }
    {
        pub fn list_box_config<S: IntoCStr>(&self, label: LblId<S>) -> ListBox<S> {
            ListBox {
                label: label.into(),
                size: im_vec2(0.0, 0.0),
                push: (),
            }
        }
        // Helper function for simple use cases
        pub fn list_box<V: Copy + PartialEq, S1: IntoCStr, S2: IntoCStr>(
            &self,
            label: LblId<S1>,
            mut height_in_items: i32,
            values: impl IntoIterator<Item=V>,
            f_name: impl Fn(V) -> S2,
            current: &mut V
        ) -> bool
        {
            // Calculate size from "height_in_items"
            if height_in_items < 0 {
                // this should be values.len().min(7) but IntoIterator is lazy evaluated
                height_in_items = 7;
            }
            let height_in_items_f = height_in_items as f32 + 0.25;
            let height_in_pixels = self.get_text_line_height_with_spacing() * height_in_items_f + self.style().FramePadding.y * 2.0;

            let mut changed = false;
            self.list_box_config(label)
                .size(vec2(0.0, height_in_pixels.floor()))
                .with(|| {
                    for (i, val) in values.into_iter().enumerate() {
                        if self.selectable_config(lbl_id(f_name(val), i.to_string()))
                            .selected(*current == val)
                            .build()
                        {
                            *current = val;
                            changed = true;
                        }
                    }
                });
            changed
        }
    }
}

decl_builder_with_opt! {TabBar, ImGui_BeginTabBar, ImGui_EndTabBar () (S: IntoCStr)
    (
        str_id (S::Temp) (str_id.as_ptr()),
        flags (TabBarFlags) (flags.bits()),
    )
    {
        decl_builder_setter!{flags: TabBarFlags}
    }
    {
        pub fn tab_bar_config<S: IntoCStr>(&self, str_id: LblId<S>) -> TabBar<S> {
            TabBar {
                str_id: str_id.into(),
                flags: TabBarFlags::None,
                push: (),
            }
        }
    }
}

decl_builder_with_opt! {TabItem, ImGui_BeginTabItem, ImGui_EndTabItem ('o) (S: IntoCStr)
    (
        str_id (S::Temp) (str_id.as_ptr()),
        opened (Option<&'o mut bool>) (optional_mut_bool(&mut opened)),
        flags (TabItemFlags) (flags.bits()),
    )
    {
        decl_builder_setter!{flags: TabItemFlags}
        decl_builder_setter!{opened: &'o mut bool}
    }
    {
        pub fn tab_item_config<S: IntoCStr>(&self, str_id: LblId<S>) -> TabItem<'_, S> {
            TabItem {
                str_id: str_id.into(),
                opened: None,
                flags: TabItemFlags::None,
                push: (),
            }
        }
        pub fn tab_item_button(label: LblId<impl IntoCStr>, flags: TabItemFlags) -> bool {
            unsafe {
                ImGui_TabItemButton(label.into().as_ptr(), flags.bits())
            }
        }
        pub fn set_tab_item_closed(tab_or_docked_window_label: LblId<impl IntoCStr>) {
            unsafe {
                ImGui_SetTabItemClosed(tab_or_docked_window_label.into().as_ptr());
            }
        }
    }
}

impl<A> Ui<A> {
    // The callback will be callable until the next call to do_frame()
    unsafe fn push_callback<X>(&self, mut cb: impl FnMut(*mut A, X) + 'static) -> usize {
        let cb = Box::new(move |data: *mut A, ptr: *mut c_void| {
            let x = ptr as *mut X;
            cb(data, unsafe { std::ptr::read(x) });
        });
        let mut callbacks = self.callbacks.borrow_mut();
        let id = callbacks.len();

        callbacks.push(cb);
        merge_generation(id, self.generation)
    }
    unsafe fn run_callback<X>(id: usize, x: X) {
        unsafe {
            let user_data = RawContext::current().io().BackendLanguageUserData;
            if user_data.is_null() {
                return;
            }
            // The lifetime of ui has been erased, but at least the types of A and X should be correct
            let ui = &*(user_data as *const Self);
            let Some(id) = remove_generation(id, ui.generation) else {
                eprintln!("lost generation callback");
                return;
            };

            let mut callbacks = ui.callbacks.borrow_mut();
            let cb = &mut callbacks[id];
            // disable the destructor of x, it will be run inside the callback
            let mut x = MaybeUninit::new(x);
            cb(ui.data, x.as_mut_ptr() as *mut c_void);
        }
    }

    pub fn get_clipboard_text(&self) -> String {
        unsafe {
            CStr::from_ptr(ImGui_GetClipboardText())
                .to_string_lossy()
                .into_owned()
        }
    }
    pub fn set_clipboard_text(&self, text: impl IntoCStr) {
        let text = text.into();
        unsafe { ImGui_SetClipboardText(text.as_ptr()) }
    }
    pub fn set_next_window_size_constraints_callback(
        &self,
        size_min: Vector2,
        size_max: Vector2,
        mut cb: impl FnMut(SizeCallbackData<'_>) + 'static,
    ) {
        unsafe {
            // Beware! This callback is called while the `do_ui()` is still running, so the argument for the
            // first callback is null!
            let id = self.push_callback(move |_, scd| cb(scd));
            ImGui_SetNextWindowSizeConstraints(
                &v2_to_im(size_min),
                &v2_to_im(size_max),
                Some(call_size_callback::<A>),
                id as *mut c_void,
            );
        }
    }
    pub fn set_next_window_size_constraints(&self, size_min: Vector2, size_max: Vector2) {
        unsafe {
            ImGui_SetNextWindowSizeConstraints(
                &v2_to_im(size_min),
                &v2_to_im(size_max),
                None,
                null_mut(),
            );
        }
    }
    pub fn set_next_item_width(&self, item_width: f32) {
        unsafe {
            ImGui_SetNextItemWidth(item_width);
        }
    }
    pub fn set_next_item_open(&self, is_open: bool, cond: Cond) {
        unsafe {
            ImGui_SetNextItemOpen(is_open, cond.bits());
        }
    }
    pub fn set_keyboard_focus_here(&self, offset: i32) {
        unsafe { ImGui_SetKeyboardFocusHere(offset) }
    }

    with_begin_end! {
        /// See `BeginGroup`, `EndGroup`.
        group ImGui_BeginGroup ImGui_EndGroup ()
    }
    with_begin_end! {
        /// See `BeginDisabled`, `EndDisabled`.
        disabled ImGui_BeginDisabled ImGui_EndDisabled (
            disabled (bool) (disabled),
        )
    }
    with_begin_end! {
        /// See `PushClipRect`, `PopClipRect`.
        clip_rect ImGui_PushClipRect ImGui_PopClipRect (
            clip_rect_min (Vector2) (&v2_to_im(clip_rect_min)),
            clip_rect_max (Vector2) (&v2_to_im(clip_rect_max)),
            intersect_with_current_clip_rect (bool) (intersect_with_current_clip_rect),
        )
    }

    with_begin_end_opt! {
        /// See `BeginMainMenuBar`, `EndMainMenuBar`.
        main_menu_bar ImGui_BeginMainMenuBar ImGui_EndMainMenuBar ()
    }
    with_begin_end_opt! {
        /// See `BeginMenuBar`, `EndMenuBar`.
        menu_bar ImGui_BeginMenuBar ImGui_EndMenuBar ()
    }
    with_begin_end_opt! {
        /// See `BeginTooltip`, `EndTooltip`.
        tooltip ImGui_BeginTooltip ImGui_EndTooltip ()
    }
    with_begin_end_opt! {
        /// See `BeginItemTooltip`, `EndTooltip`. There is not `EndItemTooltip`.
        item_tooltip ImGui_BeginItemTooltip ImGui_EndTooltip ()
    }

    /// Calls the `f` functions with the given `push`
    pub fn with_push<R>(&self, push: impl Pushable, f: impl FnOnce() -> R) -> R {
        unsafe {
            let _guard = push_guard(&push);
            f()
        }
    }
    pub fn show_demo_window(&self, mut show: Option<&mut bool>) {
        unsafe {
            ImGui_ShowDemoWindow(optional_mut_bool(&mut show));
        }
    }
    pub fn set_next_window_pos(&self, pos: Vector2, cond: Cond, pivot: Vector2) {
        unsafe {
            ImGui_SetNextWindowPos(&v2_to_im(pos), cond.bits(), &v2_to_im(pivot));
        }
    }
    pub fn set_next_window_size(&self, size: Vector2, cond: Cond) {
        unsafe {
            ImGui_SetNextWindowSize(&v2_to_im(size), cond.bits());
        }
    }
    pub fn set_next_window_content_size(&self, size: Vector2) {
        unsafe {
            ImGui_SetNextWindowContentSize(&v2_to_im(size));
        }
    }

    pub fn set_next_window_collapsed(&self, collapsed: bool, cond: Cond) {
        unsafe {
            ImGui_SetNextWindowCollapsed(collapsed, cond.bits());
        }
    }

    pub fn set_next_window_focus(&self) {
        unsafe {
            ImGui_SetNextWindowFocus();
        }
    }

    pub fn set_next_window_scroll(&self, scroll: Vector2) {
        unsafe {
            ImGui_SetNextWindowScroll(&v2_to_im(scroll));
        }
    }

    pub fn set_next_window_bg_alpha(&self, alpha: f32) {
        unsafe {
            ImGui_SetNextWindowBgAlpha(alpha);
        }
    }
    pub fn window_draw_list(&self) -> WindowDrawList<'_, A> {
        unsafe {
            let ptr = ImGui_GetWindowDrawList();
            WindowDrawList { ui: self, ptr }
        }
    }
    pub fn window_dpi_scale(&self) -> f32 {
        unsafe { ImGui_GetWindowDpiScale() }
    }
    pub fn foreground_draw_list(&self) -> WindowDrawList<'_, A> {
        unsafe {
            let ptr = ImGui_GetForegroundDrawList(std::ptr::null_mut());
            WindowDrawList { ui: self, ptr }
        }
    }
    pub fn background_draw_list(&self) -> WindowDrawList<'_, A> {
        unsafe {
            let ptr = ImGui_GetBackgroundDrawList(std::ptr::null_mut());
            WindowDrawList { ui: self, ptr }
        }
    }
    pub fn text(&self, text: &str) {
        unsafe {
            let (start, end) = text_ptrs(text);
            ImGui_TextUnformatted(start, end);
        }
    }
    pub fn text_colored(&self, color: Color, text: impl IntoCStr) {
        let text = text.into();
        unsafe { ImGui_TextColored(&color.into(), c"%s".as_ptr(), text.as_ptr()) }
    }
    pub fn text_disabled(&self, text: impl IntoCStr) {
        let text = text.into();
        unsafe { ImGui_TextDisabled(c"%s".as_ptr(), text.as_ptr()) }
    }
    pub fn text_wrapped(&self, text: impl IntoCStr) {
        let text = text.into();
        unsafe { ImGui_TextWrapped(c"%s".as_ptr(), text.as_ptr()) }
    }
    pub fn text_link(&self, label: LblId<impl IntoCStr>) -> bool {
        let label = label.into();
        unsafe { ImGui_TextLink(label.as_ptr()) }
    }
    pub fn text_link_open_url(&self, label: LblId<impl IntoCStr>, url: impl IntoCStr) -> bool {
        let label = label.into();
        let url = url.into();
        unsafe { ImGui_TextLinkOpenURL(label.as_ptr(), url.as_ptr()) }
    }
    pub fn label_text(&self, label: impl IntoCStr, text: impl IntoCStr) {
        let label = label.into();
        let text = text.into();
        unsafe { ImGui_LabelText(label.as_ptr(), c"%s".as_ptr(), text.as_ptr()) }
    }
    pub fn bullet_text(&self, text: impl IntoCStr) {
        let text = text.into();
        unsafe { ImGui_BulletText(c"%s".as_ptr(), text.as_ptr()) }
    }
    pub fn bullet(&self) {
        unsafe {
            ImGui_Bullet();
        }
    }
    pub fn separator_text(&self, text: impl IntoCStr) {
        let text = text.into();
        unsafe {
            ImGui_SeparatorText(text.as_ptr());
        }
    }
    pub fn separator(&self) {
        unsafe {
            ImGui_Separator();
        }
    }

    pub fn set_item_default_focus(&self) {
        unsafe {
            ImGui_SetItemDefaultFocus();
        }
    }
    pub fn is_item_hovered(&self) -> bool {
        self.is_item_hovered_ex(HoveredFlags::None)
    }
    pub fn is_item_hovered_ex(&self, flags: HoveredFlags) -> bool {
        unsafe { ImGui_IsItemHovered(flags.bits()) }
    }
    pub fn is_item_active(&self) -> bool {
        unsafe { ImGui_IsItemActive() }
    }
    pub fn is_item_focused(&self) -> bool {
        unsafe { ImGui_IsItemFocused() }
    }
    pub fn is_item_clicked(&self, flags: MouseButton) -> bool {
        unsafe { ImGui_IsItemClicked(flags.bits()) }
    }
    pub fn is_item_visible(&self) -> bool {
        unsafe { ImGui_IsItemVisible() }
    }
    pub fn is_item_edited(&self) -> bool {
        unsafe { ImGui_IsItemEdited() }
    }
    pub fn is_item_activated(&self) -> bool {
        unsafe { ImGui_IsItemActivated() }
    }
    pub fn is_item_deactivated(&self) -> bool {
        unsafe { ImGui_IsItemDeactivated() }
    }
    pub fn is_item_deactivated_after_edit(&self) -> bool {
        unsafe { ImGui_IsItemDeactivatedAfterEdit() }
    }
    pub fn is_item_toggled_open(&self) -> bool {
        unsafe { ImGui_IsItemToggledOpen() }
    }
    pub fn is_any_item_hovered(&self) -> bool {
        unsafe { ImGui_IsAnyItemHovered() }
    }
    pub fn is_any_item_active(&self) -> bool {
        unsafe { ImGui_IsAnyItemActive() }
    }
    pub fn is_any_item_focused(&self) -> bool {
        unsafe { ImGui_IsAnyItemFocused() }
    }
    pub fn is_window_collapsed(&self) -> bool {
        unsafe { ImGui_IsWindowCollapsed() }
    }
    pub fn is_window_focused(&self, flags: FocusedFlags) -> bool {
        unsafe { ImGui_IsWindowFocused(flags.bits()) }
    }
    pub fn is_window_hovered(&self, flags: FocusedFlags) -> bool {
        unsafe { ImGui_IsWindowHovered(flags.bits()) }
    }
    pub fn get_item_id(&self) -> ImGuiID {
        unsafe { ImGui_GetItemID() }
    }
    pub fn get_id(&self, id: impl Hashable) -> ImGuiID {
        unsafe { id.get_id() }
    }
    pub fn get_item_rect_min(&self) -> Vector2 {
        unsafe { im_to_v2(ImGui_GetItemRectMin()) }
    }
    pub fn get_item_rect_max(&self) -> Vector2 {
        unsafe { im_to_v2(ImGui_GetItemRectMax()) }
    }
    pub fn get_item_rect_size(&self) -> Vector2 {
        unsafe { im_to_v2(ImGui_GetItemRectSize()) }
    }
    /// Available space from current position. This is your best friend!
    pub fn get_content_region_avail(&self) -> Vector2 {
        unsafe { im_to_v2(ImGui_GetContentRegionAvail()) }
    }
    pub fn get_window_pos(&self) -> Vector2 {
        unsafe { im_to_v2(ImGui_GetWindowPos()) }
    }
    pub fn get_window_width(&self) -> f32 {
        unsafe { ImGui_GetWindowWidth() }
    }
    pub fn get_window_height(&self) -> f32 {
        unsafe { ImGui_GetWindowHeight() }
    }
    pub fn get_scroll_x(&self) -> f32 {
        unsafe { ImGui_GetScrollX() }
    }
    pub fn get_scroll_y(&self) -> f32 {
        unsafe { ImGui_GetScrollY() }
    }
    pub fn set_scroll_x(&self, scroll_x: f32) {
        unsafe {
            ImGui_SetScrollX(scroll_x);
        }
    }
    pub fn set_scroll_y(&self, scroll_y: f32) {
        unsafe {
            ImGui_SetScrollY(scroll_y);
        }
    }
    pub fn get_scroll_max_x(&self) -> f32 {
        unsafe { ImGui_GetScrollMaxX() }
    }
    pub fn get_scroll_max_y(&self) -> f32 {
        unsafe { ImGui_GetScrollMaxY() }
    }
    pub fn set_scroll_here_x(&self, center_x_ratio: f32) {
        unsafe {
            ImGui_SetScrollHereX(center_x_ratio);
        }
    }
    pub fn set_scroll_here_y(&self, center_y_ratio: f32) {
        unsafe {
            ImGui_SetScrollHereY(center_y_ratio);
        }
    }
    pub fn set_scroll_from_pos_x(&self, local_x: f32, center_x_ratio: f32) {
        unsafe {
            ImGui_SetScrollFromPosX(local_x, center_x_ratio);
        }
    }
    pub fn set_scroll_from_pos_y(&self, local_y: f32, center_y_ratio: f32) {
        unsafe {
            ImGui_SetScrollFromPosY(local_y, center_y_ratio);
        }
    }
    pub fn set_window_pos(&self, pos: Vector2, cond: Cond) {
        unsafe {
            ImGui_SetWindowPos(&v2_to_im(pos), cond.bits());
        }
    }
    pub fn set_window_size(&self, size: Vector2, cond: Cond) {
        unsafe {
            ImGui_SetWindowSize(&v2_to_im(size), cond.bits());
        }
    }
    pub fn set_window_collapsed(&self, collapsed: bool, cond: Cond) {
        unsafe {
            ImGui_SetWindowCollapsed(collapsed, cond.bits());
        }
    }
    pub fn set_window_focus(&self) {
        unsafe {
            ImGui_SetWindowFocus();
        }
    }
    pub fn same_line(&self) {
        unsafe {
            ImGui_SameLine(0.0, -1.0);
        }
    }
    pub fn same_line_ex(&self, offset_from_start_x: f32, spacing: f32) {
        unsafe {
            ImGui_SameLine(offset_from_start_x, spacing);
        }
    }
    pub fn new_line(&self) {
        unsafe {
            ImGui_NewLine();
        }
    }
    pub fn spacing(&self) {
        unsafe {
            ImGui_Spacing();
        }
    }
    pub fn dummy(&self, size: Vector2) {
        unsafe {
            ImGui_Dummy(&v2_to_im(size));
        }
    }
    pub fn indent(&self, indent_w: f32) {
        unsafe {
            ImGui_Indent(indent_w);
        }
    }
    pub fn unindent(&self, indent_w: f32) {
        unsafe {
            ImGui_Unindent(indent_w);
        }
    }
    /// Prefer `get_cursor_screen_pos` over this.
    pub fn get_cursor_pos(&self) -> Vector2 {
        unsafe { im_to_v2(ImGui_GetCursorPos()) }
    }
    /// Prefer `get_cursor_screen_pos` over this.
    pub fn get_cursor_pos_x(&self) -> f32 {
        unsafe { ImGui_GetCursorPosX() }
    }
    /// Prefer `get_cursor_screen_pos` over this.
    pub fn get_cursor_pos_y(&self) -> f32 {
        unsafe { ImGui_GetCursorPosY() }
    }
    /// Prefer `set_cursor_screen_pos` over this.
    pub fn set_cursor_pos(&self, local_pos: Vector2) {
        unsafe {
            ImGui_SetCursorPos(&v2_to_im(local_pos));
        }
    }
    /// Prefer `set_cursor_screen_pos` over this.
    pub fn set_cursor_pos_x(&self, local_x: f32) {
        unsafe {
            ImGui_SetCursorPosX(local_x);
        }
    }
    /// Prefer `set_cursor_screen_pos` over this.
    pub fn set_cursor_pos_y(&self, local_y: f32) {
        unsafe {
            ImGui_SetCursorPosY(local_y);
        }
    }
    /// Prefer `get_cursor_screen_pos` over this.
    pub fn get_cursor_start_pos(&self) -> Vector2 {
        unsafe { im_to_v2(ImGui_GetCursorStartPos()) }
    }
    /// Get cursor position in absolute coordinates. This is your best friend!
    pub fn get_cursor_screen_pos(&self) -> Vector2 {
        unsafe { im_to_v2(ImGui_GetCursorScreenPos()) }
    }
    /// Set cursor position in absolute coordinates. This is your best friend!
    pub fn set_cursor_screen_pos(&self, pos: Vector2) {
        unsafe {
            ImGui_SetCursorScreenPos(&v2_to_im(pos));
        }
    }
    pub fn align_text_to_frame_padding(&self) {
        unsafe {
            ImGui_AlignTextToFramePadding();
        }
    }
    pub fn get_text_line_height(&self) -> f32 {
        unsafe { ImGui_GetTextLineHeight() }
    }
    pub fn get_text_line_height_with_spacing(&self) -> f32 {
        unsafe { ImGui_GetTextLineHeightWithSpacing() }
    }
    pub fn get_frame_height(&self) -> f32 {
        unsafe { ImGui_GetFrameHeight() }
    }
    pub fn get_frame_height_with_spacing(&self) -> f32 {
        unsafe { ImGui_GetFrameHeightWithSpacing() }
    }
    pub fn calc_item_width(&self) -> f32 {
        unsafe { ImGui_CalcItemWidth() }
    }
    pub fn calc_text_size(&self, text: &str) -> Vector2 {
        self.calc_text_size_ex(text, false, -1.0)
    }
    pub fn calc_text_size_ex(
        &self,
        text: &str,
        hide_text_after_double_hash: bool,
        wrap_width: f32,
    ) -> Vector2 {
        unsafe {
            let (start, end) = text_ptrs(text);
            im_to_v2(ImGui_CalcTextSize(
                start,
                end,
                hide_text_after_double_hash,
                wrap_width,
            ))
        }
    }
    pub fn set_color_edit_options(&self, flags: ColorEditFlags) {
        unsafe {
            ImGui_SetColorEditOptions(flags.bits());
        }
    }
    pub fn key_mods(&self) -> KeyMod {
        let mods = self.io().KeyMods;
        KeyMod::from_bits_truncate(mods & ImGuiKey::ImGuiMod_Mask_.0)
    }
    pub fn is_key_down(&self, key: Key) -> bool {
        unsafe { ImGui_IsKeyDown(key.bits()) }
    }
    pub fn is_key_pressed(&self, key: Key) -> bool {
        unsafe {
            ImGui_IsKeyPressed(key.bits(), /*repeat*/ true)
        }
    }
    pub fn is_key_pressed_no_repeat(&self, key: Key) -> bool {
        unsafe {
            ImGui_IsKeyPressed(key.bits(), /*repeat*/ false)
        }
    }
    pub fn is_key_released(&self, key: Key) -> bool {
        unsafe { ImGui_IsKeyReleased(key.bits()) }
    }
    pub fn get_key_pressed_amount(&self, key: Key, repeat_delay: f32, rate: f32) -> i32 {
        unsafe { ImGui_GetKeyPressedAmount(key.bits(), repeat_delay, rate) }
    }
    pub fn get_font_tex_uv_white_pixel(&self) -> Vector2 {
        unsafe { im_to_v2(ImGui_GetFontTexUvWhitePixel()) }
    }
    //GetKeyName
    //SetNextFrameWantCaptureKeyboard
    pub fn get_font_size(&self) -> f32 {
        unsafe { ImGui_GetFontSize() }
    }
    pub fn is_mouse_down(&self, button: MouseButton) -> bool {
        unsafe { ImGui_IsMouseDown(button.bits()) }
    }
    pub fn is_mouse_clicked(&self, button: MouseButton) -> bool {
        unsafe {
            ImGui_IsMouseClicked(button.bits(), /*repeat*/ false)
        }
    }
    pub fn is_mouse_clicked_repeat(&self, button: MouseButton) -> bool {
        unsafe {
            ImGui_IsMouseClicked(button.bits(), /*repeat*/ true)
        }
    }
    pub fn is_mouse_released(&self, button: MouseButton) -> bool {
        unsafe { ImGui_IsMouseReleased(button.bits()) }
    }
    pub fn is_mouse_double_clicked(&self, button: MouseButton) -> bool {
        unsafe { ImGui_IsMouseDoubleClicked(button.bits()) }
    }
    pub fn get_mouse_clicked_count(&self, button: MouseButton) -> i32 {
        unsafe { ImGui_GetMouseClickedCount(button.bits()) }
    }
    pub fn is_rect_visible_size(&self, size: Vector2) -> bool {
        unsafe { ImGui_IsRectVisible(&v2_to_im(size)) }
    }
    pub fn is_rect_visible(&self, rect_min: Vector2, rect_max: Vector2) -> bool {
        unsafe { ImGui_IsRectVisible1(&v2_to_im(rect_min), &v2_to_im(rect_max)) }
    }
    /*
    pub fn is_mouse_hovering_rect(&self) -> bool {
        unsafe {
            ImGui_IsMouseHoveringRect(const ImVec2& r_min, const ImVec2& r_max, bool clip = true);
        }
    }
    pub fn is_mouse_pos_valid(&self) -> bool {
        unsafe {
            ImGui_IsMousePosValid(const ImVec2* mouse_pos = NULL);
        }
    }*/
    pub fn is_any_mouse_down(&self) -> bool {
        unsafe { ImGui_IsAnyMouseDown() }
    }
    pub fn get_mouse_pos(&self) -> Vector2 {
        unsafe { im_to_v2(ImGui_GetMousePos()) }
    }
    pub fn get_mouse_pos_on_opening_current_popup(&self) -> Vector2 {
        unsafe { im_to_v2(ImGui_GetMousePosOnOpeningCurrentPopup()) }
    }
    pub fn is_mouse_dragging(&self, button: MouseButton) -> bool {
        unsafe {
            ImGui_IsMouseDragging(button.bits(), /*lock_threshold*/ -1.0)
        }
    }
    pub fn get_mouse_drag_delta(&self, button: MouseButton) -> Vector2 {
        unsafe {
            im_to_v2(ImGui_GetMouseDragDelta(
                button.bits(),
                /*lock_threshold*/ -1.0,
            ))
        }
    }
    pub fn reset_mouse_drag_delta(&self, button: MouseButton) {
        unsafe {
            ImGui_ResetMouseDragDelta(button.bits());
        }
    }
    pub fn get_mouse_cursor(&self) -> MouseCursor {
        unsafe { MouseCursor::from_bits(ImGui_GetMouseCursor()).unwrap_or(MouseCursor::None) }
    }
    pub fn set_mouse_cursor(&self, cursor_type: MouseCursor) {
        unsafe {
            ImGui_SetMouseCursor(cursor_type.bits());
        }
    }
    pub fn get_time(&self) -> f64 {
        unsafe { ImGui_GetTime() }
    }
    pub fn get_frame_count(&self) -> i32 {
        unsafe { ImGui_GetFrameCount() }
    }
    pub fn is_popup_open(&self, str_id: Option<Id<impl IntoCStr>>) -> bool {
        self.is_popup_open_ex(str_id, PopupFlags::None)
    }
    pub fn is_popup_open_ex(&self, str_id: Option<Id<impl IntoCStr>>, flags: PopupFlags) -> bool {
        let temp;
        let str_id = match str_id {
            Some(s) => {
                temp = IntoCStr::into(s.0);
                temp.as_ptr()
            }
            None => null(),
        };
        unsafe { ImGui_IsPopupOpen(str_id, flags.bits()) }
    }
    /// Returns true if the current window is below a modal pop-up.
    pub fn is_below_blocking_modal(&self) -> bool {
        // Beware: internal API
        unsafe {
            let modal = ImGui_FindBlockingModal(self.CurrentWindow);
            !modal.is_null()
        }
    }
    /// Return true if there is any modal window opened
    pub fn is_blocking_modal(&self) -> bool {
        // Beware: internal API
        unsafe {
            let modal = ImGui_FindBlockingModal(std::ptr::null_mut());
            !modal.is_null()
        }
    }
    pub fn open_popup(&self, str_id: Id<impl IntoCStr>) {
        self.open_popup_ex(str_id, PopupFlags::None)
    }
    pub fn open_popup_ex(&self, str_id: Id<impl IntoCStr>, flags: PopupFlags) {
        let str_id = str_id.into();
        unsafe {
            ImGui_OpenPopup(str_id.as_ptr(), flags.bits());
        }
    }
    pub fn close_current_popup(&self) {
        unsafe {
            ImGui_CloseCurrentPopup();
        }
    }
    pub fn is_window_appearing(&self) -> bool {
        unsafe { ImGui_IsWindowAppearing() }
    }
    pub fn with_always_drag_drop_source<R>(
        &self,
        flags: DragDropSourceFlags,
        f: impl FnOnce(Option<DragDropPayloadSetter<'_>>) -> R,
    ) -> R {
        if !unsafe { ImGui_BeginDragDropSource(flags.bits()) } {
            return f(None);
        }
        let payload = DragDropPayloadSetter {
            _dummy: PhantomData,
        };
        let r = f(Some(payload));
        unsafe { ImGui_EndDragDropSource() }
        r
    }
    pub fn with_drag_drop_source<R>(
        &self,
        flags: DragDropSourceFlags,
        f: impl FnOnce(DragDropPayloadSetter<'_>) -> R,
    ) -> Option<R> {
        self.with_always_drag_drop_source(flags, move |r| r.map(f))
    }
    pub fn with_always_drag_drop_target<R>(
        &self,
        f: impl FnOnce(Option<DragDropPayloadGetter<'_>>) -> R,
    ) -> R {
        if !unsafe { ImGui_BeginDragDropTarget() } {
            return f(None);
        }
        let payload = DragDropPayloadGetter {
            _dummy: PhantomData,
        };
        let r = f(Some(payload));
        unsafe { ImGui_EndDragDropTarget() }
        r
    }
    pub fn with_drag_drop_target<R>(
        &self,
        f: impl FnOnce(DragDropPayloadGetter<'_>) -> R,
    ) -> Option<R> {
        self.with_always_drag_drop_target(move |r| r.map(f))
    }

    #[must_use]
    pub fn list_clipper(&self, items_count: usize) -> ListClipper {
        ListClipper {
            items_count,
            items_height: -1.0,
            included_ranges: Vec::new(),
        }
    }

    pub fn shortcut(&self, key_chord: impl Into<KeyChord>) -> bool {
        unsafe { ImGui_Shortcut(key_chord.into().bits(), 0) }
    }
    pub fn shortcut_ex(&self, key_chord: impl Into<KeyChord>, flags: InputFlags) -> bool {
        unsafe { ImGui_Shortcut(key_chord.into().bits(), flags.bits()) }
    }
    pub fn set_next_item_shortcut(&self, key_chord: impl Into<KeyChord>) {
        unsafe {
            ImGui_SetNextItemShortcut(key_chord.into().bits(), 0);
        }
    }
    pub fn set_next_item_shortcut_ex(&self, key_chord: impl Into<KeyChord>, flags: InputFlags) {
        unsafe {
            ImGui_SetNextItemShortcut(key_chord.into().bits(), flags.bits());
        }
    }
    pub fn is_keychord_pressed(&self, key_chord: impl Into<KeyChord>) -> bool {
        unsafe { ImGui_IsKeyChordPressed(key_chord.into().bits()) }
    }

    /// Gets the font details for a `FontId`.
    pub fn get_font(&self, font_id: FontId) -> &Font {
        unsafe {
            let font = self.io().font_atlas().font_ptr(font_id);
            Font::cast(&*font)
        }
    }

    /// Gets more information about a font.
    ///
    /// This is a member of `Ui` instead of `FontAtlas` because it requires the atlas to be fully
    /// built and that is only ensured during the frame, that is when there is a `&Ui`.
    pub fn get_font_baked(
        &self,
        font_id: FontId,
        font_size: f32,
        font_density: Option<f32>,
    ) -> &FontBaked {
        unsafe {
            let font = self.io().font_atlas().font_ptr(font_id);
            let baked = (*font).GetFontBaked(font_size, font_density.unwrap_or(-1.0));
            FontBaked::cast(&*baked)
        }
    }

    pub fn get_atlas_texture_ref(&self) -> TextureRef<'_> {
        let tex_data = self.io().font_atlas().TexData;
        let tex_data = unsafe { &*tex_data };
        TextureRef::Ref(tex_data)
    }

    pub fn get_custom_rect(&self, index: CustomRectIndex) -> Option<TextureRect<'_>> {
        let atlas = self.io().font_atlas();
        let rect = unsafe {
            let mut rect = MaybeUninit::zeroed();
            let ok = atlas.GetCustomRect(index.0, rect.as_mut_ptr());
            if !ok {
                return None;
            }
            rect.assume_init()
        };

        let tex_ref = self.get_atlas_texture_ref();
        Some(TextureRect { rect, tex_ref })
    }

    pub fn dock_space(
        &self,
        id: ImGuiID,
        size: Vector2,
        flags: DockNodeFlags, /*window_class: &WindowClass*/
    ) -> ImGuiID {
        unsafe { ImGui_DockSpace(id, &v2_to_im(size), flags.bits(), std::ptr::null()) }
    }
    pub fn dock_space_over_viewport(
        &self,
        dockspace_id: ImGuiID,
        viewport: &Viewport,
        flags: DockNodeFlags, /*window_class: &WindowClass*/
    ) -> ImGuiID {
        unsafe {
            ImGui_DockSpaceOverViewport(
                dockspace_id,
                viewport.get(),
                flags.bits(),
                std::ptr::null(), //window_class
            )
        }
    }
    pub fn set_next_window_dock_id(&self, dock_id: ImGuiID, cond: Cond) {
        unsafe {
            ImGui_SetNextWindowDockID(dock_id, cond.bits());
        }
    }
    //SetNextWindowClass(const ImGuiWindowClass* window_class)
    pub fn get_window_dock_id(&self) -> ImGuiID {
        unsafe { ImGui_GetWindowDockID() }
    }
    pub fn is_window_docked(&self) -> bool {
        unsafe { ImGui_IsWindowDocked() }
    }

    pub fn get_window_viewport(&self) -> &Viewport {
        unsafe { Viewport::cast(&*ImGui_GetWindowViewport()) }
    }
    pub fn set_next_window_viewport(&self, id: ImGuiID) {
        unsafe { ImGui_SetNextWindowViewport(id) }
    }
    pub fn viewport_foreground_draw_list(&self, viewport: &Viewport) -> WindowDrawList<'_, A> {
        unsafe {
            let ptr = ImGui_GetForegroundDrawList((&raw const *viewport.get()).cast_mut());
            WindowDrawList { ui: self, ptr }
        }
    }
    pub fn viewport_background_draw_list(&self, viewport: &Viewport) -> WindowDrawList<'_, A> {
        unsafe {
            let ptr = ImGui_GetBackgroundDrawList((&raw const *viewport.get()).cast_mut());
            WindowDrawList { ui: self, ptr }
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct TextureRect<'ui> {
    pub rect: ImFontAtlasRect,
    pub tex_ref: TextureRef<'ui>,
}

pub struct ListClipper {
    items_count: usize,
    items_height: f32,
    included_ranges: Vec<std::ops::Range<usize>>,
}

impl ListClipper {
    decl_builder_setter! {items_height: f32}

    pub fn add_included_range(&mut self, range: std::ops::Range<usize>) {
        self.included_ranges.push(range);
    }

    pub fn with(self, mut f: impl FnMut(usize)) {
        unsafe {
            let mut clip = ImGuiListClipper::new();
            clip.Begin(self.items_count as i32, self.items_height);
            for r in self.included_ranges {
                clip.IncludeItemsByIndex(r.start as i32, r.end as i32);
            }
            while clip.Step() {
                for i in clip.DisplayStart..clip.DisplayEnd {
                    f(i as usize);
                }
            }
        }
    }
}

transparent! {
    // TODO: do a proper impl Font?
    pub struct Font(ImFont);
}

transparent! {
    pub struct FontGlyph(ImFontGlyph);
}

impl FontGlyph {
    pub fn p0(&self) -> Vector2 {
        Vector2::new(self.0.X0, self.0.Y0)
    }
    pub fn p1(&self) -> Vector2 {
        Vector2::new(self.0.X1, self.0.Y1)
    }
    pub fn uv0(&self) -> Vector2 {
        Vector2::new(self.0.U0, self.0.V0)
    }
    pub fn uv1(&self) -> Vector2 {
        Vector2::new(self.0.U1, self.0.V1)
    }
    pub fn advance_x(&self) -> f32 {
        self.0.AdvanceX
    }
    pub fn visible(&self) -> bool {
        self.0.Visible() != 0
    }
    pub fn colored(&self) -> bool {
        self.0.Colored() != 0
    }
    pub fn codepoint(&self) -> char {
        char::try_from(self.0.Codepoint()).unwrap()
    }
}

impl std::fmt::Debug for FontGlyph {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.debug_struct("FontGlyph")
            .field("p0", &self.p0())
            .field("p1", &self.p1())
            .field("uv0", &self.uv0())
            .field("uv1", &self.uv1())
            .field("advance_x", &self.advance_x())
            .field("visible", &self.visible())
            .field("colored", &self.colored())
            .field("codepoint", &self.codepoint())
            .finish()
    }
}

transparent! {
    #[derive(Debug)]
    pub struct FontBaked(ImFontBaked);
}

impl FontBaked {
    /// Gets information about a glyph for a font.
    pub fn find_glyph(&self, c: char) -> &FontGlyph {
        unsafe {
            FontGlyph::cast(&*ImFontBaked_FindGlyph(
                (&raw const self.0).cast_mut(),
                ImWchar::from(c),
            ))
        }
    }

    /// Just like `find_glyph` but doesn't use the fallback character for unavailable glyphs.
    pub fn find_glyph_no_fallback(&self, c: char) -> Option<&FontGlyph> {
        unsafe {
            let p =
                ImFontBaked_FindGlyphNoFallback((&raw const self.0).cast_mut(), ImWchar::from(c));
            p.as_ref().map(FontGlyph::cast)
        }
    }

    pub unsafe fn inner(&mut self) -> &mut ImFontBaked {
        &mut self.0
    }

    // The only safe values for a loader to set are these
    pub fn set_ascent(&mut self, ascent: f32) {
        self.0.Ascent = ascent;
    }
    pub fn set_descent(&mut self, descent: f32) {
        self.0.Descent = descent;
    }
}

/// Identifier of a registered font.
///
/// `FontId::default()` will be the default font.
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FontId(u32);

/// Identifier for a registered custom rectangle.
///
/// The `CustomRectIndex::default()` is provided as a convenience, but it is always invalid, and
/// will panic if used.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CustomRectIndex(i32);

impl Default for CustomRectIndex {
    fn default() -> Self {
        // Always invalid, do not use!
        CustomRectIndex(-1)
    }
}

transparent! {
    #[derive(Debug)]
    pub struct FontAtlas(ImFontAtlas);
}

type PixelImage<'a> = image::ImageBuffer<image::Rgba<u8>, &'a mut [u8]>;
type SubPixelImage<'a, 'b> = image::SubImage<&'a mut PixelImage<'b>>;

impl FontAtlas {
    pub unsafe fn texture_ref(&self) -> ImTextureRef {
        self.TexRef
    }
    pub unsafe fn inner(&mut self) -> &mut ImFontAtlas {
        &mut self.0
    }

    pub fn current_texture_unique_id(&self) -> TextureUniqueId {
        unsafe {
            let id = (*self.TexRef._TexData).UniqueID;
            TextureUniqueId(id)
        }
    }

    fn texture_unique_id(&self, uid: TextureUniqueId) -> Option<&ImTextureData> {
        unsafe {
            self.TexList
                .iter()
                .find(|x| (***x).UniqueID == uid.0)
                .map(|p| &**p)
        }
    }

    unsafe fn font_ptr(&self, font: FontId) -> *mut ImFont {
        unsafe {
            // fonts.Fonts is never empty, at least there is the default font
            *self
                .Fonts
                .iter()
                .find(|f| f.as_ref().map(|f| f.FontId) == Some(font.0))
                .unwrap_or(&self.Fonts[0])
        }
    }

    pub fn check_texture_unique_id(&self, uid: TextureUniqueId) -> bool {
        self.texture_unique_id(uid).is_some_and(|x| {
            !matches!(
                x.Status,
                ImTextureStatus::ImTextureStatus_WantDestroy
                    | ImTextureStatus::ImTextureStatus_Destroyed
            )
        })
    }

    pub fn get_texture_by_unique_id(&self, uid: TextureUniqueId) -> Option<TextureId> {
        let p = self.texture_unique_id(uid)?;
        // Allows for ImTextureStatus_WantDestroy, because the TexID may still be valid
        if p.Status == ImTextureStatus::ImTextureStatus_Destroyed || p.TexID == 0 {
            None
        } else {
            unsafe { Some(TextureId::from_id(p.TexID)) }
        }
    }

    /// Adds the given font to the atlas.
    ///
    /// It returns the id to use this font. `FontId` implements `Pushable` so you can use it with
    /// [`Ui::with_push`].
    pub fn add_font(&mut self, font: FontInfo) -> FontId {
        self.add_font_priv(font, false)
    }

    pub fn remove_font(&mut self, font_id: FontId) {
        unsafe {
            let f = self.font_ptr(font_id);
            // Do not delete the default font!
            /*if std::ptr::eq(f, self.Fonts[0]) {
                return;
            }*/
            self.0.RemoveFont(f);
        }
    }

    /// Adds several fonts with as a single ImGui font.
    ///
    /// This is useful mainly if different TTF files have different charset coverage but you want
    /// to use them all as a unit.
    pub fn add_font_collection(&mut self, fonts: impl IntoIterator<Item = FontInfo>) -> FontId {
        let mut fonts = fonts.into_iter();
        let first = fonts.next().expect("empty font collection");
        let id = self.add_font_priv(first, false);
        for font in fonts {
            self.add_font_priv(font, true);
        }
        id
    }
    fn add_font_priv(&mut self, font: FontInfo, merge: bool) -> FontId {
        unsafe {
            let mut fc = ImFontConfig::new();
            // This is ours, do not free()
            fc.FontDataOwnedByAtlas = false;
            fc.MergeMode = merge;
            if !font.name.is_empty() {
                let cname = font.name.as_bytes();
                let name_len = cname.len().min(fc.Name.len() - 1);
                fc.Name[..name_len]
                    .copy_from_slice(std::mem::transmute::<&[u8], &[i8]>(&cname[..name_len]));
                fc.Name[name_len] = 0;
            }
            fc.Flags = font.flags.bits();
            fc.SizePixels = font.size;

            let font_ptr = match font.ttf {
                TtfData::Bytes(bytes) => {
                    self.0.AddFontFromMemoryTTF(
                        bytes.as_ptr() as *mut _,
                        bytes.len() as i32,
                        /* size_pixels */ 0.0,
                        &fc,
                        std::ptr::null(),
                    )
                }
                TtfData::DefaultFont => self.0.AddFontDefault(&fc),
                TtfData::CustomLoader(glyph_loader) => {
                    let ptr = Box::into_raw(Box::new(glyph_loader));
                    fc.FontLoader = &fontloader::FONT_LOADER.0;
                    fc.FontData = ptr as *mut c_void;
                    fc.FontDataOwnedByAtlas = true;
                    self.0.AddFont(&fc)
                }
            };
            let Some(font) = font_ptr.as_ref() else {
                log::error!("Error loading font!");
                return FontId::default();
            };
            FontId(font.FontId)
        }
    }

    /// Adds an arbitrary image to the font atlas.
    ///
    /// The returned `CustomRectIndex` can be used later to draw the image.
    pub fn add_custom_rect(
        &mut self,
        size: impl Into<mint::Vector2<u32>>,
        draw: impl FnOnce(&mut SubPixelImage<'_, '_>),
    ) -> CustomRectIndex {
        let size = size.into();
        unsafe {
            let mut rect = MaybeUninit::zeroed();
            let idx = self.0.AddCustomRect(
                i32::try_from(size.x).unwrap(),
                i32::try_from(size.y).unwrap(),
                rect.as_mut_ptr(),
            );
            let idx = CustomRectIndex(idx);
            let rect = rect.assume_init();
            let tex_data = &(*self.TexData);

            let mut pixel_image = PixelImage::from_raw(
                tex_data.Width as u32,
                tex_data.Height as u32,
                std::slice::from_raw_parts_mut(
                    tex_data.Pixels,
                    tex_data.Width as usize
                        * tex_data.Height as usize
                        * tex_data.BytesPerPixel as usize,
                ),
            )
            .unwrap();

            let mut sub_image =
                pixel_image.sub_image(rect.x as u32, rect.y as u32, rect.w as u32, rect.h as u32);
            draw(&mut sub_image);

            idx
        }
    }

    pub fn remove_custom_rect(&mut self, idx: CustomRectIndex) {
        if idx.0 < 0 {
            return;
        }
        unsafe {
            self.0.RemoveCustomRect(idx.0);
        }
    }
}

transparent_mut! {
    #[derive(Debug)]
    pub struct Io(ImGuiIO);
}

transparent! {
    /// Safe wrapper for `&mut Io`.
    ///
    /// Notably it doesn't implement DerefMut
    #[derive(Debug)]
    pub struct IoMut(ImGuiIO);
}

impl Io {
    pub fn font_atlas(&self) -> &FontAtlas {
        unsafe { FontAtlas::cast(&*self.Fonts) }
    }

    pub fn want_capture_mouse(&self) -> bool {
        self.WantCaptureMouse
    }
    pub fn want_capture_keyboard(&self) -> bool {
        self.WantCaptureKeyboard
    }
    pub fn want_text_input(&self) -> bool {
        self.WantTextInput
    }
    pub fn display_size(&self) -> Vector2 {
        im_to_v2(self.DisplaySize)
    }
    pub fn display_scale(&self) -> f32 {
        self.DisplayFramebufferScale.x
    }

    // The following are not unsafe because if you have a `&mut Io` you alreay can do anything.
    pub fn add_config_flags(&mut self, flags: ConfigFlags) {
        self.ConfigFlags |= flags.bits();
    }
    pub fn remove_config_flags(&mut self, flags: ConfigFlags) {
        self.ConfigFlags &= !flags.bits();
    }
    pub fn add_backend_flags(&mut self, flags: BackendFlags) {
        self.BackendFlags |= flags.bits();
    }
    pub fn remove_backend_flags(&mut self, flags: BackendFlags) {
        self.BackendFlags &= !flags.bits();
    }
    pub fn delta_time(&mut self) -> Duration {
        Duration::from_secs_f32(self.DeltaTime)
    }
    pub fn set_delta_time(&mut self, d: Duration) {
        self.DeltaTime = d.as_secs_f32()
    }
}

impl IoMut {
    pub unsafe fn inner(&mut self) -> &mut Io {
        Io::cast_mut(&mut self.0)
    }
    pub fn set_allow_user_scaling(&mut self, val: bool) {
        self.0.FontAllowUserScaling = val;
    }
    pub fn nav_enable_keyboard(&mut self) {
        unsafe {
            self.inner()
                .add_config_flags(ConfigFlags::NavEnableKeyboard);
        }
    }
    pub fn nav_enable_gamepad(&mut self) {
        unsafe {
            self.inner().add_config_flags(ConfigFlags::NavEnableGamepad);
        }
    }
    pub fn font_atlas_mut(&mut self) -> &mut FontAtlas {
        unsafe { FontAtlas::cast_mut(&mut *self.Fonts) }
    }
}

transparent_mut! {
    #[derive(Debug)]
    pub struct PlatformIo(ImGuiPlatformIO);
}

impl PlatformIo {
    pub unsafe fn textures_mut(&mut self) -> impl Iterator<Item = &mut ImTextureData> {
        self.Textures.iter_mut().map(|t| unsafe { &mut **t })
    }
}

#[derive(Debug)]
pub struct SizeCallbackData<'a> {
    ptr: &'a mut ImGuiSizeCallbackData,
}

impl SizeCallbackData<'_> {
    pub fn pos(&self) -> Vector2 {
        im_to_v2(self.ptr.Pos)
    }
    pub fn current_size(&self) -> Vector2 {
        im_to_v2(self.ptr.CurrentSize)
    }
    pub fn desired_size(&self) -> Vector2 {
        im_to_v2(self.ptr.DesiredSize)
    }
    pub fn set_desired_size(&mut self, sz: Vector2) {
        self.ptr.DesiredSize = v2_to_im(sz);
    }
}

unsafe extern "C" fn call_size_callback<A>(ptr: *mut ImGuiSizeCallbackData) {
    unsafe {
        let ptr = &mut *ptr;
        let id = ptr.UserData as usize;
        let data = SizeCallbackData { ptr };
        Ui::<A>::run_callback(id, data);
    }
}

pub struct WindowDrawList<'ui, A> {
    ui: &'ui Ui<A>,
    ptr: *mut ImDrawList,
}

impl<A> WindowDrawList<'_, A> {
    pub fn add_line(&self, p1: Vector2, p2: Vector2, color: Color, thickness: f32) {
        unsafe {
            ImDrawList_AddLine(
                self.ptr,
                &v2_to_im(p1),
                &v2_to_im(p2),
                color.as_u32(),
                thickness,
            );
        }
    }
    pub fn add_rect(
        &self,
        p_min: Vector2,
        p_max: Vector2,
        color: Color,
        rounding: f32,
        flags: DrawFlags,
        thickness: f32,
    ) {
        unsafe {
            ImDrawList_AddRect(
                self.ptr,
                &v2_to_im(p_min),
                &v2_to_im(p_max),
                color.as_u32(),
                rounding,
                flags.bits(),
                thickness,
            );
        }
    }
    pub fn add_rect_filled(
        &self,
        p_min: Vector2,
        p_max: Vector2,
        color: Color,
        rounding: f32,
        flags: DrawFlags,
    ) {
        unsafe {
            ImDrawList_AddRectFilled(
                self.ptr,
                &v2_to_im(p_min),
                &v2_to_im(p_max),
                color.as_u32(),
                rounding,
                flags.bits(),
            );
        }
    }
    pub fn add_rect_filled_multicolor(
        &self,
        p_min: Vector2,
        p_max: Vector2,
        col_upr_left: Color,
        col_upr_right: Color,
        col_bot_right: Color,
        col_bot_left: Color,
    ) {
        unsafe {
            ImDrawList_AddRectFilledMultiColor(
                self.ptr,
                &v2_to_im(p_min),
                &v2_to_im(p_max),
                col_upr_left.as_u32(),
                col_upr_right.as_u32(),
                col_bot_right.as_u32(),
                col_bot_left.as_u32(),
            );
        }
    }
    pub fn add_quad(
        &self,
        p1: Vector2,
        p2: Vector2,
        p3: Vector2,
        p4: Vector2,
        color: Color,
        thickness: f32,
    ) {
        unsafe {
            ImDrawList_AddQuad(
                self.ptr,
                &v2_to_im(p1),
                &v2_to_im(p2),
                &v2_to_im(p3),
                &v2_to_im(p4),
                color.as_u32(),
                thickness,
            );
        }
    }
    pub fn add_quad_filled(
        &self,
        p1: Vector2,
        p2: Vector2,
        p3: Vector2,
        p4: Vector2,
        color: Color,
    ) {
        unsafe {
            ImDrawList_AddQuadFilled(
                self.ptr,
                &v2_to_im(p1),
                &v2_to_im(p2),
                &v2_to_im(p3),
                &v2_to_im(p4),
                color.as_u32(),
            );
        }
    }
    pub fn add_triangle(
        &self,
        p1: Vector2,
        p2: Vector2,
        p3: Vector2,
        color: Color,
        thickness: f32,
    ) {
        unsafe {
            ImDrawList_AddTriangle(
                self.ptr,
                &v2_to_im(p1),
                &v2_to_im(p2),
                &v2_to_im(p3),
                color.as_u32(),
                thickness,
            );
        }
    }
    pub fn add_triangle_filled(&self, p1: Vector2, p2: Vector2, p3: Vector2, color: Color) {
        unsafe {
            ImDrawList_AddTriangleFilled(
                self.ptr,
                &v2_to_im(p1),
                &v2_to_im(p2),
                &v2_to_im(p3),
                color.as_u32(),
            );
        }
    }
    pub fn add_circle(
        &self,
        center: Vector2,
        radius: f32,
        color: Color,
        num_segments: i32,
        thickness: f32,
    ) {
        unsafe {
            ImDrawList_AddCircle(
                self.ptr,
                &v2_to_im(center),
                radius,
                color.as_u32(),
                num_segments,
                thickness,
            );
        }
    }
    pub fn add_circle_filled(&self, center: Vector2, radius: f32, color: Color, num_segments: i32) {
        unsafe {
            ImDrawList_AddCircleFilled(
                self.ptr,
                &v2_to_im(center),
                radius,
                color.as_u32(),
                num_segments,
            );
        }
    }
    pub fn add_ngon(
        &self,
        center: Vector2,
        radius: f32,
        color: Color,
        num_segments: i32,
        thickness: f32,
    ) {
        unsafe {
            ImDrawList_AddNgon(
                self.ptr,
                &v2_to_im(center),
                radius,
                color.as_u32(),
                num_segments,
                thickness,
            );
        }
    }
    pub fn add_ngon_filled(&self, center: Vector2, radius: f32, color: Color, num_segments: i32) {
        unsafe {
            ImDrawList_AddNgonFilled(
                self.ptr,
                &v2_to_im(center),
                radius,
                color.as_u32(),
                num_segments,
            );
        }
    }
    pub fn add_ellipse(
        &self,
        center: Vector2,
        radius: Vector2,
        color: Color,
        rot: f32,
        num_segments: i32,
        thickness: f32,
    ) {
        unsafe {
            ImDrawList_AddEllipse(
                self.ptr,
                &v2_to_im(center),
                &v2_to_im(radius),
                color.as_u32(),
                rot,
                num_segments,
                thickness,
            );
        }
    }
    pub fn add_ellipse_filled(
        &self,
        center: Vector2,
        radius: Vector2,
        color: Color,
        rot: f32,
        num_segments: i32,
    ) {
        unsafe {
            ImDrawList_AddEllipseFilled(
                self.ptr,
                &v2_to_im(center),
                &v2_to_im(radius),
                color.as_u32(),
                rot,
                num_segments,
            );
        }
    }
    pub fn add_text(&self, pos: Vector2, color: Color, text: &str) {
        unsafe {
            let (start, end) = text_ptrs(text);
            ImDrawList_AddText(self.ptr, &v2_to_im(pos), color.as_u32(), start, end);
        }
    }
    pub fn add_text_ex(
        &self,
        font: FontId,
        font_size: f32,
        pos: Vector2,
        color: Color,
        text: &str,
        wrap_width: f32,
        cpu_fine_clip_rect: Option<ImVec4>,
    ) {
        unsafe {
            let (start, end) = text_ptrs(text);
            ImDrawList_AddText1(
                self.ptr,
                self.ui.io().font_atlas().font_ptr(font),
                font_size,
                &v2_to_im(pos),
                color.as_u32(),
                start,
                end,
                wrap_width,
                cpu_fine_clip_rect
                    .as_ref()
                    .map(|x| x as *const _)
                    .unwrap_or(null()),
            );
        }
    }
    pub fn add_polyline(&self, points: &[ImVec2], color: Color, flags: DrawFlags, thickness: f32) {
        unsafe {
            ImDrawList_AddPolyline(
                self.ptr,
                points.as_ptr(),
                points.len() as i32,
                color.as_u32(),
                flags.bits(),
                thickness,
            );
        }
    }
    pub fn add_convex_poly_filled(&self, points: &[ImVec2], color: Color) {
        unsafe {
            ImDrawList_AddConvexPolyFilled(
                self.ptr,
                points.as_ptr(),
                points.len() as i32,
                color.as_u32(),
            );
        }
    }
    pub fn add_concave_poly_filled(&self, points: &[ImVec2], color: Color) {
        unsafe {
            ImDrawList_AddConcavePolyFilled(
                self.ptr,
                points.as_ptr(),
                points.len() as i32,
                color.as_u32(),
            );
        }
    }
    pub fn add_bezier_cubic(
        &self,
        p1: Vector2,
        p2: Vector2,
        p3: Vector2,
        p4: Vector2,
        color: Color,
        thickness: f32,
        num_segments: i32,
    ) {
        unsafe {
            ImDrawList_AddBezierCubic(
                self.ptr,
                &v2_to_im(p1),
                &v2_to_im(p2),
                &v2_to_im(p3),
                &v2_to_im(p4),
                color.as_u32(),
                thickness,
                num_segments,
            );
        }
    }
    pub fn add_bezier_quadratic(
        &self,
        p1: Vector2,
        p2: Vector2,
        p3: Vector2,
        color: Color,
        thickness: f32,
        num_segments: i32,
    ) {
        unsafe {
            ImDrawList_AddBezierQuadratic(
                self.ptr,
                &v2_to_im(p1),
                &v2_to_im(p2),
                &v2_to_im(p3),
                color.as_u32(),
                thickness,
                num_segments,
            );
        }
    }
    pub fn add_image(
        &self,
        texture_ref: TextureRef,
        p_min: Vector2,
        p_max: Vector2,
        uv_min: Vector2,
        uv_max: Vector2,
        color: Color,
    ) {
        unsafe {
            ImDrawList_AddImage(
                self.ptr,
                texture_ref.tex_ref(),
                &v2_to_im(p_min),
                &v2_to_im(p_max),
                &v2_to_im(uv_min),
                &v2_to_im(uv_max),
                color.as_u32(),
            );
        }
    }
    pub fn add_image_quad(
        &self,
        texture_ref: TextureRef,
        p1: Vector2,
        p2: Vector2,
        p3: Vector2,
        p4: Vector2,
        uv1: Vector2,
        uv2: Vector2,
        uv3: Vector2,
        uv4: Vector2,
        color: Color,
    ) {
        unsafe {
            ImDrawList_AddImageQuad(
                self.ptr,
                texture_ref.tex_ref(),
                &v2_to_im(p1),
                &v2_to_im(p2),
                &v2_to_im(p3),
                &v2_to_im(p4),
                &v2_to_im(uv1),
                &v2_to_im(uv2),
                &v2_to_im(uv3),
                &v2_to_im(uv4),
                color.as_u32(),
            );
        }
    }
    pub fn add_image_rounded(
        &self,
        texture_ref: TextureRef,
        p_min: Vector2,
        p_max: Vector2,
        uv_min: Vector2,
        uv_max: Vector2,
        color: Color,
        rounding: f32,
        flags: DrawFlags,
    ) {
        unsafe {
            ImDrawList_AddImageRounded(
                self.ptr,
                texture_ref.tex_ref(),
                &v2_to_im(p_min),
                &v2_to_im(p_max),
                &v2_to_im(uv_min),
                &v2_to_im(uv_max),
                color.as_u32(),
                rounding,
                flags.bits(),
            );
        }
    }

    pub fn add_callback(&self, cb: impl FnOnce(&mut A) + 'static) {
        // Callbacks are only called once, convert the FnOnce into an FnMut to register
        // They are called after `do_ui` so first argument pointer is valid.
        // The second argument is not used, set to `()``.
        let mut cb = Some(cb);
        unsafe {
            let id = self.ui.push_callback(move |a, _: ()| {
                if let Some(cb) = cb.take() {
                    cb(&mut *a);
                }
            });
            ImDrawList_AddCallback(
                self.ptr,
                Some(call_drawlist_callback::<A>),
                id as *mut c_void,
                0,
            );
        }
    }
    pub fn add_draw_cmd(&self) {
        unsafe {
            ImDrawList_AddDrawCmd(self.ptr);
        }
    }
}

unsafe extern "C" fn call_drawlist_callback<A>(
    _parent_list: *const ImDrawList,
    cmd: *const ImDrawCmd,
) {
    unsafe {
        let id = (*cmd).UserCallbackData as usize;
        Ui::<A>::run_callback(id, ());
    }
}

/// Represents any type that can be converted to a Dear ImGui hash id.
pub trait Hashable {
    // These are unsafe because they should be called only inside a frame (holding a &mut Ui)
    unsafe fn get_id(&self) -> ImGuiID;
    unsafe fn push(&self);
}

impl Hashable for &str {
    unsafe fn get_id(&self) -> ImGuiID {
        unsafe {
            let (start, end) = text_ptrs(self);
            ImGui_GetID1(start, end)
        }
    }
    unsafe fn push(&self) {
        unsafe {
            let (start, end) = text_ptrs(self);
            ImGui_PushID1(start, end);
        }
    }
}

impl Hashable for usize {
    unsafe fn get_id(&self) -> ImGuiID {
        unsafe { ImGui_GetID2(*self as *const c_void) }
    }
    unsafe fn push(&self) {
        unsafe {
            ImGui_PushID2(*self as *const c_void);
        }
    }
}

/// Any value that can be applied with a _push_ function and unapplied with a _pop_ function.
///
/// Apply to the current frame using [`Ui::with_push`]. If you want to apply several values at the
/// same time use a tuple or an array.
/// Only tuples up to 4 values are supported, but you can apply arbitrarily many pushables by
/// creating tuples of tuples: `(A, B, C, (D, E, F, (G, H, I, J)))`.
pub trait Pushable {
    unsafe fn push(&self);
    unsafe fn pop(&self);
}

struct PushableGuard<'a, P: Pushable>(&'a P);

impl<P: Pushable> Drop for PushableGuard<'_, P> {
    fn drop(&mut self) {
        unsafe {
            self.0.pop();
        }
    }
}

#[allow(clippy::needless_lifetimes)]
unsafe fn push_guard<'a, P: Pushable>(p: &'a P) -> PushableGuard<'a, P> {
    unsafe {
        p.push();
        PushableGuard(p)
    }
}

/// A [`Pushable`] that does nothing.
impl Pushable for () {
    unsafe fn push(&self) {}
    unsafe fn pop(&self) {}
}

impl<A: Pushable> Pushable for (A,) {
    unsafe fn push(&self) {
        unsafe {
            self.0.push();
        }
    }
    unsafe fn pop(&self) {
        unsafe {
            self.0.pop();
        }
    }
}

impl<A: Pushable, B: Pushable> Pushable for (A, B) {
    unsafe fn push(&self) {
        unsafe {
            self.0.push();
            self.1.push();
        }
    }
    unsafe fn pop(&self) {
        unsafe {
            self.1.pop();
            self.0.pop();
        }
    }
}

impl<A: Pushable, B: Pushable, C: Pushable> Pushable for (A, B, C) {
    unsafe fn push(&self) {
        unsafe {
            self.0.push();
            self.1.push();
            self.2.push();
        }
    }
    unsafe fn pop(&self) {
        unsafe {
            self.2.pop();
            self.1.pop();
            self.0.pop();
        }
    }
}

impl<A: Pushable, B: Pushable, C: Pushable, D: Pushable> Pushable for (A, B, C, D) {
    unsafe fn push(&self) {
        unsafe {
            self.0.push();
            self.1.push();
            self.2.push();
            self.3.push();
        }
    }
    unsafe fn pop(&self) {
        unsafe {
            self.3.pop();
            self.2.pop();
            self.1.pop();
            self.0.pop();
        }
    }
}

impl Pushable for &[&dyn Pushable] {
    unsafe fn push(&self) {
        unsafe {
            for st in *self {
                st.push();
            }
        }
    }
    unsafe fn pop(&self) {
        unsafe {
            for st in self.iter().rev() {
                st.pop();
            }
        }
    }
}

/// A [`Pushable`] that is applied optionally.
impl<T: Pushable> Pushable for Option<T> {
    unsafe fn push(&self) {
        unsafe {
            if let Some(s) = self {
                s.push();
            }
        }
    }
    unsafe fn pop(&self) {
        unsafe {
            if let Some(s) = self {
                s.pop();
            }
        }
    }
}

//TODO rework the font pushables
impl Pushable for FontId {
    unsafe fn push(&self) {
        unsafe {
            let font = current_font_ptr(*self);
            ImGui_PushFont(font, 0.0);
        }
    }
    unsafe fn pop(&self) {
        unsafe {
            ImGui_PopFont();
        }
    }
}

pub struct FontSize(pub f32);

impl Pushable for FontSize {
    unsafe fn push(&self) {
        unsafe {
            // maybe this should get ui and do ui.scale()
            ImGui_PushFont(std::ptr::null_mut(), self.0);
        }
    }
    unsafe fn pop(&self) {
        unsafe {
            ImGui_PopFont();
        }
    }
}

pub struct FontAndSize(pub FontId, pub f32);

impl Pushable for FontAndSize {
    unsafe fn push(&self) {
        unsafe {
            ImGui_PushFont(current_font_ptr(self.0), self.1);
        }
    }
    unsafe fn pop(&self) {
        unsafe {
            ImGui_PopFont();
        }
    }
}

pub type StyleColor = (ColorId, Color);

#[derive(Copy, Clone, Debug)]
pub enum TextureRef<'a> {
    Id(TextureId),
    Ref(&'a ImTextureData),
}

impl TextureRef<'_> {
    pub unsafe fn tex_ref(&self) -> ImTextureRef {
        match self {
            TextureRef::Id(TextureId(id)) => ImTextureRef {
                _TexData: null_mut(),
                _TexID: *id,
            },
            TextureRef::Ref(tex_data) => ImTextureRef {
                _TexData: (&raw const **tex_data).cast_mut(),
                _TexID: 0,
            },
        }
    }

    pub unsafe fn tex_id(&self) -> TextureId {
        unsafe {
            match self {
                TextureRef::Id(tex_id) => *tex_id,
                TextureRef::Ref(tex_data) => {
                    let id = tex_data.TexID;
                    TextureId::from_id(id)
                }
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TextureId(ImTextureID);

impl TextureId {
    pub fn id(&self) -> ImTextureID {
        self.0
    }
    pub unsafe fn from_id(id: ImTextureID) -> Self {
        Self(id)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TextureUniqueId(i32);

impl Pushable for StyleColor {
    unsafe fn push(&self) {
        unsafe {
            ImGui_PushStyleColor1(self.0.bits(), &self.1.into());
        }
    }
    unsafe fn pop(&self) {
        unsafe {
            ImGui_PopStyleColor(1);
        }
    }
}

impl Pushable for [StyleColor] {
    unsafe fn push(&self) {
        unsafe {
            for sc in self {
                sc.push();
            }
        }
    }
    unsafe fn pop(&self) {
        unsafe {
            ImGui_PopStyleColor(self.len() as i32);
        }
    }
}

impl<const N: usize> Pushable for [StyleColor; N] {
    unsafe fn push(&self) {
        unsafe {
            self.as_slice().push();
        }
    }
    unsafe fn pop(&self) {
        unsafe {
            self.as_slice().pop();
        }
    }
}

pub type StyleColorF = (ColorId, ImVec4);

impl Pushable for StyleColorF {
    unsafe fn push(&self) {
        unsafe {
            ImGui_PushStyleColor1(self.0.bits(), &self.1);
        }
    }
    unsafe fn pop(&self) {
        unsafe {
            ImGui_PopStyleColor(1);
        }
    }
}

impl Pushable for [StyleColorF] {
    unsafe fn push(&self) {
        unsafe {
            for sc in self {
                sc.push();
            }
        }
    }
    unsafe fn pop(&self) {
        unsafe {
            ImGui_PopStyleColor(self.len() as i32);
        }
    }
}

impl<const N: usize> Pushable for [StyleColorF; N] {
    unsafe fn push(&self) {
        unsafe {
            self.as_slice().push();
        }
    }
    unsafe fn pop(&self) {
        unsafe {
            self.as_slice().pop();
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum StyleValue {
    F32(f32),
    Vec2(Vector2),
    X(f32),
    Y(f32),
}

pub type Style = (StyleVar, StyleValue);

impl Pushable for Style {
    unsafe fn push(&self) {
        unsafe {
            match self.1 {
                StyleValue::F32(f) => ImGui_PushStyleVar(self.0.bits(), f),
                StyleValue::Vec2(v) => ImGui_PushStyleVar1(self.0.bits(), &v2_to_im(v)),
                StyleValue::X(x) => ImGui_PushStyleVarX(self.0.bits(), x),
                StyleValue::Y(y) => ImGui_PushStyleVarX(self.0.bits(), y),
            }
        }
    }
    unsafe fn pop(&self) {
        unsafe {
            ImGui_PopStyleVar(1);
        }
    }
}

impl Pushable for [Style] {
    unsafe fn push(&self) {
        unsafe {
            for sc in self {
                sc.push();
            }
        }
    }
    unsafe fn pop(&self) {
        unsafe {
            ImGui_PopStyleVar(self.len() as i32);
        }
    }
}

impl<const N: usize> Pushable for [Style; N] {
    unsafe fn push(&self) {
        unsafe {
            self.as_slice().push();
        }
    }
    unsafe fn pop(&self) {
        unsafe {
            self.as_slice().pop();
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ItemWidth(pub f32);

impl Pushable for ItemWidth {
    unsafe fn push(&self) {
        unsafe {
            ImGui_PushItemWidth(self.0);
        }
    }
    unsafe fn pop(&self) {
        unsafe {
            ImGui_PopItemWidth();
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Indent(pub f32);

impl Pushable for Indent {
    unsafe fn push(&self) {
        unsafe {
            ImGui_Indent(self.0);
        }
    }
    unsafe fn pop(&self) {
        unsafe {
            ImGui_Unindent(self.0);
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct TextWrapPos(pub f32);

impl Pushable for TextWrapPos {
    unsafe fn push(&self) {
        unsafe {
            ImGui_PushTextWrapPos(self.0);
        }
    }
    unsafe fn pop(&self) {
        unsafe {
            ImGui_PopTextWrapPos();
        }
    }
}

impl Pushable for (ItemFlags, bool) {
    unsafe fn push(&self) {
        unsafe {
            ImGui_PushItemFlag(self.0.bits(), self.1);
        }
    }
    unsafe fn pop(&self) {
        unsafe {
            ImGui_PopItemFlag();
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ItemId<H: Hashable>(pub H);

impl<H: Hashable> Pushable for ItemId<H> {
    unsafe fn push(&self) {
        unsafe {
            self.0.push();
        }
    }
    unsafe fn pop(&self) {
        unsafe {
            ImGui_PopID();
        }
    }
}

transparent! {
    #[derive(Debug)]
    pub struct Viewport(ImGuiViewport);
}

impl Viewport {
    pub fn flags(&self) -> ViewportFlags {
        ViewportFlags::from_bits_truncate(self.Flags)
    }
    pub fn pos(&self) -> Vector2 {
        im_to_v2(self.Pos)
    }
    pub fn size(&self) -> Vector2 {
        im_to_v2(self.Size)
    }
    pub fn work_pos(&self) -> Vector2 {
        im_to_v2(self.WorkPos)
    }
    pub fn work_size(&self) -> Vector2 {
        im_to_v2(self.WorkSize)
    }
}

decl_builder_with_opt! { TableConfig, ImGui_BeginTable, ImGui_EndTable () (S: IntoCStr)
    (
        str_id (S::Temp) (str_id.as_ptr()),
        column (i32) (column),
        flags (TableFlags) (flags.bits()),
        outer_size (ImVec2) (&outer_size),
        inner_width (f32) (inner_width),
    )
    {
        decl_builder_setter!{flags: TableFlags}
        decl_builder_setter_vector2!{outer_size: Vector2}
        decl_builder_setter!{inner_width: f32}
    }
    {
        pub fn table_config<S: IntoCStr>(&self, str_id: LblId<S>, column: i32) -> TableConfig<S> {
            TableConfig {
                str_id: str_id.into(),
                column,
                flags: TableFlags::None,
                outer_size: im_vec2(0.0, 0.0),
                inner_width: 0.0,
                push: (),
            }
        }
        pub fn table_next_row(&self, flags: TableRowFlags, min_row_height: f32) {
            unsafe {
                ImGui_TableNextRow(flags.bits(), min_row_height);
            }
        }
        pub fn table_next_column(&self) -> bool {
            unsafe {
                ImGui_TableNextColumn()
            }
        }
        pub fn table_set_column_index(&self, column_n: i32) -> bool {
            unsafe {
                ImGui_TableSetColumnIndex(column_n)
            }
        }
        pub fn table_setup_column(&self, label: impl IntoCStr, flags: TableColumnFlags, init_width_or_weight: f32, user_id: ImGuiID) {
            unsafe {
                ImGui_TableSetupColumn(label.into().as_ptr(), flags.bits(), init_width_or_weight, user_id);
            }
        }
        pub fn table_setup_scroll_freeze(&self, cols: i32, rows: i32) {
            unsafe {
                ImGui_TableSetupScrollFreeze(cols, rows);
            }
        }
        pub fn table_headers_row(&self) {
            unsafe {
                ImGui_TableHeadersRow();
            }
        }
        pub fn table_angle_headers_row(&self) {
            unsafe {
                ImGui_TableAngledHeadersRow();
            }
        }
        pub fn table_get_columns_count(&self) -> i32 {
            unsafe {
                ImGui_TableGetColumnCount()
            }
        }
        pub fn table_get_column_index(&self) -> i32 {
            unsafe {
                ImGui_TableGetColumnIndex()
            }
        }
        /// Can return one-pass the last column if hovering the empty space
        pub fn table_get_hovered_column(&self) -> Option<i32> {
            unsafe {
                let res = ImGui_TableGetHoveredColumn();
                if res < 0 {
                    None
                } else {
                    Some(res)
                }
            }
        }
        pub fn table_get_row_index(&self) -> i32 {
            unsafe {
                ImGui_TableGetRowIndex()
            }
        }
        pub fn table_get_column_flags(&self, column_n: Option<i32>) -> TableColumnFlags {
            let bits = unsafe {
                ImGui_TableGetColumnFlags(column_n.unwrap_or(-1))
            };
            TableColumnFlags::from_bits_truncate(bits)
        }
        pub fn table_get_column_name(&self, column_n: Option<i32>) -> String {
            unsafe {
                let c_str = ImGui_TableGetColumnName(column_n.unwrap_or(-1));
                CStr::from_ptr(c_str).to_string_lossy().into_owned()
            }
        }
        pub fn table_set_column_enabled(&self, column_n: Option<i32>, enabled: bool) {
            unsafe {
                ImGui_TableSetColumnEnabled(column_n.unwrap_or(-1), enabled);
            };
        }
        pub fn table_set_bg_color(&self, target: TableBgTarget, color: Color, column_n: Option<i32>) {
            unsafe {
                ImGui_TableSetBgColor(target.bits(), color.as_u32(), column_n.unwrap_or(-1));
            };
        }
        pub fn table_with_sort_specs(&self, sort_fn: impl FnOnce(&[TableColumnSortSpec])) {
            self.table_with_sort_specs_always(|dirty, spec| {
                if dirty {
                    sort_fn(spec);
                }
                false
            })
        }
        /// The `sort_fn` takes the old `dirty` and returns the new `dirty`.
        pub fn table_with_sort_specs_always(&self, sort_fn: impl FnOnce(bool, &[TableColumnSortSpec]) -> bool) {
            unsafe {
                let specs = ImGui_TableGetSortSpecs();
                if specs.is_null() {
                    return;
                }
                // SAFETY: TableColumnSortSpec is a repr(transparent), so this pointer cast should be ok
                let slice = {
                    let len = (*specs).SpecsCount as usize;
                    if len == 0 {
                        &[]
                    } else {
                        let ptr = std::mem::transmute::<*const ImGuiTableColumnSortSpecs, *const TableColumnSortSpec>((*specs).Specs);
                        std::slice::from_raw_parts(ptr, len)
                    }
                };
                (*specs).SpecsDirty = sort_fn((*specs).SpecsDirty, slice);
            }
        }
    }
}

/// Helper token class that allows to set the drag&drop payload, once.
pub struct DragDropPayloadSetter<'a> {
    _dummy: PhantomData<&'a ()>,
}

/// This is a sub-set of [`Cond`], only for drag&drop payloads.
pub enum DragDropPayloadCond {
    Always,
    Once,
}

impl DragDropPayloadSetter<'_> {
    pub fn set(self, type_: impl IntoCStr, data: &[u8], cond: DragDropPayloadCond) -> bool {
        // For some reason ImGui does not accept a non-null pointer with length 0.
        let ptr = if data.is_empty() {
            null()
        } else {
            data.as_ptr() as *const c_void
        };
        let len = data.len();
        let cond = match cond {
            DragDropPayloadCond::Always => Cond::Always,
            DragDropPayloadCond::Once => Cond::Once,
        };
        unsafe { ImGui_SetDragDropPayload(type_.into().as_ptr(), ptr, len, cond.bits()) }
    }
}

/// Helpar class to get the drag&drop payload.
pub struct DragDropPayloadGetter<'a> {
    _dummy: PhantomData<&'a ()>,
}

/// The payload of a drag&drop operation.
///
/// It contains a "type", and a byte array.
pub struct DragDropPayload<'a> {
    pay: &'a ImGuiPayload,
}

impl<'a> DragDropPayloadGetter<'a> {
    pub fn any(&self, flags: DragDropAcceptFlags) -> Option<DragDropPayload<'a>> {
        unsafe {
            let pay = ImGui_AcceptDragDropPayload(null(), flags.bits());
            if pay.is_null() {
                None
            } else {
                Some(DragDropPayload { pay: &*pay })
            }
        }
    }
    pub fn by_type(
        &self,
        type_: impl IntoCStr,
        flags: DragDropAcceptFlags,
    ) -> Option<DragDropPayload<'a>> {
        unsafe {
            let pay = ImGui_AcceptDragDropPayload(type_.into().as_ptr(), flags.bits());
            if pay.is_null() {
                None
            } else {
                Some(DragDropPayload { pay: &*pay })
            }
        }
    }
    pub fn peek(&self) -> Option<DragDropPayload<'a>> {
        unsafe {
            let pay = ImGui_GetDragDropPayload();
            if pay.is_null() {
                None
            } else {
                Some(DragDropPayload { pay: &*pay })
            }
        }
    }
}

impl DragDropPayload<'_> {
    //WARNING: inline functions
    pub fn is_data_type(&self, type_: impl IntoCStr) -> bool {
        if self.pay.DataFrameCount == -1 {
            return false;
        }
        let data_type = unsafe { std::mem::transmute::<&[i8], &[u8]>(&self.pay.DataType) };
        let data_type = CStr::from_bytes_until_nul(data_type).unwrap();
        data_type == type_.into().as_ref()
    }
    pub fn type_(&self) -> Cow<'_, str> {
        let data_type = unsafe { std::mem::transmute::<&[i8], &[u8]>(&self.pay.DataType) };
        let data_type = CStr::from_bytes_until_nul(data_type).unwrap();
        data_type.to_string_lossy()
    }
    pub fn is_preview(&self) -> bool {
        self.pay.Preview
    }
    pub fn is_delivery(&self) -> bool {
        self.pay.Delivery
    }
    pub fn data(&self) -> &[u8] {
        if self.pay.Data.is_null() {
            &[]
        } else {
            unsafe {
                std::slice::from_raw_parts(self.pay.Data as *const u8, self.pay.DataSize as usize)
            }
        }
    }
}

pub const PAYLOAD_TYPE_COLOR_3F: &CStr =
    unsafe { CStr::from_bytes_with_nul_unchecked(IMGUI_PAYLOAD_TYPE_COLOR_3F) };
pub const PAYLOAD_TYPE_COLOR_4F: &CStr =
    unsafe { CStr::from_bytes_with_nul_unchecked(IMGUI_PAYLOAD_TYPE_COLOR_4F) };

/// This is an ImGuiKey plus several ImGuiMods.
///
/// Functions that use a `KeyChord` usually get a `impl Into<KeyChord>`. That is
/// implemented also for `Key` and `(KeyMod, Key)`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct KeyChord(ImGuiKey);

impl KeyChord {
    pub fn new(mods: KeyMod, key: Key) -> KeyChord {
        KeyChord(ImGuiKey(mods.bits() | key.bits().0))
    }
    pub fn bits(&self) -> i32 {
        self.0.0
    }
    pub fn from_bits(bits: i32) -> Option<KeyChord> {
        // Validate that the bits are valid when building self
        let key = bits & !ImGuiKey::ImGuiMod_Mask_.0;
        let mods = bits & ImGuiKey::ImGuiMod_Mask_.0;
        match (Key::from_bits(ImGuiKey(key)), KeyMod::from_bits(mods)) {
            (Some(_), Some(_)) => Some(KeyChord(ImGuiKey(bits))),
            _ => None,
        }
    }
    pub fn key(&self) -> Key {
        let key = self.bits() & !ImGuiKey::ImGuiMod_Mask_.0;
        Key::from_bits(ImGuiKey(key)).unwrap_or(Key::None)
    }
    pub fn mods(&self) -> KeyMod {
        let mods = self.bits() & ImGuiKey::ImGuiMod_Mask_.0;
        KeyMod::from_bits_truncate(mods)
    }
}

impl From<Key> for KeyChord {
    fn from(value: Key) -> Self {
        KeyChord::new(KeyMod::None, value)
    }
}

impl From<(KeyMod, Key)> for KeyChord {
    fn from(value: (KeyMod, Key)) -> Self {
        KeyChord::new(value.0, value.1)
    }
}

/// Return type for `Ui::table_get_sort_specs`.
#[repr(transparent)]
pub struct TableColumnSortSpec(ImGuiTableColumnSortSpecs);

impl std::fmt::Debug for TableColumnSortSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TableColumnSortSpec")
            .field("id", &self.id())
            .field("index", &self.index())
            .field("sort_order", &self.sort_order())
            .field("sort_direction", &self.sort_direction())
            .finish()
    }
}

impl TableColumnSortSpec {
    pub fn id(&self) -> ImGuiID {
        self.0.ColumnUserID
    }
    pub fn index(&self) -> usize {
        self.0.ColumnIndex as usize
    }
    pub fn sort_order(&self) -> usize {
        self.0.SortOrder as usize
    }
    pub fn sort_direction(&self) -> SortDirection {
        SortDirection::from_bits(self.0.SortDirection).unwrap_or(SortDirection::None)
    }
}
