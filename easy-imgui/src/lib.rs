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
 * ```rust
 * use easy_imgui_window::{
 *     easy_imgui as imgui,
 *     MainWindow,
 *     MainWindowWithRenderer,
 *     winit::event_loop::EventLoopBuilder,
 * };
 *
 * // The App type, this will do the actual app. stuff.
 * struct App;
 *
 * impl imgui::UiBuilder for App {
 *     // There are other function in this trait, but this is the only one
 *     // mandatory and the most important: it build the UI.
 *     fn do_ui(&mut self, ui: &imgui::Ui<Self>) {
 *         ui.show_demo_window(None);
 *     }
 * }
 *
 * fn main() {
 *     // Create a `winit` event loop.
 *     let event_loop = EventLoopBuilder::new().build().unwrap();
 *     // Create a `winit` window.
 *     let main_window = MainWindow::new(&event_loop, "Example").unwrap();
 *     // Create a `easy-imgui` window.
 *     let mut window = MainWindowWithRenderer::new(main_window);
 *
 *     // Create the app object.
 *     let mut app = App;
 *
 *     // Run the loop.
 *     event_loop.run(move |event, w| {
 *         // Do the magic!
 *         let res = window.do_event(&mut app, &event, w);
 *         // If the user requested to close the app, you should consider doing that.
 *         if res.is_break() {
 *             w.exit();
 *         }
 *     }).unwrap();
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
 *  * `docking`: Uses the `docking` branch of Dear ImGui. Note that this is considered somewhat
 *    experimental.
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
use std::cell::{Cell, RefCell};
use std::ffi::{c_char, c_void, CStr, CString};
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::ptr::{null, null_mut};

/// A type alias of the `cgmath::Vector2<f32>`.
///
/// Used in this crate to describe a 2D position or size.
/// The equivalent type in Dear ImGui would be [`ImVec2`].
pub type Vector2 = cgmath::Vector2<f32>;

mod enums;
pub mod style;

pub use easy_imgui_sys::{self, ImGuiID};
pub use enums::*;
pub use image;
pub use mint;

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

fn merge_generation(id: usize, gen: usize) -> usize {
    if (id & GEN_ID_MASK) != id {
        panic!("UI callback overflow")
    }
    (gen << GEN_ID_BITS) | id
}
fn remove_generation(id: usize, gen: usize) -> Option<usize> {
    if (id >> GEN_ID_BITS) != (gen & GEN_MASK) {
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
pub const fn v2_to_im(v: Vector2) -> ImVec2 {
    ImVec2 { x: v.x, y: v.y }
}
/// Helper function to create a `Vector2`.
pub const fn im_to_v2(v: ImVec2) -> Vector2 {
    Vector2 { x: v.x, y: v.y }
}

/// A color is stored as a `[r, g, b, a]`, each value between 0.0 and 1.0.
#[derive(Debug, Copy, Clone, PartialEq)]
#[repr(C)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}
impl Color {
    // Primary and secondary colors
    pub const TRANSPARENT: Color = Color::new(0.0, 0.0, 0.0, 0.0);
    pub const WHITE: Color = Color::new(1.0, 1.0, 1.0, 1.0);
    pub const BLACK: Color = Color::new(0.0, 0.0, 0.0, 1.0);
    pub const RED: Color = Color::new(1.0, 0.0, 0.0, 1.0);
    pub const GREEN: Color = Color::new(0.0, 1.0, 0.0, 1.0);
    pub const BLUE: Color = Color::new(0.0, 0.0, 1.0, 1.0);
    pub const YELLOW: Color = Color::new(1.0, 1.0, 0.0, 1.0);
    pub const MAGENTA: Color = Color::new(1.0, 0.0, 1.0, 1.0);
    pub const CYAN: Color = Color::new(0.0, 1.0, 1.0, 1.0);

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
    imgui: *mut ImGuiContext,
    pending_atlas: bool,
}

pub struct CurrentContext<'a> {
    ctx: &'a mut Context,
}

impl Context {
    pub unsafe fn new() -> Context {
        let imgui = unsafe {
            let imgui = ImGui_CreateContext(null_mut());
            ImGui_SetCurrentContext(imgui);

            let io = ImGui_GetIO();
            (*io).IniFilename = null();

            // If you eanbled the "docking" feature you will want to use it
            #[cfg(feature = "docking")]
            {
                (*io).ConfigFlags |= ConfigFlags::DockingEnable.bits();
            }

            //ImGui_StyleColorsDark(null_mut());
            imgui
        };
        Context {
            imgui,
            pending_atlas: true,
        }
    }
    /// Makes this context the current one.
    ///
    /// SAFETY: Do not make two different contexts current at the same time
    /// in the same thread.
    pub unsafe fn set_current(&mut self) -> CurrentContext<'_> {
        ImGui_SetCurrentContext(self.imgui);
        CurrentContext { ctx: self }
    }
    /// The next time [`CurrentContext::do_frame()`] is called, it will trigger a call to
    /// [`UiBuilder::build_custom_atlas`].
    pub fn invalidate_font_atlas(&mut self) {
        self.pending_atlas = true;
    }
}

impl CurrentContext<'_> {
    pub fn set_allow_user_scaling(&mut self, val: bool) {
        unsafe {
            let io = ImGui_GetIO();
            (*io).FontAllowUserScaling = val;
        }
    }
    pub fn want_capture_mouse(&self) -> bool {
        unsafe {
            let io = &*ImGui_GetIO();
            io.WantCaptureMouse
        }
    }
    pub fn want_capture_keyboard(&self) -> bool {
        unsafe {
            let io = &*ImGui_GetIO();
            io.WantCaptureKeyboard
        }
    }
    pub fn want_text_input(&self) -> bool {
        unsafe {
            let io = &*ImGui_GetIO();
            io.WantTextInput
        }
    }
    pub fn io(&self) -> &ImGuiIO {
        unsafe { &*ImGui_GetIO() }
    }
    pub fn io_mut(&mut self) -> &mut ImGuiIO {
        unsafe { &mut *ImGui_GetIO() }
    }
    // This is unsafe because you could break thing setting weird flags
    // If possible use the safe wrappers below
    pub unsafe fn add_config_flags(&mut self, flags: ConfigFlags) {
        let io = ImGui_GetIO();
        (*io).ConfigFlags |= flags.bits();
    }
    pub unsafe fn remove_config_flags(&mut self, flags: ConfigFlags) {
        let io = ImGui_GetIO();
        (*io).ConfigFlags &= !flags.bits();
    }
    pub fn nav_enable_keyboard(&mut self) {
        unsafe {
            self.add_config_flags(ConfigFlags::NavEnableKeyboard);
        }
    }
    pub fn nav_enable_gamepad(&mut self) {
        unsafe {
            self.add_config_flags(ConfigFlags::NavEnableGamepad);
        }
    }
    pub unsafe fn set_size(&mut self, size: Vector2, scale: f32) {
        let io = ImGui_GetIO();
        (*io).DisplaySize = v2_to_im(size);
        if self.scale() != scale {
            (*io).DisplayFramebufferScale = ImVec2 { x: scale, y: scale };
            (*io).FontGlobalScale = scale.recip();
            self.invalidate_font_atlas();
        }
    }
    pub unsafe fn size(&self) -> Vector2 {
        let io = ImGui_GetIO();
        im_to_v2((*io).DisplaySize)
    }
    pub unsafe fn scale(&self) -> f32 {
        let io = ImGui_GetIO();
        (*io).DisplayFramebufferScale.x
    }
    /// The next time [`CurrentContext::do_frame()`] is called, it will trigger a call to
    /// [`UiBuilder::build_custom_atlas`].
    pub fn invalidate_font_atlas(&mut self) {
        self.ctx.invalidate_font_atlas();
    }
    // I like to be explicit about this particular lifetime
    #[allow(clippy::needless_lifetimes)]
    pub unsafe fn update_atlas<'ui, A: UiBuilder>(&'ui mut self, app: &mut A) -> bool {
        if !std::mem::take(&mut self.ctx.pending_atlas) {
            return false;
        }
        let io = ImGui_GetIO();
        ImFontAtlas_Clear((*io).Fonts);
        (*(*io).Fonts).TexPixelsUseColors = true;

        let scale = (*io).DisplayFramebufferScale.x;
        let mut atlas = FontAtlasMut {
            ptr: FontAtlasPtr {
                ptr: &mut *(*io).Fonts,
            },
            scale,
            glyph_ranges: Vec::new(),
            custom_rects: Vec::new(),
        };

        app.build_custom_atlas(&mut atlas);
        // If the app did not create any font, create a default one here.
        // ImGui will do it by itself, but that would not be properly scaled if scale != 1.0
        if atlas.ptr.ptr.Fonts.is_empty() {
            atlas.add_font(FontInfo::default_font(13.0));
        }
        atlas.build_custom_rects(app);
        true
    }
    /// Builds and renders a UI frame.
    ///
    /// * `app`: `UiBuilder` to be used to build the frame.
    /// * `re_render`: function to be called after `app.do_ui` but before rendering.
    /// * `render`: function to do the actual render.
    pub unsafe fn do_frame<A: UiBuilder>(
        &mut self,
        app: &mut A,
        pre_render: impl FnOnce(),
        render: impl FnOnce(&ImDrawData),
    ) {
        let mut ui = Ui {
            data: std::ptr::null_mut(),
            generation: ImGui_GetFrameCount() as usize,
            callbacks: RefCell::new(Vec::new()),
            pending_atlas: Cell::new(false),
        };

        let io = ImGui_GetIO();
        (*io).BackendLanguageUserData = &ui as *const Ui<A> as *mut c_void;
        let _guard = UiPtrToNullGuard(self.ctx);
        ImGui_NewFrame();

        app.do_ui(&ui);

        pre_render();
        app.pre_render();

        ImGui_Render();

        ui.data = app;

        // This is the same pointer, but without it, there is something fishy about stacked borrows
        // and the mutable access to `ui` above.
        (*io).BackendLanguageUserData = &ui as *const Ui<A> as *mut c_void;

        let draw_data = ImGui_GetDrawData();
        render(&*draw_data);

        _guard.0.pending_atlas |= ui.pending_atlas.get();
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            ImGui_DestroyContext(self.imgui);
        }
    }
}

struct UiPtrToNullGuard<'a>(&'a mut Context);

impl Drop for UiPtrToNullGuard<'_> {
    fn drop(&mut self) {
        unsafe {
            let io = ImGui_GetIO();
            (*io).BackendLanguageUserData = null_mut();
        }
    }
}

/// The main trait that the user must implement to create a UI.
pub trait UiBuilder {
    /// This function is run the first time an ImGui context is used to create the font atlas.
    ///
    /// You can force new call by invalidating the current atlas by calling [`Context::invalidate_font_atlas`].
    fn build_custom_atlas(&mut self, _atlas: &mut FontAtlasMut<'_, Self>) {}
    /// This function is run after `do_ui` but before rendering.
    ///
    /// It can be used to clear the framebuffer.
    fn pre_render(&mut self) {}
    /// User the `ui` value to create a UI frame.
    ///
    /// This is equivalent to the Dear ImGui code between `NewFrame` and `EndFrame`.
    fn do_ui(&mut self, ui: &Ui<Self>);
}

enum TtfData {
    Bytes(Cow<'static, [u8]>),
    DefaultFont,
}

/// A font to be fed to the ImGui atlas.
pub struct FontInfo {
    ttf: TtfData,
    size: f32,
    char_ranges: Vec<[ImWchar; 2]>,
}

impl FontInfo {
    /// Creates a new `FontInfo` from a TTF content and a font size.
    pub fn new(ttf: impl Into<Cow<'static, [u8]>>, size: f32) -> FontInfo {
        FontInfo {
            ttf: TtfData::Bytes(ttf.into()),
            size,
            char_ranges: Vec::new(),
        }
    }
    /// Creates a `FontInfo` using the embedded default Dear ImGui font, with the given font size.
    ///
    /// The default size of the default font is 13.0.
    pub fn default_font(size: f32) -> FontInfo {
        FontInfo {
            ttf: TtfData::DefaultFont,
            size,
            char_ranges: Vec::new(),
        }
    }
    /// Adds the given char range to this font info.
    ///
    /// If the range list is empty, it is as if `'\u{20}'..='\u{ff}'`, that is the "ISO-8859-1"
    /// table. But if you call this function for a font, then it will not be added by default, you
    /// should add it yourself.
    pub fn add_char_range(mut self, range: std::ops::RangeInclusive<char>) -> Self {
        self.char_ranges
            .push([ImWchar::from(*range.start()), ImWchar::from(*range.end())]);
        self
    }
}

/// Represents any type that can be converted into something that can be deref'ed to a `&CStr`.
pub trait IntoCStr {
    type Temp: Deref<Target = CStr>;
    fn into(self) -> Self::Temp;
}

impl IntoCStr for &str {
    type Temp = CString;

    fn into(self) -> Self::Temp {
        CString::new(self).unwrap()
    }
}
impl IntoCStr for &String {
    type Temp = CString;

    fn into(self) -> Self::Temp {
        CString::new(self.as_str()).unwrap()
    }
}
impl IntoCStr for String {
    type Temp = CString;

    fn into(self) -> Self::Temp {
        CString::new(self).unwrap()
    }
}
impl IntoCStr for &CStr {
    type Temp = Self;
    fn into(self) -> Self {
        self
    }
}
impl IntoCStr for CString {
    type Temp = Self;

    fn into(self) -> Self {
        self
    }
}
impl<'a> IntoCStr for &'a CString {
    type Temp = &'a CStr;

    fn into(self) -> &'a CStr {
        self.as_c_str()
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

unsafe fn font_ptr(font: FontId) -> *mut ImFont {
    let io = &*ImGui_GetIO();
    let fonts = &*io.Fonts;
    // fonts.Fonts is never empty, at least there is the default font
    fonts.Fonts[font.0]
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
    data: *mut A, // only for callbacks, after `do_ui` has finished, do not use directly
    generation: usize,
    callbacks: RefCell<Vec<UiCallback<A>>>,
    pending_atlas: Cell<bool>,
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
                let r = f();
                unsafe { $end() }
                r
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
                let r = f(true);
                unsafe { $end() }
                r
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
        pub struct $sname<$($life,)* $($gen_n : $gen_d, )* > {
            $(
                $arg: $($ty)*,
            )*
        }
        impl <$($life,)* $($gen_n : $gen_d, )* > $sname<$($life,)* $($gen_n, )* > {
            pub fn build(self) -> $tres {
                let $sname { $($arg, )* } = self;
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
                let r = f(bres);
                unsafe {
                    if $always_run_end || bres {
                        $func_end();
                    }
                }
                r
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
        pub fn child_config<S: IntoCStr>(&self, name: S) -> Child<S> {
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
        pub fn window_config<S: IntoCStr>(&self, name: S) -> Window<S> {
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
        pub fn shortcut_opt<S3: IntoCStr>(self, shortcut: Option<S3>) -> MenuItem<S1, S3> {
            MenuItem {
                label: self.label,
                shortcut: shortcut.map(|s| s.into()),
                selected: self.selected,
                enabled: self.enabled,
            }
        }
        pub fn shortcut<S3: IntoCStr>(self, shortcut: S3) -> MenuItem<S1, S3> {
            self.shortcut_opt(Some(shortcut))
        }
        decl_builder_setter!{selected: bool}
        decl_builder_setter!{enabled: bool}
    }
    {
        pub fn menu_item_config<S: IntoCStr>(&self, label: S) -> MenuItem<S, &str> {
            MenuItem {
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
        pub fn button_config<S: IntoCStr>(&self, label: S) -> Button<S> {
            Button {
                label: label.into(),
                size: im_vec2(0.0, 0.0),
            }
        }
        pub fn button<S: IntoCStr>(&self, label: S) -> bool {
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
        pub fn small_button_config<S: IntoCStr>(&self, label: S) -> SmallButton<S> {
            SmallButton {
                label: label.into(),
            }
        }
        pub fn small_button<S: IntoCStr>(&self, label: S) -> bool {
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
        pub fn invisible_button_config<S: IntoCStr>(&self, id: S) -> InvisibleButton<S> {
            InvisibleButton {
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
        pub fn arrow_button_config<S: IntoCStr>(&self, id: S, dir: Dir) -> ArrowButton<S> {
            ArrowButton {
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
        pub fn checkbox_config<'v, S: IntoCStr>(&self, label: S, value: &'v mut bool) -> Checkbox<'v, S> {
            Checkbox {
                label: label.into(),
                value,
            }
        }
        pub fn checkbox<S: IntoCStr>(&self, label: S, value: &mut bool) -> bool {
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
        pub fn radio_button_config<S: IntoCStr>(&self, label: S, active: bool) -> RadioButton<S> {
            RadioButton {
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
        pub fn overlay<S2: IntoCStr>(self, overlay: S2) -> ProgressBar<S2> {
            ProgressBar {
                fraction: self.fraction,
                size: self.size,
                overlay: Some(overlay.into()),
            }
        }
    }
    {
        pub fn progress_bar_config<'a>(&self, fraction: f32) -> ProgressBar<&'a str> {
            ProgressBar {
                fraction,
                size: im_vec2(-f32::MIN_POSITIVE, 0.0),
                overlay: None,
            }
        }
    }
}

decl_builder! { Image -> (), ImGui_Image () ()
    (
        user_texture_id (TextureId) (user_texture_id.id()),
        size (ImVec2) (&size),
        uv0 (ImVec2) (&uv0),
        uv1 (ImVec2) (&uv1),
        tint_col (ImVec4) (&tint_col),
        border_col (ImVec4) (&border_col),
    )
    {
        decl_builder_setter_vector2!{uv0: Vector2}
        decl_builder_setter_vector2!{uv1: Vector2}
        decl_builder_setter!{tint_col: Color}
        decl_builder_setter!{border_col: Color}
    }
    {
        pub fn image_config(&self, user_texture_id: TextureId, size: Vector2) -> Image {
            Image {
                user_texture_id,
                size: v2_to_im(size),
                uv0: im_vec2(0.0, 0.0),
                uv1: im_vec2(1.0, 1.0),
                tint_col: Color::WHITE.into(),
                border_col: Color::TRANSPARENT.into(),
            }
        }
        pub fn image_with_custom_rect_config(&self, ridx: CustomRectIndex, scale: f32) -> Image {
            let atlas = self.font_atlas();
            let rect = atlas.get_custom_rect(ridx);
            let tex_id = atlas.texture_id();
            let tex_size = atlas.texture_size();
            let inv_tex_w = 1.0 / tex_size[0] as f32;
            let inv_tex_h = 1.0 / tex_size[1] as f32;
            let uv0 = vec2(rect.X as f32 * inv_tex_w, rect.Y as f32 * inv_tex_h);
            let uv1 = vec2((rect.X + rect.Width) as f32 * inv_tex_w, (rect.Y + rect.Height) as f32 * inv_tex_h);

            self.image_config(tex_id, vec2(scale * rect.Width as f32, scale * rect.Height as f32))
                .uv0(uv0)
                .uv1(uv1)
        }

    }
}
decl_builder! { ImageButton -> bool, ImGui_ImageButton () (S: IntoCStr)
    (
        str_id (S::Temp) (str_id.as_ptr()),
        user_texture_id (TextureId) (user_texture_id.id()),
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
        pub fn image_button_config<S: IntoCStr>(&self, str_id: S, user_texture_id: TextureId, size: Vector2) -> ImageButton<S> {
            ImageButton {
                str_id: str_id.into(),
                user_texture_id,
                size: v2_to_im(size),
                uv0: im_vec2(0.0, 0.0),
                uv1: im_vec2(1.0, 1.0),
                bg_col: Color::TRANSPARENT.into(),
                tint_col: Color::WHITE.into(),
            }
        }
        pub fn image_button_with_custom_rect_config<S: IntoCStr>(&self, str_id: S, ridx: CustomRectIndex, scale: f32) -> ImageButton<S> {
            let atlas = self.font_atlas();
            let rect = atlas.get_custom_rect(ridx);
            let tex_id = atlas.texture_id();
            let tex_size = atlas.texture_size();
            let inv_tex_w = 1.0 / tex_size[0] as f32;
            let inv_tex_h = 1.0 / tex_size[1] as f32;
            let uv0 = vec2(rect.X as f32 * inv_tex_w, rect.Y as f32 * inv_tex_h);
            let uv1 = vec2((rect.X + rect.Width) as f32 * inv_tex_w, (rect.Y + rect.Height) as f32 * inv_tex_h);

            self.image_button_config(str_id, tex_id, vec2(scale * rect.Width as f32, scale * rect.Height as f32))
                .uv0(uv0)
                .uv1(uv1)
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
        pub fn selectable_config<S: IntoCStr>(&self, label: S) -> Selectable<S> {
            Selectable {
                label: label.into(),
                selected: false,
                flags: SelectableFlags::None,
                size: im_vec2(0.0, 0.0),
            }
        }
        pub fn selectable<S: IntoCStr>(&self, label: S) -> bool {
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
                pub fn $func<$life, S: IntoCStr>(&self, label: S, value: $ty) -> $name<$life, S> {
                    $name {
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
        impl<S: IntoCStr> $name<'_, S> {
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
                pub fn $func<$life, S: IntoCStr>(&self, label: S, value: $ty) -> $name<$life, S> {
                    $name {
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
        pub fn slider_angle_config<'v, S: IntoCStr>(&self, label: S, v_rad: &'v mut f32) -> SliderAngle<'v, S> {
            SliderAngle {
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
        pub fn color_edit_3_config<'v, S: IntoCStr>(&self, label: S, color: &'v mut [f32; 3]) -> ColorEdit3<'v, S> {
            ColorEdit3 {
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
        pub fn color_edit_4_config<'v, S: IntoCStr>(&self, label: S, color: &'v mut Color) -> ColorEdit4<'v, S> {
            ColorEdit4 {
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
        pub fn color_picker_3_config<'v, S: IntoCStr>(&self, label: S, color: &'v mut [f32; 3]) -> ColorPicker3<'v, S> {
            ColorPicker3 {
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
        pub fn color_picker_4_config<'v, S: IntoCStr>(&self, label: S, color: &'v mut Color) -> ColorPicker4<'v, S> {
            ColorPicker4 {
                label: label.into(),
                color: color.as_mut(),
                flags: ColorEditFlags::None,
                ref_col: None,
            }
        }
    }
}

unsafe extern "C" fn input_text_callback(data: *mut ImGuiInputTextCallbackData) -> i32 {
    let data = &mut *data;
    if data.EventFlag == InputTextFlags::CallbackResize.bits() {
        let this = &mut *(data.UserData as *mut String);
        let extra = (data.BufSize as usize).saturating_sub(this.len());
        this.reserve(extra);
        data.Buf = this.as_mut_ptr() as *mut c_char;
    }
    0
}

#[inline]
fn text_pre_edit(text: &mut String) {
    // Ensure a NUL at the end
    text.push('\0');
}

#[inline]
unsafe fn text_post_edit(text: &mut String) {
    let buf = text.as_mut_vec();
    // Look for the ending NUL that must be there, instead of memchr or iter::position, leverage the standard CStr
    let len = CStr::from_ptr(buf.as_ptr() as *const c_char)
        .to_bytes()
        .len();
    buf.set_len(len);
}

unsafe fn input_text_wrapper(
    label: *const c_char,
    text: &mut String,
    flags: InputTextFlags,
) -> bool {
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
        pub fn input_text_config<'v, S: IntoCStr>(&self, label: S, text: &'v mut String) -> InputText<'v, S> {
            InputText {
                label:label.into(),
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
        pub fn input_text_multiline_config<'v, S: IntoCStr>(&self, label: S, text: &'v mut String) -> InputTextMultiline<'v, S> {
            InputTextMultiline {
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
        pub fn input_text_hint_config<'v, S1: IntoCStr, S2: IntoCStr>(&self, label: S1, hint: S2, text: &'v mut String) -> InputTextHint<'v, S1, S2> {
            InputTextHint {
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
        pub fn input_float_config<'v, S: IntoCStr>(&self, label: S, value: &'v mut f32) -> InputFloat<'v, S> {
            InputFloat {
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
        pub fn input_int_config<'v, S: IntoCStr>(&self, label: S, value: &'v mut i32) -> InputInt<'v, S> {
            InputInt {
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
            pub fn $func<'v, S: IntoCStr>(&self, label: S, value: &'v mut [f32; $len]) -> $name<'v, S> {
                $name {
                    label:label.into(),
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
            pub fn $func<'v, S: IntoCStr>(&self, label: S, value: &'v mut [i32; $len]) -> $name<'v, S> {
                $name {
                    label:label.into(),
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
        pub fn menu_config<S: IntoCStr>(&self, name: S) -> Menu<S> {
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
        pub fn collapsing_header_config<S: IntoCStr>(&self, label: S) -> CollapsingHeader<S> {
            CollapsingHeader {
                label: label.into(),
                flags: TreeNodeFlags::None,
                push: (),
            }
        }
    }
}

enum LabelId<'a, S: IntoCStr, H: Hashable> {
    Label(S),
    LabelId(&'a str, H),
}

unsafe fn tree_node_ex_helper<S: IntoCStr, H: Hashable>(
    label_id: LabelId<'_, S, H>,
    flags: TreeNodeFlags,
) -> bool {
    match label_id {
        LabelId::Label(lbl) => ImGui_TreeNodeEx(lbl.into().as_ptr(), flags.bits()),
        LabelId::LabelId(lbl, id) => {
            let (start, end) = text_ptrs(lbl);
            // Warning! internal imgui API ahead, the alterative would be to call all the TreeNodeEx* functions without the Hashable generics
            ImGui_TreeNodeBehavior(id.get_id(), flags.bits(), start, end)
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
        pub fn tree_node_config<S: IntoCStr>(&self, label: S) -> TreeNode<'static, S, usize> {
            TreeNode {
                label: LabelId::Label(label),
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
        pub fn popup_config<S: IntoCStr>(&self, str_id: S) -> Popup<S> {
            Popup {
                str_id: str_id.into(),
                flags: WindowFlags::None,
                push: (),
            }
        }
    }
}

decl_builder_with_opt! {PopupModal, ImGui_BeginPopupModal, ImGui_EndPopup () (S: IntoCStr)
    (
        name (S::Temp) (name.as_ptr()),
        opened (Option<bool>) (optional_mut_bool(&mut opened.as_mut())),
        flags (WindowFlags) (flags.bits()),
    )
    {
        decl_builder_setter!{flags: WindowFlags}
        pub fn close_button(mut self, close_button: bool) -> Self {
            self.opened = if close_button { Some(true) } else { None };
            self
        }
    }
    {
        pub fn popup_modal_config<S: IntoCStr>(&self, name: S) -> PopupModal<S> {
            PopupModal {
                name: name.into(),
                opened: None,
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
                pub fn str_id<S2: IntoCStr>(self, str_id: S2) -> $struct<S2, P> {
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
        pub fn combo_config<'a, S: IntoCStr>(&self, label: S) -> Combo<S, &'a str> {
            Combo {
                label: label.into(),
                preview_value: None,
                flags: ComboFlags::None,
                push: (),
            }
        }
        // Helper function for simple use cases
        pub fn combo<V: Copy + PartialEq, S2: IntoCStr>(
            &self,
            label: impl IntoCStr,
            values: impl IntoIterator<Item=V>,
            f_name: impl Fn(V) -> S2,
            current: &mut V
        ) -> bool
        {
            let mut changed = false;
            self.combo_config(label)
                .preview_value(f_name(*current))
                .with(|| {
                    for val in values {
                        if self.selectable_config(f_name(val))
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
        pub fn list_box_config<S: IntoCStr>(&self, label: S) -> ListBox<S> {
            ListBox {
                label: label.into(),
                size: im_vec2(0.0, 0.0),
                push: (),
            }
        }
        // Helper function for simple use cases
        pub fn list_box<V: Copy + PartialEq, S2: IntoCStr>(
            &self,
            label: impl IntoCStr,
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
            let height_in_pixels = self.get_text_line_height_with_spacing() * height_in_items_f + self.style().frame_padding().y * 2.0;

            let mut changed = false;
            self.list_box_config(label)
                .size(vec2(0.0, height_in_pixels.floor()))
                .with(|| {
                    for val in values {
                        if self.selectable_config(f_name(val))
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
        std_id (S::Temp) (std_id.as_ptr()),
        flags (TabBarFlags) (flags.bits()),
    )
    {
        decl_builder_setter!{flags: TabBarFlags}
    }
    {
        pub fn tab_bar_config<S: IntoCStr>(&self, std_id: S) -> TabBar<S> {
            TabBar {
                std_id: std_id.into(),
                flags: TabBarFlags::None,
                push: (),
            }
        }
    }
}

decl_builder_with_opt! {TabItem, ImGui_BeginTabItem, ImGui_EndTabItem ('o) (S: IntoCStr)
    (
        std_id (S::Temp) (std_id.as_ptr()),
        opened (Option<&'o mut bool>) (optional_mut_bool(&mut opened)),
        flags (TabItemFlags) (flags.bits()),
    )
    {
        decl_builder_setter!{flags: TabItemFlags}
        decl_builder_setter!{opened: &'o mut bool}
    }
    {
        pub fn tab_item_config<S: IntoCStr>(&self, std_id: S) -> TabItem<S> {
            TabItem {
                std_id: std_id.into(),
                opened: None,
                flags: TabItemFlags::None,
                push: (),
            }
        }
        pub fn tab_item_button(label: impl IntoCStr, flags: TabItemFlags) -> bool {
            unsafe {
                ImGui_TabItemButton(label.into().as_ptr(), flags.bits())
            }
        }
        pub fn set_tab_item_closed(tab_or_docked_window_label: impl IntoCStr) {
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
        let io = &*ImGui_GetIO();
        if io.BackendLanguageUserData.is_null() {
            return;
        }
        // The lifetime of ui has been erased, but at least the types of A and X should be correct
        let ui = &*(io.BackendLanguageUserData as *const Self);
        let Some(id) = remove_generation(id, ui.generation) else {
            eprintln!("lost generation callback");
            return;
        };

        let mut callbacks = ui.callbacks.borrow_mut();
        let cb = &mut callbacks[id];
        // disable the destructor of x, it will be run inside the callback
        let mut x = MaybeUninit::new(x);
        cb(&mut *ui.data, x.as_mut_ptr() as *mut c_void);
    }
    /// The next time [`CurrentContext::do_frame()`] is called, it will trigger a call to
    /// [`UiBuilder::build_custom_atlas`].
    pub fn invalidate_font_atlas(&self) {
        self.pending_atlas.set(true);
    }

    pub fn display_size(&self) -> Vector2 {
        unsafe {
            let io = ImGui_GetIO();
            im_to_v2((*io).DisplaySize)
        }
    }
    pub fn display_scale(&self) -> f32 {
        unsafe {
            let io = ImGui_GetIO();
            (*io).DisplayFramebufferScale.x
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
    pub fn set_keyboard_focus_here(offset: i32) {
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
    pub fn foreground_draw_list(&self) -> WindowDrawList<'_, A> {
        unsafe {
            let ptr = ImGui_GetForegroundDrawList();
            WindowDrawList { ui: self, ptr }
        }
    }
    pub fn background_draw_list(&self) -> WindowDrawList<'_, A> {
        unsafe {
            let ptr = ImGui_GetBackgroundDrawList();
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
    pub fn get_main_viewport(&self) -> Viewport<'_> {
        unsafe {
            Viewport {
                ptr: &*ImGui_GetMainViewport(),
            }
        }
    }
    pub fn get_content_region_avail(&self) -> Vector2 {
        unsafe { im_to_v2(ImGui_GetContentRegionAvail()) }
    }
    pub fn get_content_region_max(&self) -> Vector2 {
        unsafe { im_to_v2(ImGui_GetContentRegionMax()) }
    }
    pub fn get_window_content_region_min(&self) -> Vector2 {
        unsafe { im_to_v2(ImGui_GetWindowContentRegionMin()) }
    }
    pub fn get_window_content_region_max(&self) -> Vector2 {
        unsafe { im_to_v2(ImGui_GetWindowContentRegionMax()) }
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
    pub fn get_cursor_pos(&self) -> Vector2 {
        unsafe { im_to_v2(ImGui_GetCursorPos()) }
    }
    pub fn get_cursor_pos_x(&self) -> f32 {
        unsafe { ImGui_GetCursorPosX() }
    }
    pub fn get_cursor_pos_y(&self) -> f32 {
        unsafe { ImGui_GetCursorPosY() }
    }
    pub fn set_cursor_pos(&self, local_pos: Vector2) {
        unsafe {
            ImGui_SetCursorPos(&v2_to_im(local_pos));
        }
    }
    pub fn set_cursor_pos_x(&self, local_x: f32) {
        unsafe {
            ImGui_SetCursorPosX(local_x);
        }
    }
    pub fn set_cursor_pos_y(&self, local_y: f32) {
        unsafe {
            ImGui_SetCursorPosY(local_y);
        }
    }
    pub fn get_cursor_start_pos(&self) -> Vector2 {
        unsafe { im_to_v2(ImGui_GetCursorStartPos()) }
    }
    pub fn get_cursor_screen_pos(&self) -> Vector2 {
        unsafe { im_to_v2(ImGui_GetCursorScreenPos()) }
    }
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
    pub fn is_popup_open(&self, str_id: Option<&str>) -> bool {
        self.is_popup_open_ex(str_id, PopupFlags::None)
    }
    pub fn is_popup_open_ex(&self, str_id: Option<&str>, flags: PopupFlags) -> bool {
        let temp;
        let str_id = match str_id {
            Some(s) => {
                temp = IntoCStr::into(s);
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
            let modal = ImGui_FindBlockingModal((*ImGui_GetCurrentContext()).CurrentWindow);
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
    pub fn open_popup(&self, str_id: impl IntoCStr) {
        self.open_popup_ex(str_id, PopupFlags::None)
    }
    pub fn open_popup_ex(&self, str_id: impl IntoCStr, flags: PopupFlags) {
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

    pub fn io(&self) -> &ImGuiIO {
        unsafe { &*ImGui_GetIO() }
    }
    pub fn font_atlas(&self) -> FontAtlas<'_> {
        unsafe {
            let io = &*ImGui_GetIO();
            FontAtlas {
                ptr: FontAtlasPtr {
                    ptr: &mut *io.Fonts,
                },
            }
        }
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

    pub fn with_list_clipper(
        &self,
        items_count: usize,
        items_height: f32,
        included_ranges: &[std::ops::Range<usize>],
        mut f: impl FnMut(usize),
    ) {
        unsafe {
            let mut clip = ImGuiListClipper::new();
            clip.Begin(items_count as i32, items_height);
            for r in included_ranges {
                clip.IncludeItemsByIndex(r.start as i32, r.end as i32);
            }
            while clip.Step() {
                for i in clip.DisplayStart..clip.DisplayEnd {
                    f(i as usize);
                }
            }
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

    /// Gets information about a glyph for a font.
    ///
    /// This is a member of `Ui` instead of `FontAtlas` because it requires the atlas to be fully
    /// built, and in `build_custom_atlas` it is not yet. The only way to ensure a fully built
    /// atlas is by requiring a `&Ui`.
    pub fn find_glyph(&self, font_id: FontId, c: char) -> FontGlyph<'_> {
        unsafe {
            let font = font_ptr(font_id);
            FontGlyph(&*ImFont_FindGlyph(font, ImWchar::from(c)))
        }
    }
    /// Just like `find_glyph` but doesn't use the fallback character for unavailable glyphs.
    pub fn find_glyph_no_fallback(&self, font_id: FontId, c: char) -> Option<FontGlyph<'_>> {
        unsafe {
            let font = font_ptr(font_id);
            let p = ImFont_FindGlyphNoFallback(font, ImWchar::from(c));
            p.as_ref().map(FontGlyph)
        }
    }
    /// Gets the font details for a `FontId`.
    ///
    /// TODO: do a proper ImFont wrapper?
    pub fn get_font(&self, font_id: FontId) -> &ImFont {
        unsafe {
            let font = font_ptr(font_id);
            &*font
        }
    }
}

pub struct FontGlyph<'a>(&'a ImFontGlyph);

impl FontGlyph<'_> {
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

impl std::fmt::Debug for FontGlyph<'_> {
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

#[cfg(feature = "docking")]
impl<A> Ui<A> {
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
        flags: DockNodeFlags, /*window_class: &WindowClass*/
    ) -> ImGuiID {
        unsafe {
            ImGui_DockSpaceOverViewport(
                dockspace_id,
                std::ptr::null(),
                flags.bits(),
                std::ptr::null(),
            )
        }
    }
    pub fn set_next_window_dock_id(&self, dock_id: ImGuiID, cond: Cond) {
        unsafe {
            ImGui_SetNextWindowDockID(dock_id, cond.bits());
        }
    }
    //SetNextWindowClass(const ImGuiWindowClass* window_class)
    pub fn get_window_doc_id(&self) -> ImGuiID {
        unsafe { ImGui_GetWindowDockID() }
    }
    pub fn is_window_docked(&self) -> bool {
        unsafe { ImGui_IsWindowDocked() }
    }
}

/// Identifier of a registered font. Only the values obtained from the latest call to [`UiBuilder::build_custom_atlas`] are actually valid.
///
/// `FontId::default()` wil be the default font.
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FontId(usize);

/// Identifier for a registered custom rectangle. Only the values obtained from the latest call to
/// [`UiBuilder::build_custom_atlas`] are actually valid.
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

#[derive(Debug)]
pub struct FontAtlasPtr<'ui> {
    ptr: &'ui mut ImFontAtlas,
}

type PixelImage<'a> = image::ImageBuffer<image::Rgba<u8>, &'a mut [u8]>;
type SubPixelImage<'a, 'b> = image::SubImage<&'a mut PixelImage<'b>>;

type FuncCustomRect<A> = Box<dyn FnOnce(&mut A, &mut SubPixelImage<'_, '_>)>;

pub struct FontAtlasMut<'ui, A: ?Sized> {
    ptr: FontAtlasPtr<'ui>,
    scale: f32,
    // glyph_ranges pointers have to live until the atlas texture is built
    glyph_ranges: Vec<Vec<[ImWchar; 2]>>,
    custom_rects: Vec<Option<FuncCustomRect<A>>>,
}

/// A reference to the font altas that is to be built.
///
/// You get a value of this type when implementing [`UiBuilder::build_custom_atlas`]. If you want
/// that function to be called again, call [`Context::invalidate_font_atlas`].
impl<'ui, A> FontAtlasMut<'ui, A> {
    /// Adds the given font to the atlas.
    ///
    /// It returns the id to use this font. `FontId` implements `Pushable` so you can use it with
    /// [`Ui::with_push`].
    pub fn add_font(&mut self, font: FontInfo) -> FontId {
        self.add_font_priv(font, false)
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
    fn add_font_priv(&mut self, mut font: FontInfo, merge: bool) -> FontId {
        unsafe {
            let mut fc = ImFontConfig::new();
            // This is ours, do not free()
            fc.FontDataOwnedByAtlas = false;

            fc.MergeMode = merge;

            // glyph_ranges must be valid for the duration of the atlas, so do not modify the existing self.fonts.
            // You can add new fonts however, but they will not show unless you call update_altas() again
            let glyph_ranges = if font.char_ranges.is_empty() {
                null()
            } else {
                // keep the ptr alive
                let mut char_ranges = std::mem::take(&mut font.char_ranges);
                char_ranges.push([0, 0]); // add the marking NULs
                let ptr = char_ranges[0].as_ptr();
                self.glyph_ranges.push(char_ranges);
                ptr
            };
            let io = ImGui_GetIO();
            match font.ttf {
                TtfData::Bytes(bytes) => {
                    ImFontAtlas_AddFontFromMemoryTTF(
                        (*io).Fonts,
                        bytes.as_ptr() as *mut _,
                        bytes.len() as i32,
                        font.size * self.scale,
                        &fc,
                        glyph_ranges,
                    );
                }
                TtfData::DefaultFont => {
                    fc.SizePixels = font.size * self.scale;
                    ImFontAtlas_AddFontDefault((*io).Fonts, &fc);
                }
            }
            FontId((*(*io).Fonts).Fonts.len() - 1)
        }
    }
    /// Adds an image as a substitution for a character in a font.
    pub fn add_custom_rect_font_glyph(
        &mut self,
        font: FontId,
        id: char,
        size: impl Into<mint::Vector2<u32>>,
        advance_x: f32,
        offset: Vector2,
        draw: impl FnOnce(&mut A, &mut SubPixelImage<'_, '_>) + 'static,
    ) -> CustomRectIndex {
        let size = size.into();
        unsafe {
            let io = ImGui_GetIO();

            let font = font_ptr(font);
            let idx = ImFontAtlas_AddCustomRectFontGlyph(
                (*io).Fonts,
                font,
                id as ImWchar,
                i32::try_from(size.x).unwrap(),
                i32::try_from(size.y).unwrap(),
                advance_x,
                &v2_to_im(offset),
            );
            self.add_custom_rect_at(idx as usize, Box::new(draw));
            CustomRectIndex(idx)
        }
    }
    /// Adds an arbitrary image to the font atlas.
    ///
    /// The returned `CustomRectIndex` can be used later to draw the image.
    pub fn add_custom_rect_regular(
        &mut self,
        size: impl Into<mint::Vector2<u32>>,
        draw: impl FnOnce(&mut A, &mut SubPixelImage<'_, '_>) + 'static,
    ) -> CustomRectIndex {
        let size = size.into();
        unsafe {
            let io = ImGui_GetIO();

            let idx = ImFontAtlas_AddCustomRectRegular(
                (*io).Fonts,
                i32::try_from(size.x).unwrap(),
                i32::try_from(size.y).unwrap(),
            );
            self.add_custom_rect_at(idx as usize, Box::new(draw));
            CustomRectIndex(idx)
        }
    }
    fn add_custom_rect_at(&mut self, idx: usize, f: FuncCustomRect<A>) {
        if idx >= self.custom_rects.len() {
            self.custom_rects.resize_with(idx + 1, || None);
        }
        self.custom_rects[idx] = Some(f);
    }
    unsafe fn build_custom_rects(self, app: &mut A) {
        let mut tex_data = std::ptr::null_mut();
        let mut tex_width = 0;
        let mut tex_height = 0;
        let mut pixel_size = 0;
        let io;
        unsafe {
            io = ImGui_GetIO();
            ImFontAtlas_GetTexDataAsRGBA32(
                (*io).Fonts,
                &mut tex_data,
                &mut tex_width,
                &mut tex_height,
                &mut pixel_size,
            );
        }
        let pixel_size = pixel_size as usize;
        assert!(pixel_size == 4);
        let tex_data = unsafe {
            std::slice::from_raw_parts_mut(
                tex_data,
                tex_width as usize * tex_height as usize * pixel_size,
            )
        };
        let mut pixel_image =
            PixelImage::from_raw(tex_width as u32, tex_height as u32, tex_data).unwrap();

        for (idx, f) in self.custom_rects.into_iter().enumerate() {
            if let Some(f) = f {
                unsafe {
                    let rect = &(*(*io).Fonts).CustomRects[idx];
                    let mut sub_image = pixel_image.sub_image(
                        rect.X as u32,
                        rect.Y as u32,
                        rect.Width as u32,
                        rect.Height as u32,
                    );
                    f(app, &mut sub_image);
                }
            }
        }
    }
}

impl<'ui, A> Deref for FontAtlasMut<'ui, A> {
    type Target = FontAtlasPtr<'ui>;
    fn deref(&self) -> &FontAtlasPtr<'ui> {
        &self.ptr
    }
}

pub struct FontAtlas<'ui> {
    ptr: FontAtlasPtr<'ui>,
}

impl<'ui> Deref for FontAtlas<'ui> {
    type Target = FontAtlasPtr<'ui>;
    fn deref(&self) -> &FontAtlasPtr<'ui> {
        &self.ptr
    }
}

impl FontAtlasPtr<'_> {
    pub fn texture_id(&self) -> TextureId {
        unsafe { TextureId::from_id(self.ptr.TexID) }
    }
    pub fn texture_size(&self) -> [i32; 2] {
        [self.ptr.TexWidth, self.ptr.TexHeight]
    }
    pub fn get_custom_rect(&self, index: CustomRectIndex) -> ImFontAtlasCustomRect {
        self.ptr.CustomRects[index.0 as usize]
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
    let ptr = &mut *ptr;
    let id = ptr.UserData as usize;
    let data = SizeCallbackData { ptr };
    Ui::<A>::run_callback(id, data);
}

pub struct WindowDrawList<'ui, A> {
    ui: &'ui Ui<A>,
    ptr: *mut ImDrawList,
}

impl<'ui, A> WindowDrawList<'ui, A> {
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
                font_ptr(font),
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
        user_texture_id: TextureId,
        p_min: Vector2,
        p_max: Vector2,
        uv_min: Vector2,
        uv_max: Vector2,
        color: Color,
    ) {
        unsafe {
            ImDrawList_AddImage(
                self.ptr,
                user_texture_id.id(),
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
        user_texture_id: TextureId,
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
                user_texture_id.id(),
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
        user_texture_id: TextureId,
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
                user_texture_id.id(),
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
    _parent_lilst: *const ImDrawList,
    cmd: *const ImDrawCmd,
) {
    let id = (*cmd).UserCallbackData as usize;
    Ui::<A>::run_callback(id, ());
}

/// Represents any type that can be converted to a Dear ImGui hash id.
pub trait Hashable {
    // These are unsafe because they should be called only inside a frame (holding a &mut Ui)
    unsafe fn get_id(&self) -> ImGuiID;
    unsafe fn push(&self);
}

impl Hashable for &str {
    unsafe fn get_id(&self) -> ImGuiID {
        let (start, end) = text_ptrs(self);
        ImGui_GetID1(start, end)
    }
    unsafe fn push(&self) {
        let (start, end) = text_ptrs(self);
        ImGui_PushID1(start, end);
    }
}

impl Hashable for usize {
    unsafe fn get_id(&self) -> ImGuiID {
        ImGui_GetID2(*self as *const c_void)
    }
    unsafe fn push(&self) {
        ImGui_PushID2(*self as *const c_void);
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
    p.push();
    PushableGuard(p)
}

/// A [`Pushable`] that does nothing.
impl Pushable for () {
    unsafe fn push(&self) {}
    unsafe fn pop(&self) {}
}

impl<A: Pushable> Pushable for (A,) {
    unsafe fn push(&self) {
        self.0.push();
    }
    unsafe fn pop(&self) {
        self.0.pop();
    }
}

impl<A: Pushable, B: Pushable> Pushable for (A, B) {
    unsafe fn push(&self) {
        self.0.push();
        self.1.push();
    }
    unsafe fn pop(&self) {
        self.1.pop();
        self.0.pop();
    }
}

impl<A: Pushable, B: Pushable, C: Pushable> Pushable for (A, B, C) {
    unsafe fn push(&self) {
        self.0.push();
        self.1.push();
        self.2.push();
    }
    unsafe fn pop(&self) {
        self.2.pop();
        self.1.pop();
        self.0.pop();
    }
}

impl<A: Pushable, B: Pushable, C: Pushable, D: Pushable> Pushable for (A, B, C, D) {
    unsafe fn push(&self) {
        self.0.push();
        self.1.push();
        self.2.push();
        self.3.push();
    }
    unsafe fn pop(&self) {
        self.3.pop();
        self.2.pop();
        self.1.pop();
        self.0.pop();
    }
}

impl Pushable for &[&dyn Pushable] {
    unsafe fn push(&self) {
        for st in *self {
            st.push();
        }
    }
    unsafe fn pop(&self) {
        for st in self.iter().rev() {
            st.pop();
        }
    }
}

/// A [`Pushable`] that is applied optionally.
impl<T: Pushable> Pushable for Option<T> {
    unsafe fn push(&self) {
        if let Some(s) = self {
            s.push();
        }
    }
    unsafe fn pop(&self) {
        if let Some(s) = self {
            s.pop();
        }
    }
}

impl Pushable for FontId {
    unsafe fn push(&self) {
        ImGui_PushFont(font_ptr(*self));
    }
    unsafe fn pop(&self) {
        ImGui_PopFont();
    }
}

pub type StyleColor = (ColorId, Color);

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

impl Pushable for StyleColor {
    unsafe fn push(&self) {
        ImGui_PushStyleColor1(self.0.bits(), &self.1.into());
    }
    unsafe fn pop(&self) {
        ImGui_PopStyleColor(1);
    }
}

impl Pushable for [StyleColor] {
    unsafe fn push(&self) {
        for sc in self {
            sc.push();
        }
    }
    unsafe fn pop(&self) {
        ImGui_PopStyleColor(self.len() as i32);
    }
}

impl<const N: usize> Pushable for [StyleColor; N] {
    unsafe fn push(&self) {
        self.as_slice().push();
    }
    unsafe fn pop(&self) {
        self.as_slice().pop();
    }
}

pub type StyleColorF = (ColorId, ImVec4);

impl Pushable for StyleColorF {
    unsafe fn push(&self) {
        ImGui_PushStyleColor1(self.0.bits(), &self.1);
    }
    unsafe fn pop(&self) {
        ImGui_PopStyleColor(1);
    }
}

impl Pushable for [StyleColorF] {
    unsafe fn push(&self) {
        for sc in self {
            sc.push();
        }
    }
    unsafe fn pop(&self) {
        ImGui_PopStyleColor(self.len() as i32);
    }
}

impl<const N: usize> Pushable for [StyleColorF; N] {
    unsafe fn push(&self) {
        self.as_slice().push();
    }
    unsafe fn pop(&self) {
        self.as_slice().pop();
    }
}

#[derive(Debug, Copy, Clone)]
pub enum StyleValue {
    F32(f32),
    Vec2(Vector2),
}

pub type Style = (StyleVar, StyleValue);

impl Pushable for Style {
    unsafe fn push(&self) {
        match self.1 {
            StyleValue::F32(f) => ImGui_PushStyleVar(self.0.bits(), f),
            StyleValue::Vec2(v) => ImGui_PushStyleVar1(self.0.bits(), &v2_to_im(v)),
        }
    }
    unsafe fn pop(&self) {
        ImGui_PopStyleVar(1);
    }
}

impl Pushable for [Style] {
    unsafe fn push(&self) {
        for sc in self {
            sc.push();
        }
    }
    unsafe fn pop(&self) {
        ImGui_PopStyleVar(self.len() as i32);
    }
}

impl<const N: usize> Pushable for [Style; N] {
    unsafe fn push(&self) {
        self.as_slice().push();
    }
    unsafe fn pop(&self) {
        self.as_slice().pop();
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ItemWidth(pub f32);

impl Pushable for ItemWidth {
    unsafe fn push(&self) {
        ImGui_PushItemWidth(self.0);
    }
    unsafe fn pop(&self) {
        ImGui_PopItemWidth();
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Indent(pub f32);

impl Pushable for Indent {
    unsafe fn push(&self) {
        ImGui_Indent(self.0);
    }
    unsafe fn pop(&self) {
        ImGui_Unindent(self.0);
    }
}

#[derive(Debug, Copy, Clone)]
pub struct TextWrapPos(pub f32);

impl Pushable for TextWrapPos {
    unsafe fn push(&self) {
        ImGui_PushTextWrapPos(self.0);
    }
    unsafe fn pop(&self) {
        ImGui_PopTextWrapPos();
    }
}

#[derive(Debug, Copy, Clone)]
pub struct TabStop(pub bool);

impl Pushable for TabStop {
    unsafe fn push(&self) {
        ImGui_PushTabStop(self.0);
    }
    unsafe fn pop(&self) {
        ImGui_PopTabStop();
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ButtonRepeat(pub bool);

impl Pushable for ButtonRepeat {
    unsafe fn push(&self) {
        ImGui_PushButtonRepeat(self.0);
    }
    unsafe fn pop(&self) {
        ImGui_PopButtonRepeat();
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ItemId<H: Hashable>(pub H);

impl<H: Hashable> Pushable for ItemId<H> {
    unsafe fn push(&self) {
        self.0.push();
    }
    unsafe fn pop(&self) {
        ImGui_PopID();
    }
}

pub struct Viewport<'s> {
    ptr: &'s ImGuiViewport,
}

impl Viewport<'_> {
    pub fn flags(&self) -> ViewportFlags {
        ViewportFlags::from_bits_truncate(self.ptr.Flags)
    }
    pub fn pos(&self) -> Vector2 {
        im_to_v2(self.ptr.Pos)
    }
    pub fn size(&self) -> Vector2 {
        im_to_v2(self.ptr.Size)
    }
    pub fn work_pos(&self) -> Vector2 {
        im_to_v2(self.ptr.WorkPos)
    }
    pub fn work_size(&self) -> Vector2 {
        im_to_v2(self.ptr.WorkSize)
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
        pub fn table_config<S: IntoCStr>(&self, str_id: S, column: i32) -> TableConfig<S> {
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
        //TODO: ImGui_TableGetSortSpecs, TableGetColumnName
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

impl<'a> DragDropPayloadSetter<'a> {
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

impl<'a> DragDropPayload<'a> {
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
        self.0 .0
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
        Key::from_bits(ImGuiKey(key)).unwrap()
    }
    pub fn mods(&self) -> KeyMod {
        let mods = self.bits() & ImGuiKey::ImGuiMod_Mask_.0;
        KeyMod::from_bits(mods).unwrap()
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
