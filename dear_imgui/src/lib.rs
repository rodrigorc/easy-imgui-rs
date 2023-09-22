use std::ffi::{CString, c_char, CStr, c_void};
use std::ops::Deref;
use std::ptr::{null, null_mut};
use std::mem::MaybeUninit;
use std::cell::UnsafeCell;
use std::borrow::Cow;
use cstr::cstr;
use dear_imgui_sys::*;

mod enums;
mod style;

pub use enums::*;
pub use dear_imgui_sys::{Vector2, Color};

struct BackendData {
    generation: usize,
    ui_ptr: *mut c_void,
}

impl Default for BackendData {
    fn default() -> Self {
        BackendData {
            generation: 0,
            ui_ptr: null_mut(),
        }
    }
}

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
        eprintln!("lost generation callback");
        None
    } else {
        Some(id & GEN_ID_MASK)
    }
}

pub struct Context {
    imgui: *mut ImGuiContext,
    backend: Box<UnsafeCell<BackendData>>,
    pending_atlas: bool,
}

impl Context {
    pub unsafe fn new() -> Context {
        let backend = Box::new(UnsafeCell::new(BackendData::default()));
        let imgui = unsafe {
            let imgui = ImGui_CreateContext(null_mut());
            ImGui_SetCurrentContext(imgui);

            let io = &mut *ImGui_GetIO();
            io.BackendLanguageUserData = backend.get() as *mut c_void;
            io.IniFilename = null();
            //ImGui_StyleColorsDark(null_mut());
            imgui
        };
        Context {
            imgui,
            backend,
            pending_atlas: true,
        }
    }
    pub fn set_allow_user_scaling(&mut self, val: bool) {
        unsafe {
            let io = &mut *ImGui_GetIO();
            io.FontAllowUserScaling = val;
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
    // This is unsafe because you could break thing setting weird flags
    // TODO safe wrappers for known flags
    pub unsafe fn add_config_flags(&mut self, flags: ConfigFlags) {
        unsafe {
            let io = &mut *ImGui_GetIO();
            io.ConfigFlags |= flags.bits();
        }
    }
    pub unsafe fn set_current(&mut self) {
        ImGui_SetCurrentContext(self.imgui);
    }
    pub unsafe fn set_size(&mut self, size: impl Into<Vector2>, scale: f32) {
        let io = &mut *ImGui_GetIO();
        io.DisplaySize = size.into().into();
        io.DisplayFramebufferScale = ImVec2 { x: scale, y: scale };
        io.FontGlobalScale = scale.recip();
        self.invalidate_font_atlas();
    }
    pub fn invalidate_font_atlas(&mut self) {
        self.pending_atlas = true;
    }
    pub unsafe fn update_atlas<'a, 'ctx>(&'a mut self) -> Option<FontAtlasMut<'a, 'ctx>> {
        if !std::mem::take(&mut self.pending_atlas) {
            return None;
        }
        let io = &mut *ImGui_GetIO();
        ImFontAtlas_Clear(io.Fonts);
        (*io.Fonts).TexPixelsUseColors = true;

        let scale = io.DisplayFramebufferScale.x;
        Some(FontAtlasMut {
            ptr: FontAtlasPtr { ptr: &mut *io.Fonts },
            scale,
            glyph_ranges: Vec::new(),
            custom_rects: Vec::new(),
        })
    }
    pub unsafe fn do_frame<'ctx, A: UiBuilder>(
        &'ctx mut self,
        data: &'ctx mut A::Data,
        app: &mut A,
        do_render: impl FnOnce(&mut A),
    )
    {
        let ref mut gen = self.backend.get_mut().generation;
        *gen = gen.wrapping_add(1);

        let mut ui = UnsafeCell::new(Ui {
            data,
            generation: *gen,
            callbacks: Vec::new(),
        });


        // Not sure if this is totally sound, C callbacks will cast this pointer back to a
        // mutable reference, but those callbacks cannot see this "ui" any other way.
        self.backend.get_mut().ui_ptr = &mut ui as *mut _ as *mut c_void;
        let _guard = UiPtrToNullGuard(self);

        ImGui_NewFrame();
        app.do_ui(ui.get_mut());
        ImGui_Render();
        do_render(app);
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
        self.0.backend.get_mut().ui_ptr = null_mut();

        // change the generation to avoid trying to call stale callbacks
        let ref mut gen = self.0.backend.get_mut().generation;
        *gen = gen.wrapping_add(1);
    }
}

pub trait UiBuilder {
    type Data;
    fn do_ui(&mut self, ui: &mut Ui<Self::Data>);
    fn do_custom_atlas<'ctx>(&'ctx mut self, _atlas: &mut FontAtlasMut<'ctx, '_>) {}
}

enum TtfData {
    Bytes(Cow<'static, [u8]>),
    DefaultFont,
}
pub struct FontInfo {
    ttf: TtfData,
    size: f32,
    char_ranges: Vec<[ImWchar; 2]>,
}

impl FontInfo {
    pub fn new(ttf: impl Into<Cow<'static, [u8]>>, size: f32) -> FontInfo {
        FontInfo {
            ttf: TtfData::Bytes(ttf.into()),
            size,
            char_ranges: Vec::new(),
        }
    }
    pub fn default_font(size: f32) -> FontInfo {
        FontInfo {
            ttf: TtfData::DefaultFont,
            size,
            char_ranges: Vec::new(),
        }
    }
    pub fn char_range(mut self, char_from: ImWchar, char_to: ImWchar) -> Self {
        self.char_ranges.push([char_from, char_to]);
        self
    }
}

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

// Helper functions

// Take care to not consume the argument before using the pointer
fn optional_str<S: Deref<Target = CStr>>(t: &Option<S>) -> *const c_char {
    t.as_ref().map(|s| s.as_ptr()).unwrap_or(null())
}

fn optional_mut_bool(b: &mut Option<&mut bool>) -> *mut bool {
    b.as_mut().map(|x| *x as *mut bool).unwrap_or(null_mut())
}

pub unsafe fn text_ptrs(text: &str) -> (*const c_char, *const c_char) {
    let btxt = text.as_bytes();
    let start = btxt.as_ptr() as *const c_char;
    let end = unsafe { start.add(btxt.len()) };
    ( start, end )

}
pub unsafe fn font_ptr(font: FontId) -> *mut ImFont {
    let io = &*ImGui_GetIO();
    let fonts = &*io.Fonts;
    // If there is no fonts, create the default here
    if fonts.Fonts.is_empty() {
        ImFontAtlas_AddFontDefault(io.Fonts, null_mut());
    }
    fonts.Fonts[font.0]
}

unsafe fn no_op() {}

pub struct Ui<'ctx, D> {
    data: &'ctx mut D,
    generation: usize,
    callbacks: Vec<Box<dyn FnMut(&'ctx mut D, *mut c_void) + 'ctx>>,
}

macro_rules! with_begin_end {
    ( $name:ident $begin:ident $end:ident ($($arg:ident ($($type:tt)*) ($pass:expr),)*) ) => {
        pub fn $name<R>(&mut self, $($arg: $($type)*,)* f: impl FnOnce(&mut Self) -> R) -> R {
            unsafe { $begin( $( $pass, )* ) }
            let r = f(self);
            unsafe { $end() }
            r
        }
    };
}

macro_rules! with_begin_end_opt {
    ( $name:ident $begin:ident $end:ident ($($arg:ident ($($type:tt)*) ($pass:expr),)*) ) => {
        pub fn $name<R>(&mut self, $($arg: $($type)*,)* f: impl FnOnce(&mut Self) -> R) -> Option<R> {
            if !unsafe { $begin( $( $pass, )* ) } {
                return None;
            }
            let r = f(self);
            unsafe { $end() }
            Some(r)
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
        pub struct $sname<$($life,)* U, $($gen_n : $gen_d, )* > {
            u: U,
            $(
                $arg: $($ty)*,
            )*
        }
        impl <$($life,)* U, $($gen_n : $gen_d, )* > $sname<$($life,)* U, $($gen_n, )* > {
            pub fn build(self) -> $tres {
                let $sname { u: _u, $($arg, )* } = self;
                unsafe {
                    $func($($pass,)*)
                }
            }
            $($extra)*
        }

        impl<'ctx, D: 'ctx> Ui<'ctx, D> {
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
    ($name:ident: $ty:ty) => { decl_builder_setter_ex!{ $name: $ty = $name.into() } };
}

macro_rules! decl_builder_setter_into {
    ($name:ident: $ty:ty) => { decl_builder_setter_ex!{ $name: impl Into<$ty> = $name.into().into() } };
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
        pub struct $sname< $($life,)* U, $($gen_n : $gen_d, )* P: Pushable = () > {
            u: U,
            $(
                $arg: $($ty)*,
            )*
            push: P,
        }
        impl <$($life,)* U, $($gen_n : $gen_d, )* P: Pushable > $sname<$($life,)* U, $($gen_n,)* P > {
            pub fn push_for_begin<P2: Pushable>(self, push: P2) -> $sname< $($life,)* U, $($gen_n,)* (P, P2) > {
                $sname {
                    u: self.u,
                    $(
                        $arg: self.$arg,
                    )*
                    push: (self.push, push),
                }
            }
            pub fn with<R>(self, f: impl FnOnce(U) -> R) -> Option<R> {
                self.with_always(move |ui, opened| { opened.then(|| f(ui)) })
            }
            pub fn with_always<R>(self, f: impl FnOnce(U, bool) -> R) -> R {
                #[allow(unused_mut)]
                let $sname { u, $(mut $arg, )* push } = self;
                let bres;
                unsafe {
                    push.push();
                    bres = $func_beg($($pass,)*);
                    push.pop();
                };
                let r = f(u, bres);
                unsafe {
                    if $always_run_end || bres {
                        $func_end();
                    }
                }
                r
            }
            $($extra)*
        }

        impl<'ctx, D: 'ctx> Ui<'ctx, D> {
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

decl_builder_with!{Child, ImGui_BeginChild, ImGui_EndChild () (S: IntoCStr)
    (
        name (S::Temp) (name.as_ptr()),
        size (ImVec2) (&size),
        border (bool) (border),
        flags (WindowFlags) (flags.bits()),
    )
    {
        decl_builder_setter_into!{size: Vector2}
        decl_builder_setter!{border: bool}
        decl_builder_setter!{flags: WindowFlags}
    }
    {
        pub fn do_child<S: IntoCStr>(&mut self, name: S) -> Child<&mut Self, S> {
            Child {
                u: self,
                name: name.into(),
                size: ImVec2::zero(),
                border: false,
                flags: WindowFlags::None,
                push: (),
            }
        }
    }
}

decl_builder_with!{Window, ImGui_Begin, ImGui_End ('v) (S: IntoCStr)
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
        pub fn do_window<S: IntoCStr>(&mut self, name: S) -> Window<&mut Self, S> {
            Window {
                u: self,
                name: name.into(),
                open: None,
                flags: WindowFlags::None,
                push: (),
            }
        }
    }
}

decl_builder!{ MenuItem -> bool, ImGui_MenuItem () (S1: IntoCStr, S2: IntoCStr)
    (
        label (S1::Temp) (label.as_ptr()),
        shortcut (Option<S2::Temp>) (optional_str(&shortcut)),
        selected (bool) (selected),
        enabled (bool) (enabled),
    )
    {
        pub fn shortcut_opt<S3: IntoCStr>(self, shortcut: Option<S3>) -> MenuItem<U, S1, S3> {
            MenuItem {
                u: self.u,
                label: self.label,
                shortcut: shortcut.map(|s| s.into()),
                selected: self.selected,
                enabled: self.enabled,
            }
        }
        pub fn shortcut<S3: IntoCStr>(self, shortcut: S3) -> MenuItem<U, S1, S3> {
            self.shortcut_opt(Some(shortcut))
        }
        decl_builder_setter!{selected: bool}
        decl_builder_setter!{enabled: bool}
    }
    {
        pub fn do_menu_item<S: IntoCStr>(&mut self, label: S) -> MenuItem<&mut Self, S, &str> {
            MenuItem {
                u: self,
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
        decl_builder_setter_into!{size: Vector2}
    }
    {
        pub fn do_button<S: IntoCStr>(&mut self, label: S) -> Button<&mut Self, S> {
            Button {
                u: self,
                label: label.into(),
                size: ImVec2::zero(),
            }
        }
    }
}

decl_builder! { SmallButton -> bool, ImGui_SmallButton () (S: IntoCStr)
    (
        label (S::Temp) (label.as_ptr()),
    )
    {}
    {
        pub fn do_small_button<S: IntoCStr>(&mut self, label: S) -> SmallButton<&mut Self, S> {
            SmallButton {
                u: self,
                label: label.into(),
            }
        }
        pub fn small_button<S: IntoCStr>(&mut self, label: S) -> bool {
            self.do_small_button(label).build()
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
        decl_builder_setter_into!{size: Vector2}
        decl_builder_setter!{flags: ButtonFlags}
    }
    {
        pub fn do_invisible_button<S: IntoCStr>(&mut self, id: S) -> InvisibleButton<&mut Self, S> {
            InvisibleButton {
                u: self,
                id: id.into(),
                size: ImVec2::zero(),
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
        pub fn do_arrow_button<S: IntoCStr>(&mut self, id: S, dir: Dir) -> ArrowButton<&mut Self, S> {
            ArrowButton {
                u: self,
                id: id.into(),
                dir,
            }
        }
        pub fn arrow_button<S: IntoCStr>(&mut self, id: S, dir: Dir) -> bool {
            self.do_arrow_button(id, dir).build()
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
        pub fn do_checkbox<'v, S: IntoCStr>(&mut self, label: S, value: &'v mut bool) -> Checkbox<'v, &mut Self, S> {
            Checkbox {
                u: self,
                label: label.into(),
                value,
            }
        }
        pub fn checkbox<'v, S: IntoCStr>(&mut self, label: S, value: &'v mut bool) -> bool {
            self.do_checkbox(label, value).build()
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
        pub fn do_radio_button<S: IntoCStr>(&mut self, label: S, active: bool) -> RadioButton<&mut Self, S> {
            RadioButton {
                u: self,
                label: label.into(),
                active,
            }
        }
    }
}

//ProgressBar

decl_builder! { Image -> (), ImGui_Image () ()
    (
        user_texture_id (ImTextureID) (user_texture_id),
        size (ImVec2) (&size),
        uv0 (ImVec2) (&uv0),
        uv1 (ImVec2) (&uv1),
        tint_col (ImVec4) (&tint_col),
        border_col (ImVec4) (&border_col),
    )
    {
        decl_builder_setter_into!{uv0: Vector2}
        decl_builder_setter_into!{uv1: Vector2}
        decl_builder_setter_into!{tint_col: Color}
        decl_builder_setter_into!{border_col: Color}
    }
    {
        pub fn do_image(&mut self, user_texture_id: ImTextureID, size: impl Into<Vector2>) -> Image<&mut Self> {
            Image {
                u: self,
                user_texture_id,
                size: size.into().into(),
                uv0: ImVec2::new(0.0, 0.0),
                uv1: ImVec2::new(1.0, 1.0),
                tint_col: Color::from([1.0, 1.0, 1.0, 1.0]).into(),
                border_col: Color::from([0.0, 0.0, 0.0, 0.0]).into(),
            }
        }
    }
}
decl_builder! { ImageButton -> bool, ImGui_ImageButton () (S: IntoCStr)
    (
        str_id (S::Temp) (str_id.as_ptr()),
        user_texture_id (ImTextureID) (user_texture_id),
        size (ImVec2) (&size),
        uv0 (ImVec2) (&uv0),
        uv1 (ImVec2) (&uv1),
        bg_col (ImVec4) (&bg_col),
        tint_col (ImVec4) (&tint_col),
    )
    {
        decl_builder_setter_into!{uv0: Vector2}
        decl_builder_setter_into!{uv1: Vector2}
        decl_builder_setter_into!{bg_col: Color}
        decl_builder_setter_into!{tint_col: Color}
    }
    {
        pub fn do_image_button<S: IntoCStr>(&mut self, str_id: S, user_texture_id: ImTextureID, size: impl Into<Vector2>) -> ImageButton<&mut Self, S> {
            ImageButton {
                u: self,
                str_id: str_id.into(),
                user_texture_id,
                size: size.into().into(),
                uv0: ImVec2::new(0.0, 0.0),
                uv1: ImVec2::new(1.0, 1.0),
                bg_col: Color::from([0.0, 0.0, 0.0, 0.0]).into(),
                tint_col: Color::from([1.0, 1.0, 1.0, 1.0]).into(),
            }
        }
        pub fn do_image_button_with_custom_rect<S: IntoCStr>(&mut self, str_id: S, ridx: CustomRectIndex, scale: f32) -> ImageButton<&mut Self, S> {
            let atlas = self.font_atlas();
            let rect = atlas.get_custom_rect(ridx);
            let tex_id = atlas.texture_id();
            let tex_size = atlas.texture_size();
            let inv_tex_w = 1.0 / tex_size[0] as f32;
            let inv_tex_h = 1.0 / tex_size[1] as f32;
            let uv0 = [rect.X as f32 * inv_tex_w, rect.Y as f32 * inv_tex_h];
            let uv1 = [(rect.X + rect.Width) as f32 * inv_tex_w, (rect.Y + rect.Height) as f32 * inv_tex_h];

            self.do_image_button(str_id, tex_id, [scale * rect.Width as f32, scale * rect.Height as f32])
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
        decl_builder_setter_into!{size: Vector2}
    }
    {
        pub fn do_selectable<S: IntoCStr>(&mut self, label: S) -> Selectable<&mut Self, S> {
            Selectable {
                u: self,
                label: label.into(),
                selected: false,
                flags: SelectableFlags::None,
                size: ImVec2::zero(),
            }
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
                pub fn $func<$life, S: IntoCStr>(&mut self, label: S, value: $ty) -> $name<$life, &mut Self, S> {
                    $name {
                        u: self,
                        label: label.into(),
                        value,
                        speed: 1.0,
                        min: <$argty>::default(),
                        max: <$argty>::default(),
                        format: Cow::Borrowed(cstr!("%.3f")),
                        flags: SliderFlags::None,
                    }
                }
            }
        }
    };
}

macro_rules! impl_float_format {
    ($name:ident) => {
        impl<U, S: IntoCStr> $name<'_, U, S> {
            pub fn display_format(mut self, format: FloatFormat) -> Self {
                self.format = match format {
                    FloatFormat::G => Cow::Borrowed(cstr!("%g")),
                    FloatFormat::F(0) => Cow::Borrowed(cstr!("%.0f")),
                    FloatFormat::F(3) => Cow::Borrowed(cstr!("%.3f")),
                    FloatFormat::F(n) => Cow::Owned(CString::new(format!("%.{n}f")).unwrap()),
                };
                self
            }
        }
    }
}

decl_builder_drag!{ DragFloat do_drag_float ImGui_DragFloat 'v (f32) (&'v mut f32) (std::convert::identity)}
decl_builder_drag!{ DragFloat2 do_drag_float_2 ImGui_DragFloat2 'v (f32) (&'v mut [f32; 2]) (<[f32]>::as_mut_ptr)}
decl_builder_drag!{ DragFloat3 do_drag_float_3 ImGui_DragFloat3 'v (f32) (&'v mut [f32; 3]) (<[f32]>::as_mut_ptr)}
decl_builder_drag!{ DragFloat4 do_drag_float_4 ImGui_DragFloat4 'v (f32) (&'v mut [f32; 4]) (<[f32]>::as_mut_ptr)}

impl_float_format!{ DragFloat }
impl_float_format!{ DragFloat2 }
impl_float_format!{ DragFloat3 }
impl_float_format!{ DragFloat4 }

decl_builder_drag!{ DragInt do_drag_int ImGui_DragInt 'v (i32) (&'v mut i32) (std::convert::identity)}
decl_builder_drag!{ DragInt2 do_drag_int_2 ImGui_DragInt2 'v (i32) (&'v mut [i32; 2]) (<[i32]>::as_mut_ptr)}
decl_builder_drag!{ DragInt3 do_drag_int_3 ImGui_DragInt3 'v (i32) (&'v mut [i32; 3]) (<[i32]>::as_mut_ptr)}
decl_builder_drag!{ DragInt4 do_drag_int_4 ImGui_DragInt4 'v (i32) (&'v mut [i32; 4]) (<[i32]>::as_mut_ptr)}

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
                pub fn $func<$life, S: IntoCStr>(&mut self, label: S, value: $ty) -> $name<$life, &mut Self, S> {
                    $name {
                        u: self,
                        label: label.into(),
                        value,
                        min: <$argty>::default(),
                        max: <$argty>::default(),
                        format: Cow::Borrowed(cstr!("%.3f")),
                        flags: SliderFlags::None,
                    }
                }
            }
        }
    };
}

decl_builder_slider!{ SliderFloat do_slider_float ImGui_SliderFloat 'v (f32) (&'v mut f32) (std::convert::identity)}
decl_builder_slider!{ SliderFloat2 do_slider_float_2 ImGui_SliderFloat2 'v (f32) (&'v mut [f32; 2]) (<[f32]>::as_mut_ptr)}
decl_builder_slider!{ SliderFloat3 do_slider_float_3 ImGui_SliderFloat3 'v (f32) (&'v mut [f32; 3]) (<[f32]>::as_mut_ptr)}
decl_builder_slider!{ SliderFloat4 do_slider_float_4 ImGui_SliderFloat4 'v (f32) (&'v mut [f32; 4]) (<[f32]>::as_mut_ptr)}

impl_float_format!{ SliderFloat }
impl_float_format!{ SliderFloat2 }
impl_float_format!{ SliderFloat3 }
impl_float_format!{ SliderFloat4 }

decl_builder_slider!{ SliderInt do_slider_int ImGui_SliderInt 'v (i32) (&'v mut i32) (std::convert::identity)}
decl_builder_slider!{ SliderInt2 do_slider_int_2 ImGui_SliderInt2 'v (i32) (&'v mut [i32; 2]) (<[i32]>::as_mut_ptr)}
decl_builder_slider!{ SliderInt3 do_slider_int_3 ImGui_SliderInt3 'v (i32) (&'v mut [i32; 3]) (<[i32]>::as_mut_ptr)}
decl_builder_slider!{ SliderInt4 do_slider_int_4 ImGui_SliderInt4 'v (i32) (&'v mut [i32; 4]) (<[i32]>::as_mut_ptr)}

unsafe extern "C" fn input_text_callback(data: *mut ImGuiInputTextCallbackData) -> i32 {
    let data = &mut *data;
    if data.EventFlag  == InputTextFlags::CallbackResize.bits() {
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
    let len = CStr::from_ptr(buf.as_ptr() as *const c_char).to_bytes().len();
    buf.set_len(len);
}

unsafe fn input_text_wrapper(label: *const c_char, text: &mut String, flags: InputTextFlags) -> bool {
    let flags = flags | InputTextFlags::CallbackResize;

    text_pre_edit(text);
    let r = ImGui_InputText(
        label,
        text.as_mut_ptr() as *mut c_char,
        text.capacity(),
        flags.bits(),
        Some(input_text_callback),
        text as *mut String as *mut c_void
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
        pub fn do_input_text<'v, S: IntoCStr>(&mut self, label: S, text: &'v mut String) -> InputText<'v, &mut Self, S> {
            InputText {
                u:self,
                label:label.into(),
                text,
                flags: InputTextFlags::None,
            }
        }
    }
}

unsafe fn input_text_multiline_wrapper(label: *const c_char, text: &mut String, size: &ImVec2, flags: InputTextFlags) -> bool {
    let flags = flags | InputTextFlags::CallbackResize;
    text_pre_edit(text);
    let r = ImGui_InputTextMultiline(
        label,
        text.as_mut_ptr() as *mut c_char,
        text.capacity(),
        size,
        flags.bits(),
        Some(input_text_callback),
        text as *mut String as *mut c_void
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
        decl_builder_setter_into!{size: Vector2}
    }
    {
        pub fn do_input_text_multiline<'v, S: IntoCStr>(&mut self, label: S, text: &'v mut String) -> InputTextMultiline<'v, &mut Self, S> {
            InputTextMultiline {
                u:self,
                label:label.into(),
                text,
                flags: InputTextFlags::None,
                size: ImVec2::zero(),
            }
        }
    }
}

unsafe fn input_text_hint_wrapper(label: *const c_char, hint: *const c_char, text: &mut String, flags: InputTextFlags) -> bool {
    let flags = flags | InputTextFlags::CallbackResize;
    text_pre_edit(text);
    let r = ImGui_InputTextWithHint(
        label,
        hint,
        text.as_mut_ptr() as *mut c_char,
        text.capacity(),
        flags.bits(),
        Some(input_text_callback),
        text as *mut String as *mut c_void
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
        pub fn do_input_text_hint<'v, S1: IntoCStr, S2: IntoCStr>(&mut self, label: S1, hint: S2, text: &'v mut String) -> InputTextHint<'v, &mut Self, S1, S2> {
            InputTextHint {
                u:self,
                label:label.into(),
                hint: hint.into(),
                text,
                flags: InputTextFlags::None,
            }
        }
    }
}

pub enum FloatFormat {
    F(u32),
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
        pub fn do_input_float<'v, S: IntoCStr>(&mut self, label: S, value: &'v mut f32) -> InputFloat<'v, &mut Self, S> {
            InputFloat {
                u:self,
                label:label.into(),
                value,
                step: 0.0,
                step_fast: 0.0,
                format: Cow::Borrowed(cstr!("%.3f")),
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
        pub fn do_input_int<'v, S: IntoCStr>(&mut self, label: S, value: &'v mut i32) -> InputInt<'v, &mut Self, S> {
            InputInt {
                u:self,
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
            pub fn $func<'v, S: IntoCStr>(&mut self, label: S, value: &'v mut [f32; $len]) -> $name<'v, &mut Self, S> {
                $name {
                    u:self,
                    label:label.into(),
                    value,
                    format: Cow::Borrowed(cstr!("%.3f")),
                    flags: InputTextFlags::None,
                }
            }
        }
    }

    };
}

decl_builder_input_f!{ InputFloat2 do_input_float_2 ImGui_InputFloat2 2}
decl_builder_input_f!{ InputFloat3 do_input_float_3 ImGui_InputFloat3 3}
decl_builder_input_f!{ InputFloat4 do_input_float_4 ImGui_InputFloat4 4}

impl_float_format!{ InputFloat }
impl_float_format!{ InputFloat2 }
impl_float_format!{ InputFloat3 }
impl_float_format!{ InputFloat4 }


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
            pub fn $func<'v, S: IntoCStr>(&mut self, label: S, value: &'v mut [i32; $len]) -> $name<'v, &mut Self, S> {
                $name {
                    u:self,
                    label:label.into(),
                    value,
                    flags: InputTextFlags::None,
                }
            }
        }
    }

    };
}

decl_builder_input_i!{ InputInt2 do_input_int_2 ImGui_InputInt2 2}
decl_builder_input_i!{ InputInt3 do_input_int_3 ImGui_InputInt3 3}
decl_builder_input_i!{ InputInt4 do_input_int_4 ImGui_InputInt4 4}

decl_builder_with_opt!{Menu, ImGui_BeginMenu, ImGui_EndMenu () (S: IntoCStr)
    (
        name (S::Temp) (name.as_ptr()),
        enabled (bool) (enabled),
    )
    {
        decl_builder_setter!{enabled: bool}
    }
    {
        pub fn do_menu<S: IntoCStr>(&mut self, name: S) -> Menu<&mut Self, S> {
            Menu {
                u: self,
                name: name.into(),
                enabled: true,
                push: (),
            }
        }
    }
}

decl_builder_with_opt!{CollapsingHeader, ImGui_CollapsingHeader, no_op () (S: IntoCStr)
    (
        label (S::Temp) (label.as_ptr()),
        flags (TreeNodeFlags) (flags.bits()),
    )
    {
        decl_builder_setter!{flags: TreeNodeFlags}
    }
    {
        pub fn do_collapsing_header<S: IntoCStr>(&mut self, label: S) -> CollapsingHeader<&mut Self, S> {
            CollapsingHeader {
                u: self,
                label: label.into(),
                flags: TreeNodeFlags::None,
                push: (),
            }
        }
    }
}

decl_builder_with_opt!{TreeNode, ImGui_TreeNodeEx, ImGui_TreePop () (S: IntoCStr)
    (
        label (S::Temp) (label.as_ptr()),
        flags (TreeNodeFlags) (flags.bits()),
    )
    {
        decl_builder_setter!{flags: TreeNodeFlags}
    }
    {
        pub fn do_tree_node<S: IntoCStr>(&mut self, label: S) -> TreeNode<&mut Self, S> {
            TreeNode {
                u: self,
                label: label.into(),
                flags: TreeNodeFlags::None,
                push: (),
            }
        }
    }
}

decl_builder_with_opt!{Popup, ImGui_BeginPopup, ImGui_EndPopup () (S: IntoCStr)
    (
        str_id (S::Temp) (str_id.as_ptr()),
        flags (WindowFlags) (flags.bits()),
    )
    {
        decl_builder_setter!{flags: WindowFlags}
    }
    {
        pub fn do_popup<S: IntoCStr>(&mut self, str_id: S) -> Popup<&mut Self, S> {
            Popup {
                u: self,
                str_id: str_id.into(),
                flags: WindowFlags::None,
                push: (),
            }
        }
    }
}

decl_builder_with_opt!{PopupModal, ImGui_BeginPopupModal, ImGui_EndPopup () (S: IntoCStr)
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
        pub fn do_popup_modal<S: IntoCStr>(&mut self, name: S) -> PopupModal<&mut Self, S> {
            PopupModal {
                u: self,
                name: name.into(),
                opened: None,
                flags: WindowFlags::None,
                push: (),
            }
        }
    }
}

decl_builder_with_opt!{Combo, ImGui_BeginCombo, ImGui_EndCombo () (S1: IntoCStr, S2: IntoCStr)
    (
        label (S1::Temp) (label.as_ptr()),
        preview_value (Option<S2::Temp>) (optional_str(&preview_value)),
        flags (ComboFlags) (flags.bits()),
    )
    {
        decl_builder_setter!{flags: ComboFlags}
        pub fn preview_value_opt<S3: IntoCStr>(self, preview_value: Option<S3>) -> Combo<U, S1, S3> {
            Combo {
                u: self.u,
                label: self.label,
                preview_value: preview_value.map(|x| x.into()),
                flags: ComboFlags::None,
                push: (),
            }
        }
        pub fn preview_value<S3: IntoCStr>(self, preview_value: S3) -> Combo<U, S1, S3> {
            self.preview_value_opt(Some(preview_value))
        }
    }
    {
        pub fn do_combo<S: IntoCStr>(&mut self, label: S) -> Combo<&mut Self, S, &'static str> {
            Combo {
                u: self,
                label: label.into(),
                preview_value: None,
                flags: ComboFlags::None,
                push: (),
            }
        }
        // Helper function for simple use cases
        pub fn combo<'a, V: Copy + PartialEq, S2: IntoCStr>(
            &mut self,
            label: impl IntoCStr,
            values: impl IntoIterator<Item=V>,
            f_name: impl Fn(V) -> S2,
            current: &'a mut V
        ) -> bool
        {
            let mut changed = false;
            self.do_combo(label)
                .preview_value(f_name(*current))
                .with(|ui| {
                    for val in values {
                        if ui.do_selectable(f_name(val))
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

impl<'ctx, D: 'ctx> Ui<'ctx, D> {
    // The callback will be callable until the next call to do_frame()
    unsafe fn push_callback<A>(&mut self, mut cb: impl FnMut(&'ctx mut D, A) + 'ctx) -> usize {
        let cb = Box::new(move |data: &'ctx mut D, ptr: *mut c_void| {
            let a = ptr as *mut A;
            cb(data, unsafe { std::ptr::read(a) });
        });
        let id = self.callbacks.len();

        self.callbacks.push(cb);
        merge_generation(id, self.generation)
    }
    unsafe fn run_callback<A>(id: usize, a: A) {
        let io = &*ImGui_GetIO();
        if io.BackendLanguageUserData.is_null() {
            return;
        }
        let backend = &*(io.BackendLanguageUserData as *const BackendData);
        let Some(id) = remove_generation(id, backend.generation) else {
            // lost callback!
            return;
        };

        // The lifetime of ui has been erased, but at least the type of D should be correct
        let ui = &mut *(backend.ui_ptr as *mut Self);

        let cb = &mut ui.callbacks[id];
        let mut a = MaybeUninit::new(a);
        cb(ui.data, a.as_mut_ptr() as *mut c_void);
    }
    pub fn data(&mut self) -> &mut D {
        self.data
    }
    pub fn set_next_window_size_constraints_callback(&mut self,
        size_min: impl Into<Vector2>,
        size_max: impl Into<Vector2>,
        cb: impl FnMut(&'ctx mut D, SizeCallbackData<'_>) + 'ctx,
    )
    {
        unsafe {
            let id = self.push_callback(cb);
            ImGui_SetNextWindowSizeConstraints(
                &size_min.into().into(),
                &size_max.into().into(),
                Some(call_size_callback::<D>),
                id as *mut c_void,
            );
        }
    }
    pub fn set_next_window_size_constraints(&mut self,
        size_min: impl Into<Vector2>,
        size_max: impl Into<Vector2>,
    )
    {
        unsafe {
            ImGui_SetNextWindowSizeConstraints(
                &size_min.into().into(),
                &size_max.into().into(),
                None,
                null_mut(),
            );
        }
    }
    pub fn set_next_item_width(&mut self, item_width: f32) {
        unsafe {
            ImGui_SetNextItemWidth(item_width);
        }
    }

    with_begin_end!{with_group ImGui_BeginGroup ImGui_EndGroup ()}
    with_begin_end!{with_disabled ImGui_BeginDisabled ImGui_EndDisabled (
        disabled (bool) (disabled),
    ) }
    with_begin_end!{with_clip_rect ImGui_PushClipRect ImGui_PopClipRect (
        clip_rect_min (Vector2) (&clip_rect_min.into()),
        clip_rect_max (Vector2) (&clip_rect_max.into()),
        intersect_with_current_clip_rect (bool) (intersect_with_current_clip_rect),
    ) }

    with_begin_end_opt!{with_main_menu_bar ImGui_BeginMainMenuBar ImGui_EndMainMenuBar ()}
    with_begin_end_opt!{with_menu_bar ImGui_BeginMenuBar ImGui_EndMenuBar () }
    with_begin_end_opt!{with_tooltip ImGui_BeginTooltip ImGui_EndTooltip () }
    with_begin_end_opt!{with_item_tooltip ImGui_BeginItemTooltip ImGui_EndTooltip () }

    pub fn with_push<R>(&mut self, style: impl Pushable, f: impl FnOnce(&mut Self) -> R) -> R {
        let r;
        unsafe {
            style.push();
            r = f(self);
            style.pop();
        }
        r
    }
    pub fn show_demo_window(&mut self, show: Option<&mut bool>) {
        unsafe {
            ImGui_ShowDemoWindow(show.map(|b| b as *mut bool).unwrap_or(null_mut()));
        }
    }
    pub fn set_next_window_pos(&mut self, pos: impl Into<Vector2>, cond: Cond, pivot: impl Into<Vector2>) {
        unsafe {
            ImGui_SetNextWindowPos(&pos.into().into(), cond.bits(), &pivot.into().into());
        }
    }
    pub fn set_next_window_size(&mut self, size: impl Into<Vector2>, cond: Cond) {
        unsafe {
            ImGui_SetNextWindowSize(&size.into().into(), cond.bits());
        }
    }
    pub fn set_next_window_content_size(&mut self, size: impl Into<Vector2>) {
        unsafe {
            ImGui_SetNextWindowContentSize(&size.into().into());
        }
    }

    pub fn set_next_window_collapsed(&mut self, collapsed: bool, cond: Cond) {
        unsafe {
           ImGui_SetNextWindowCollapsed(collapsed, cond.bits());
        }
    }

    pub fn set_next_window_focus(&mut self) {
        unsafe {
           ImGui_SetNextWindowFocus();
        }
    }

    pub fn set_next_window_scroll(&mut self, scroll: impl Into<Vector2>) {
        unsafe {
            ImGui_SetNextWindowScroll(&scroll.into().into());
        }
    }

    pub fn set_next_window_bg_alpha(&mut self, alpha: f32) {
        unsafe {
            ImGui_SetNextWindowBgAlpha(alpha);
        }
    }
    pub fn window_draw_list<'a>(&'a mut self) -> WindowDrawList<'a, 'ctx, D> {
        unsafe {
            let ptr = ImGui_GetWindowDrawList();
            WindowDrawList {
                ui: self,
                ptr: &mut *ptr,
            }
        }
    }
    pub fn foreground_draw_list<'a>(&'a mut self) -> WindowDrawList<'a, 'ctx, D> {
        unsafe {
            let ptr = ImGui_GetForegroundDrawList();
            WindowDrawList {
                ui: self,
                ptr: &mut *ptr,
            }
        }
    }
    pub fn background_draw_list<'a>(&'a mut self) -> WindowDrawList<'a, 'ctx, D> {
        unsafe {
            let ptr = ImGui_GetBackgroundDrawList();
            WindowDrawList {
                ui: self,
                ptr: &mut *ptr,
            }
        }
    }
    pub fn text(&mut self, text: &str) {
        unsafe {
            let (start, end) = text_ptrs(text);
            ImGui_TextUnformatted(start, end);
        }

    }
    pub fn text_colored(&mut self, color: impl Into<Color>, text: impl IntoCStr) {
        let text = text.into();
        unsafe {
            ImGui_TextColored(&color.into().into(), cstr!("%s").as_ptr(), text.as_ptr())
        }
    }
    pub fn text_disabled(&mut self, text: impl IntoCStr) {
        let text = text.into();
        unsafe {
            ImGui_TextDisabled(cstr!("%s").as_ptr(), text.as_ptr())
        }
    }
    pub fn text_wrapped(&mut self, text: impl IntoCStr) {
        let text = text.into();
        unsafe {
            ImGui_TextWrapped(cstr!("%s").as_ptr(), text.as_ptr())
        }
    }
    pub fn label_text(&mut self, label: impl IntoCStr, text: impl IntoCStr) {
        let label = label.into();
        let text = text.into();
        unsafe {
            ImGui_LabelText(label.as_ptr(), cstr!("%s").as_ptr(), text.as_ptr())
        }
    }
    pub fn bullet_text(&mut self, text: impl IntoCStr) {
        let text = text.into();
        unsafe {
            ImGui_BulletText(cstr!("%s").as_ptr(), text.as_ptr())
        }
    }
    pub fn bullet(&mut self) {
        unsafe {
            ImGui_Bullet();
        }
    }
    pub fn separator_text(&mut self, text: impl IntoCStr) {
        let text = text.into();
        unsafe {
            ImGui_SeparatorText(text.as_ptr());
        }
    }
    pub fn separator(&mut self) {
        unsafe {
            ImGui_Separator();
        }
    }

    pub fn set_item_default_focus(&mut self) {
        unsafe {
            ImGui_SetItemDefaultFocus();
        }
    }
    pub fn is_item_hovered(&mut self, flags: HoveredFlags) -> bool {
        unsafe {
            ImGui_IsItemHovered(flags.bits())
        }
    }
    pub fn is_item_active(&mut self) -> bool {
        unsafe {
            ImGui_IsItemActive()
        }
    }
    pub fn is_item_focused(&mut self) -> bool {
        unsafe {
            ImGui_IsItemFocused()
        }
    }
    pub fn is_item_clicked(&mut self, flags: MouseButton) -> bool {
        unsafe {
            ImGui_IsItemClicked(flags.bits())
        }
    }
    pub fn is_item_visible(&mut self) -> bool {
        unsafe {
            ImGui_IsItemVisible()
        }
    }
    pub fn is_item_edited(&mut self) -> bool {
        unsafe {
            ImGui_IsItemEdited()
        }
    }
    pub fn is_item_activated(&mut self) -> bool {
        unsafe {
            ImGui_IsItemActivated()
        }
    }
    pub fn is_item_deactivated(&mut self) -> bool {
        unsafe {
            ImGui_IsItemDeactivated()
        }
    }
    pub fn is_item_deactivated_after_edit(&mut self) -> bool {
        unsafe {
            ImGui_IsItemDeactivatedAfterEdit()
        }
    }
    pub fn is_item_toggled_open(&mut self) -> bool {
        unsafe {
            ImGui_IsItemToggledOpen()
        }
    }
    pub fn is_any_item_hovered(&mut self) -> bool {
        unsafe {
            ImGui_IsAnyItemHovered()
        }
    }
    pub fn is_any_item_active(&mut self) -> bool {
        unsafe {
            ImGui_IsAnyItemActive()
        }
    }
    pub fn is_any_item_focused(&mut self) -> bool {
        unsafe {
            ImGui_IsAnyItemFocused()
        }
    }
    pub fn get_item_id(&mut self) -> ImGuiID {
        unsafe {
            ImGui_GetItemID()
        }
    }
    pub fn get_item_rect_min(&mut self) -> Vector2 {
        unsafe {
            ImGui_GetItemRectMin().into()
        }
    }
    pub fn get_item_rect_max(&mut self) -> Vector2 {
        unsafe {
            ImGui_GetItemRectMax().into()
        }
    }
    pub fn get_item_rect_size(&mut self) -> Vector2 {
        unsafe {
            ImGui_GetItemRectSize().into()
        }
    }
    pub fn get_main_viewport<'s>(&'s mut self) -> Viewport<'s> {
        unsafe {
            Viewport {
                ptr: &*ImGui_GetMainViewport()
            }
        }
    }
    pub fn get_content_region_avail(&mut self) -> Vector2 {
        unsafe {
            ImGui_GetContentRegionAvail().into()
        }
    }
    pub fn get_content_region_max(&mut self) -> Vector2 {
        unsafe {
            ImGui_GetContentRegionMax().into()
        }
    }
    pub fn get_window_content_region_min(&mut self) -> Vector2 {
        unsafe {
            ImGui_GetWindowContentRegionMin().into()
        }
    }
    pub fn get_window_content_region_max(&mut self) -> Vector2 {
        unsafe {
            ImGui_GetWindowContentRegionMax().into()
        }
    }
    pub fn get_scroll_x(&mut self) -> f32 {
        unsafe {
            ImGui_GetScrollX()
        }
    }
    pub fn get_scroll_y(&mut self) -> f32 {
        unsafe {
            ImGui_GetScrollY()
        }
    }
    pub fn set_scroll_x(&mut self, scroll_x: f32) {
        unsafe {
            ImGui_SetScrollX(scroll_x);
        }
    }
    pub fn set_scroll_y(&mut self, scroll_y: f32) {
        unsafe {
            ImGui_SetScrollY(scroll_y);
        }
    }
    pub fn get_scroll_max_x(&mut self) -> f32 {
        unsafe {
            ImGui_GetScrollMaxX()
        }
    }
    pub fn get_scroll_max_y(&mut self) -> f32 {
        unsafe {
            ImGui_GetScrollMaxY()
        }
    }
    pub fn set_scroll_here_x(&mut self, center_x_ratio: f32) {
        unsafe {
            ImGui_SetScrollHereX(center_x_ratio);
        }
    }
    pub fn set_scroll_here_y(&mut self, center_y_ratio: f32) {
        unsafe {
            ImGui_SetScrollHereY(center_y_ratio);
        }
    }
    pub fn set_scroll_from_pos_x(&mut self, local_x: f32, center_x_ratio: f32) {
        unsafe {
            ImGui_SetScrollFromPosX(local_x, center_x_ratio);
        }
    }
    pub fn set_scroll_from_pos_y(&mut self, local_y: f32, center_y_ratio: f32) {
        unsafe {
            ImGui_SetScrollFromPosY(local_y, center_y_ratio);
        }
    }
    pub fn same_line(&mut self) {
        unsafe {
            ImGui_SameLine(0.0, -1.0);
        }
    }
    pub fn same_line_ex(&mut self, offset_from_start_x: f32, spacing: f32) {
        unsafe {
            ImGui_SameLine(offset_from_start_x, spacing);
        }
    }
    pub fn new_line(&mut self) {
        unsafe {
            ImGui_NewLine();
        }
    }
    pub fn spacing(&mut self) {
        unsafe {
            ImGui_Spacing();
        }
    }
    pub fn dummy(&mut self, size: impl Into<Vector2>) {
        unsafe {
            ImGui_Dummy(&size.into().into());
        }
    }
    pub fn indent(&mut self, indent_w: f32) {
        unsafe {
            ImGui_Indent(indent_w);
        }
    }
    pub fn unindent(&mut self, indent_w: f32) {
        unsafe {
            ImGui_Unindent(indent_w);
        }
    }
    pub fn get_cursor_pos(&mut self) -> Vector2 {
        unsafe {
            ImGui_GetCursorPos().into()
        }
    }
    pub fn get_cursor_pos_x(&mut self) -> f32 {
        unsafe {
            ImGui_GetCursorPosX()
        }
    }
    pub fn get_cursor_pos_y(&mut self) -> f32 {
        unsafe {
            ImGui_GetCursorPosY()
        }
    }
    pub fn set_cursor_pos(&mut self, local_pos: impl Into<Vector2>) {
        unsafe {
            ImGui_SetCursorPos(&local_pos.into().into());
        }
    }
    pub fn set_cursor_pos_x(&mut self, local_x: f32) {
        unsafe {
            ImGui_SetCursorPosX(local_x);
        }
    }
    pub fn set_cursor_pos_y(&mut self, local_y: f32) {
        unsafe {
            ImGui_SetCursorPosY(local_y);
        }
    }
    pub fn get_cursor_start_pos(&mut self) -> Vector2 {
        unsafe {
            ImGui_GetCursorStartPos().into()
        }
    }
    pub fn get_cursor_screen_pos(&mut self) -> Vector2 {
        unsafe {
            ImGui_GetCursorScreenPos().into()
        }
    }
    pub fn set_cursor_screen_pos(&mut self, pos: impl Into<Vector2>) {
        unsafe {
            ImGui_SetCursorScreenPos(&pos.into().into());
        }
    }
    pub fn align_text_to_frame_padding(&mut self) {
        unsafe {
            ImGui_AlignTextToFramePadding();
        }
    }
    pub fn get_text_line_height(&mut self) -> f32 {
        unsafe {
            ImGui_GetTextLineHeight()
        }
    }
    pub fn get_text_line_height_with_spacing(&mut self) -> f32 {
        unsafe {
            ImGui_GetTextLineHeightWithSpacing()
        }
    }
    pub fn get_frame_height(&mut self) -> f32 {
        unsafe {
            ImGui_GetFrameHeight()
        }
    }
    pub fn get_frame_height_with_spacing(&mut self) -> f32 {
        unsafe {
            ImGui_GetFrameHeightWithSpacing()
        }
    }
    pub fn calc_text_size(&mut self, text: &str, hide_text_after_double_hash: bool, wrap_width: f32) -> Vector2 {
        unsafe {
            let (start, end) = text_ptrs(text);
            ImGui_CalcTextSize(start, end, hide_text_after_double_hash, wrap_width).into()
        }
    }
    pub fn is_key_down(&mut self, key: Key) -> bool {
        unsafe {
            ImGui_IsKeyDown(ImGuiKey(key.bits()))
        }
    }
    pub fn is_key_pressed(&mut self, key: Key) -> bool {
        unsafe {
            ImGui_IsKeyPressed(ImGuiKey(key.bits()), /*repeat*/ true)
        }
    }
    pub fn is_key_pressed_no_repeat(&mut self, key: Key) -> bool {
        unsafe {
            ImGui_IsKeyPressed(ImGuiKey(key.bits()), /*repeat*/ false)
        }
    }
    pub fn is_key_released(&mut self, key: Key) -> bool {
        unsafe {
            ImGui_IsKeyReleased(ImGuiKey(key.bits()))
        }
    }
    pub fn get_key_pressed_amount(&mut self, key: Key, repeat_delay: f32, rate: f32) -> i32 {
        unsafe {
            ImGui_GetKeyPressedAmount(ImGuiKey(key.bits()), repeat_delay, rate)
        }
    }
    //GetKeyName
    //SetNextFrameWantCaptureKeyboard
    pub fn get_font_size(&mut self) -> f32 {
        unsafe {
            ImGui_GetFontSize()
        }
    }
    pub fn is_mouse_down(&mut self, button: MouseButton) -> bool {
        unsafe {
            ImGui_IsMouseDown(button.bits())
        }
    }
    pub fn is_mouse_clicked(&mut self, button: MouseButton) -> bool {
        unsafe {
            ImGui_IsMouseClicked(button.bits(), /*repeat*/ false)
        }
    }
    pub fn is_mouse_clicked_repeat(&mut self, button: MouseButton) -> bool {
        unsafe {
            ImGui_IsMouseClicked(button.bits(), /*repeat*/ true)
        }
    }
    pub fn is_mouse_released(&mut self, button: MouseButton) -> bool {
        unsafe {
            ImGui_IsMouseReleased(button.bits())
        }
    }
    pub fn is_mouse_double_clicked(&mut self, button: MouseButton) -> bool {
        unsafe {
            ImGui_IsMouseDoubleClicked(button.bits())
        }
    }
    pub fn get_mouse_clicked_count(&mut self, button: MouseButton) -> i32 {
        unsafe {
            ImGui_GetMouseClickedCount(button.bits())
        }
    }
    /*
    pub fn is_mouse_hovering_rect(&mut self) -> bool {
        unsafe {
            ImGui_IsMouseHoveringRect(const ImVec2& r_min, const ImVec2& r_max, bool clip = true);
        }
    }
    pub fn is_mouse_pos_valid(&mut self) -> bool {
        unsafe {
            ImGui_IsMousePosValid(const ImVec2* mouse_pos = NULL);
        }
    }*/
    pub fn is_any_mouse_down(&mut self) -> bool {
        unsafe {
            ImGui_IsAnyMouseDown()
        }
    }
    pub fn get_mouse_pos(&mut self) -> Vector2 {
        unsafe {
            ImGui_GetMousePos().into()
        }
    }
    pub fn get_mouse_pos_on_opening_current_popup(&mut self) -> Vector2 {
        unsafe {
            ImGui_GetMousePosOnOpeningCurrentPopup().into()
        }
    }
    pub fn is_mouse_dragging(&mut self, button: MouseButton) -> bool {
        unsafe {
            ImGui_IsMouseDragging(button.bits(), /*lock_threshold*/ -1.0)
        }
    }
    pub fn get_mouse_drag_delta(&mut self, button: MouseButton) -> Vector2 {
        unsafe {
            ImGui_GetMouseDragDelta(button.bits(), /*lock_threshold*/ -1.0).into()
        }
    }
    pub fn reset_mouse_drag_delta(&mut self, button: MouseButton) {
        unsafe {
            ImGui_ResetMouseDragDelta(button.bits());
        }
    }
    pub fn get_mouse_cursor(&mut self) -> MouseCursor {
        unsafe {
            MouseCursor::from_bits(ImGui_GetMouseCursor())
                .unwrap_or(MouseCursor::None)
        }
    }
    pub fn set_mouse_cursor(&mut self, cursor_type: MouseCursor) {
        unsafe {
            ImGui_SetMouseCursor(cursor_type.bits());
        }
    }

    pub fn is_popup_open(&mut self, str_id: Option<&str>, flags: PopupFlags) -> bool {
        let temp;
        let str_id = match str_id {
            Some(s) => {
                temp = IntoCStr::into(s);
                temp.as_ptr()
            }
            None => null()
        };
        unsafe {
            ImGui_IsPopupOpen(str_id, flags.bits())
        }
    }
    pub fn open_popup(&mut self, str_id: impl IntoCStr, flags: PopupFlags) {
        let str_id = str_id.into();
        unsafe {
            ImGui_OpenPopup(str_id.as_ptr(), flags.bits());
        }
    }
    pub fn close_current_popup(&mut self) {
        unsafe {
            ImGui_CloseCurrentPopup();
        }
    }
    pub fn is_window_appearing(&mut self) -> bool {
        unsafe {
            ImGui_IsWindowAppearing()
        }
    }

    pub fn io(&mut self) -> &ImGuiIO {
        unsafe {
            &*ImGui_GetIO()
        }
    }
    pub fn font_atlas<'a>(&'a mut self) -> FontAtlas<'a> {
        unsafe {
            let io = &*ImGui_GetIO();
            FontAtlas {
                ptr: FontAtlasPtr { ptr: &mut *io.Fonts },
            }
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FontId(usize);

impl Default for FontId {
    fn default() -> FontId {
        FontId(0)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CustomRectIndex(i32);

impl Default for CustomRectIndex {
    fn default() -> Self {
        // Always invalid, do not use!
        CustomRectIndex(-1)
    }
}

#[derive(Debug)]
pub struct FontAtlasPtr<'a> {
    ptr: &'a mut ImFontAtlas,
}

pub struct FontAtlasMut<'ctx, 'a> {
    ptr: FontAtlasPtr<'a>,
    scale: f32,
    // glyph_ranges pointers have to live until the atlas texture is built
    glyph_ranges: Vec<Vec<[ImWchar; 2]>>,
    custom_rects: Vec<Option<Box<dyn FnOnce(&mut [&mut [[u8; 4]]]) + 'ctx>>>,
}

impl<'a, 'ctx> FontAtlasMut<'ctx, 'a> {
    pub fn add_font(&mut self, font: FontInfo) -> FontId {
        self.add_font_priv(font, false)
    }
    pub fn add_font_collection(&mut self, fonts: impl IntoIterator<Item = FontInfo>) -> FontId {
        let mut fonts = fonts.into_iter();
        let first = fonts.next().expect("empty font collection");
        let id = self.add_font_priv(first, false);
        while let Some(font) = fonts.next() {
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
            let io = &mut *ImGui_GetIO();
            match font.ttf {
                TtfData::Bytes(bytes) => {
                    ImFontAtlas_AddFontFromMemoryTTF(
                        io.Fonts,
                        bytes.as_ptr() as *mut _,
                        bytes.len() as i32,
                        font.size * self.scale,
                        &fc,
                        glyph_ranges
                    );
                }
                TtfData::DefaultFont => {
                    ImFontAtlas_AddFontDefault(io.Fonts, &fc);
                }
            }
            FontId((*io.Fonts).Fonts.len() - 1)
        }
    }
    pub fn add_custom_rect_font_glyph(&mut self, font: FontId, id: char, width: i32, height: i32, advance_x: f32, offset: impl Into<Vector2>, draw: impl FnOnce(&mut [&mut [[u8; 4]]]) + 'ctx) -> CustomRectIndex {
        unsafe {
            let io = &mut *ImGui_GetIO();

            let font = font_ptr(font);
            let idx = ImFontAtlas_AddCustomRectFontGlyph(io.Fonts, font, id as ImWchar, width, height, advance_x, &offset.into().into());
            self.add_custom_rect_at(idx as usize, Box::new(draw));
            CustomRectIndex(idx)
        }
    }
    pub fn add_custom_rect_regular(&mut self, width: i32, height: i32, draw: impl FnOnce(&mut [&mut [[u8; 4]]]) + 'ctx) -> CustomRectIndex {
        unsafe {
            let io = &mut *ImGui_GetIO();

            let idx = ImFontAtlas_AddCustomRectRegular(io.Fonts, width, height);
            self.add_custom_rect_at(idx as usize, Box::new(draw));
            CustomRectIndex(idx)
        }
    }
    fn add_custom_rect_at(&mut self, idx: usize, f: Box<dyn FnOnce(&mut [&mut [[u8; 4]]]) + 'ctx>) {
        if idx >= self.custom_rects.len() {
            self.custom_rects.resize_with(idx + 1, || None);
        }
        self.custom_rects[idx] = Some(f);
    }
    pub fn build_custom_rects(self) {
        let mut tex_data = std::ptr::null_mut();
        let mut tex_width = 0;
        let mut tex_height = 0;
        let mut pixel_size = 0;
        let io;
        unsafe {
            io = &mut *ImGui_GetIO();
            ImFontAtlas_GetTexDataAsRGBA32(io.Fonts, &mut tex_data, &mut tex_width, &mut tex_height, &mut pixel_size);
        }
        let pixel_size = pixel_size as usize;
        assert!(pixel_size == 4);

        for (idx, f) in self.custom_rects.into_iter().enumerate() {
            if let Some(f) = f {
                unsafe {
                    let rect = &(*io.Fonts).CustomRects[idx as usize];
                    let pixel_size = pixel_size as usize;
                    assert!(pixel_size == 4);

                    let stride = tex_width as usize * pixel_size;

                    let rx = rect.X as usize * pixel_size;
                    let ry = rect.Y as usize;
                    let mut pixels = (ry .. ry + rect.Height as usize).map(|y| {
                        let p = tex_data.add(y * stride + rx) as *mut [u8; 4];
                        std::slice::from_raw_parts_mut(p, rect.Width as usize)
                    }).collect::<Vec<_>>();
                    f(&mut pixels);
                }
            }
        }
    }
}

impl<'a> Deref for FontAtlasMut<'_, 'a> {
    type Target = FontAtlasPtr<'a>;
    fn deref(&self) -> &FontAtlasPtr<'a> {
        &self.ptr
    }
}

pub struct FontAtlas<'a> {
    ptr: FontAtlasPtr<'a>,
}

impl<'a> Deref for FontAtlas<'a> {
    type Target = FontAtlasPtr<'a>;
    fn deref(&self) -> &FontAtlasPtr<'a> {
        &self.ptr
    }
}

impl FontAtlasPtr<'_> {
    pub fn texture_id(&self) -> ImTextureID {
        self.ptr.TexID
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
        self.ptr.Pos.into()
    }
    pub fn current_size(&self) -> Vector2 {
        self.ptr.CurrentSize.into()
    }
    pub fn desired_size(&self) -> Vector2 {
        self.ptr.DesiredSize.into()
    }
    pub fn set_desired_size(&mut self, sz: impl Into<Vector2>) {
        self.ptr.DesiredSize = sz.into().into();
    }
}

unsafe extern "C" fn call_size_callback<D>(ptr: *mut ImGuiSizeCallbackData) {
    let ptr = &mut *ptr;
    let id = ptr.UserData as usize;
    let data = SizeCallbackData {
        ptr,
    };
    Ui::<D>::run_callback(id, data);
}

pub struct WindowDrawList<'a, 'ctx, D> {
    ui: &'a mut Ui<'ctx, D>,
    ptr: &'a mut ImDrawList,
}

pub fn color_to_u32(c: impl Into<Color>) -> u32 {
    unsafe {
        ImGui_ColorConvertFloat4ToU32(&c.into().into())
    }
}

impl<'a, 'ctx, D> WindowDrawList<'a, 'ctx, D> {
    pub fn add_line(&mut self, p1: impl Into<Vector2>, p2: impl Into<Vector2>, color: impl Into<Color>, thickness: f32) {
        unsafe {
            ImDrawList_AddLine(self.ptr, &p1.into().into(), &p2.into().into(), color_to_u32(color), thickness);
        }
    }
    pub fn add_rect(&mut self, p_min: impl Into<Vector2>, p_max: impl Into<Vector2>, color: impl Into<Color>, rounding: f32, flags: DrawFlags, thickness: f32) {
        unsafe {
            ImDrawList_AddRect(self.ptr, &p_min.into().into(), &p_max.into().into(), color_to_u32(color), rounding, flags.bits(), thickness);
        }
    }
    pub fn add_rect_filled(&mut self, p_min: impl Into<Vector2>, p_max: impl Into<Vector2>, color: impl Into<Color>, rounding: f32, flags: DrawFlags) {
        unsafe {
            ImDrawList_AddRectFilled(self.ptr, &p_min.into().into(), &p_max.into().into(), color_to_u32(color), rounding, flags.bits());
        }
    }
    pub fn add_rect_filled_multicolor(&mut self, p_min: impl Into<Vector2>, p_max: impl Into<Vector2>, col_upr_left: impl Into<Color>, col_upr_right: impl Into<Color>, col_bot_right: impl Into<Color>, col_bot_left: impl Into<Color>) {
        unsafe {
            ImDrawList_AddRectFilledMultiColor(self.ptr, &p_min.into().into(), &p_max.into().into(), color_to_u32(col_upr_left), color_to_u32(col_upr_right), color_to_u32(col_bot_right), color_to_u32(col_bot_left));
        }
    }
    pub fn add_quad(&mut self, p1: impl Into<Vector2>, p2: impl Into<Vector2>, p3: impl Into<Vector2>, p4: impl Into<Vector2>, color: impl Into<Color>, thickness: f32) {
        unsafe {
            ImDrawList_AddQuad(self.ptr, &p1.into().into(), &p2.into().into(), &p3.into().into(), &p4.into().into(), color_to_u32(color), thickness);
        }
    }
    pub fn add_quad_filled(&mut self, p1: impl Into<Vector2>, p2: impl Into<Vector2>, p3: impl Into<Vector2>, p4: impl Into<Vector2>, color: impl Into<Color>) {
        unsafe {
            ImDrawList_AddQuadFilled(self.ptr, &p1.into().into(), &p2.into().into(), &p3.into().into(), &p4.into().into(), color_to_u32(color));
        }
    }
    pub fn add_triangle(&mut self, p1: impl Into<Vector2>, p2: impl Into<Vector2>, p3: impl Into<Vector2>, color: impl Into<Color>, thickness: f32) {
        unsafe {
            ImDrawList_AddTriangle(self.ptr, &p1.into().into(), &p2.into().into(), &p3.into().into(), color_to_u32(color), thickness);
        }
    }
    pub fn add_triangle_filled(&mut self, p1: impl Into<Vector2>, p2: impl Into<Vector2>, p3: impl Into<Vector2>, color: impl Into<Color>) {
        unsafe {
            ImDrawList_AddTriangleFilled(self.ptr, &p1.into().into(), &p2.into().into(), &p3.into().into(), color_to_u32(color));
        }
    }
    pub fn add_circle(&mut self, center: impl Into<Vector2>, radius: f32, color: impl Into<Color>, num_segments: i32, thickness: f32) {
        unsafe {
            ImDrawList_AddCircle(self.ptr, &center.into().into(), radius, color_to_u32(color), num_segments, thickness);
        }
    }
    pub fn add_circle_filled(&mut self, center: impl Into<Vector2>, radius: f32, color: impl Into<Color>, num_segments: i32) {
        unsafe {
            ImDrawList_AddCircleFilled(self.ptr, &center.into().into(), radius, color_to_u32(color), num_segments);
        }
    }
    pub fn add_ngon(&mut self, center: impl Into<Vector2>, radius: f32, color: impl Into<Color>, num_segments: i32, thickness: f32) {
        unsafe {
            ImDrawList_AddNgon(self.ptr, &center.into().into(), radius, color_to_u32(color), num_segments, thickness);
        }
    }
    pub fn add_ngon_filled(&mut self, center: impl Into<Vector2>, radius: f32, color: impl Into<Color>, num_segments: i32) {
        unsafe {
            ImDrawList_AddNgonFilled(self.ptr, &center.into().into(), radius, color_to_u32(color), num_segments);
        }
    }
    pub fn add_text(&mut self, pos: impl Into<Vector2>, color: impl Into<Color>, text: &str) {
        unsafe {
            let (start, end) = text_ptrs(text);
            ImDrawList_AddText(self.ptr, &pos.into().into(), color_to_u32(color), start, end);
        }
    }
    pub fn add_text_ex(&mut self, font: FontId, font_size: f32, pos: impl Into<Vector2>, color: impl Into<Color>, text: &str, wrap_width: f32, cpu_fine_clip_rect: Option<ImVec4>) {
        unsafe {
            let (start, end) = text_ptrs(text);
            ImDrawList_AddText1(
                self.ptr, font_ptr(font), font_size, &pos.into().into(), color_to_u32(color), start, end,
                wrap_width, cpu_fine_clip_rect.as_ref().map(|x| x as *const _).unwrap_or(null())
            );
        }
    }
    pub fn add_polyline(&mut self, points: &[ImVec2], color: impl Into<Color>, flags: DrawFlags, thickness: f32) {
        unsafe {
            ImDrawList_AddPolyline(self.ptr, points.as_ptr(), points.len() as i32, color_to_u32(color), flags.bits(), thickness);
        }
    }
    pub fn add_convex_poly_filled(&mut self, points: &[ImVec2], color: impl Into<Color>) {
        unsafe {
            ImDrawList_AddConvexPolyFilled(self.ptr, points.as_ptr(), points.len() as i32, color_to_u32(color));
        }
    }
    pub fn add_bezier_cubic(&mut self, p1: impl Into<Vector2>, p2: impl Into<Vector2>, p3: impl Into<Vector2>, p4: impl Into<Vector2>, color: impl Into<Color>, thickness: f32, num_segments: i32) {
        unsafe {
            ImDrawList_AddBezierCubic(self.ptr, &p1.into().into(), &p2.into().into(), &p3.into().into(), &p4.into().into(), color_to_u32(color), thickness, num_segments);
        }
    }
    pub fn add_bezier_quadratic(&mut self, p1: impl Into<Vector2>, p2: impl Into<Vector2>, p3: impl Into<Vector2>, color: impl Into<Color>, thickness: f32, num_segments: i32) {
        unsafe {
            ImDrawList_AddBezierQuadratic(self.ptr, &p1.into().into(), &p2.into().into(), &p3.into().into(), color_to_u32(color), thickness, num_segments);
        }
    }
    pub fn add_image(&mut self, user_texture_id: ImTextureID, p_min: impl Into<Vector2>, p_max: impl Into<Vector2>, uv_min: impl Into<Vector2>, uv_max: impl Into<Vector2>, color: impl Into<Color>) {
        unsafe {
            ImDrawList_AddImage(self.ptr, user_texture_id, &p_min.into().into(), &p_max.into().into(), &uv_min.into().into(), &uv_max.into().into(), color_to_u32(color));
        }
    }
    pub fn add_image_quad(&mut self, user_texture_id: ImTextureID, p1: impl Into<Vector2>, p2: impl Into<Vector2>, p3: impl Into<Vector2>, p4: impl Into<Vector2>, uv1: impl Into<Vector2>, uv2: impl Into<Vector2>, uv3: impl Into<Vector2>, uv4: impl Into<Vector2>, color: impl Into<Color>) {
        unsafe {
            ImDrawList_AddImageQuad(self.ptr, user_texture_id, &p1.into().into(), &p2.into().into(), &p3.into().into(), &p4.into().into(), &uv1.into().into(), &uv2.into().into(), &uv3.into().into(), &uv4.into().into(), color_to_u32(color));
        }
    }
    pub fn add_image_rounded(&mut self, user_texture_id: ImTextureID, p_min: impl Into<Vector2>, p_max: impl Into<Vector2>, uv_min: impl Into<Vector2>, uv_max: impl Into<Vector2>, color: impl Into<Color>, rounding: f32, flags: DrawFlags) {
        unsafe {
            ImDrawList_AddImageRounded(self.ptr, user_texture_id, &p_min.into().into(), &p_max.into().into(), &uv_min.into().into(), &uv_max.into().into(), color_to_u32(color), rounding, flags.bits());
        }
    }

    pub fn add_callback(&mut self, cb: impl FnOnce(&'ctx mut D) + 'ctx) {
        // Callbacks are only called once, convert the FnOnce into an FnMut to register
        let mut cb = Some(cb);
        unsafe {
            let id = self.ui.push_callback(move |d, _: ()| {
                if let Some(cb) = cb.take() {
                    cb(d);
                }
            });
            ImDrawList_AddCallback(self.ptr, Some(call_drawlist_callback::<D>), id as *mut c_void);
        }
    }
    pub fn add_draw_cmd(&mut self) {
        unsafe {
            ImDrawList_AddDrawCmd(self.ptr);
        }

    }
}

unsafe extern "C" fn call_drawlist_callback<D>(_parent_lilst: *const ImDrawList, cmd: *const ImDrawCmd) {
    let id = (*cmd).UserCallbackData as usize;
    Ui::<D>::run_callback(id, ());
}

pub trait Hashable {
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

pub trait Pushable {
    unsafe fn push(&self);
    unsafe fn pop(&self);
}

impl Pushable for () {
    unsafe fn push(&self) {}
    unsafe fn pop(&self) {}
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

pub type TextureId = ImTextureID;

impl Pushable for StyleColor {
    unsafe fn push(&self) {
        ImGui_PushStyleColor(self.0.bits(), color_to_u32(self.1));
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
            StyleValue::Vec2(v) => ImGui_PushStyleVar1(self.0.bits(), &v.into()),
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
        self.ptr.Pos.into()
    }
    pub fn size(&self) -> Vector2 {
        self.ptr.Size.into()
    }
    pub fn work_pos(&self) -> Vector2 {
        self.ptr.WorkPos.into()
    }
    pub fn work_size(&self) -> Vector2 {
        self.ptr.WorkSize.into()
    }
}
