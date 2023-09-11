use std::ffi::{CString, c_char, CStr, c_void};
use std::ops::Deref;
use std::ptr::{null, null_mut};
use std::mem::MaybeUninit;
use std::cell::UnsafeCell;
use std::borrow::Cow;
use cstr::cstr;
use dear_imgui_sys::*;

mod enums;
pub use enums::*;

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
    fonts: Vec<FontInfo>,
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
            //io.FontAllowUserScaling = true;
            //ImGui_StyleColorsDark(null_mut());
            imgui
        };
        Context {
            imgui,
            backend,
            pending_atlas: true,
            fonts: Vec::new(),
        }
    }
    pub unsafe fn set_current(&mut self) {
        ImGui_SetCurrentContext(self.imgui);
    }
    pub unsafe fn set_size(&mut self, size: ImVec2, scale: f32) {
        self.pending_atlas = true;
        let io = &mut *ImGui_GetIO();
        io.DisplaySize = size;
        io.DisplayFramebufferScale = ImVec2 { x: scale, y: scale };
        io.FontGlobalScale = scale.recip();
    }
    pub fn add_font(&mut self, mut font: FontInfo) -> FontId {
        self.pending_atlas = true;
        let id = match self.fonts.last() {
            None => 0,
            Some(f) => f.id + 1,
        };
        font.id = id;

        self.fonts.push(font);
        FontId(id)
    }
    pub fn merge_font(&mut self, mut font: FontInfo) {
        self.pending_atlas = true;
        font.merge = true;
        font.id = self.fonts.last().expect("first font cannot be merge").id;
        self.fonts.push(font);
    }
    pub unsafe fn update_atlas(&mut self) -> bool {
        if !std::mem::take(&mut self.pending_atlas) {
            return false;
        }
        let io = &mut *ImGui_GetIO();
        ImFontAtlas_Clear(io.Fonts);

        let scale = io.DisplayFramebufferScale.x;
        for font in &self.fonts {
            let mut fc = ImFontConfig::new();
            // This is ours, do not free()
            fc.FontDataOwnedByAtlas = false;

            fc.MergeMode = font.merge;

            // glyph_ranges must be valid for the duration of the atlas, so do not modify the existing self.fonts.
            // You can add new fonts however, but they will not show unless you call update_altas() again
            let glyph_ranges = if font.char_ranges.len() > 1 {
                font.char_ranges[0].as_ptr()
            } else {
                null()
            };
            ImFontAtlas_AddFontFromMemoryTTF(
                io.Fonts,
                font.ttf.as_ptr() as *mut _,
                font.ttf.len() as i32,
                font.size * scale,
                &fc,
                glyph_ranges
            );
        }
        true
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
}

pub struct FontInfo {
    ttf: Cow<'static, [u8]>,
    size: f32,
    char_ranges: Vec<[ImWchar; 2]>,
    merge: bool,
    id: usize,
}

impl FontInfo {
    pub fn new(ttf: impl Into<Cow<'static, [u8]>>, size: f32) -> Self {
        FontInfo {
            ttf: ttf.into(),
            size,
            char_ranges: vec![[0, 0]], //always a [0,0] at the end
            merge: false,
            id: 0,
        }
    }
    pub fn char_range(mut self, char_from: ImWchar, char_to: ImWchar) -> Self {
        *self.char_ranges.last_mut().unwrap() = [char_from, char_to];
        self.char_ranges.push([0, 0]);
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

// Take care to not consume the argument before using the pointer
fn optional_str<S: Deref<Target = CStr>>(t: &Option<S>) -> *const c_char {
    t.as_ref().map(|s| s.as_ptr()).unwrap_or(null())
}

// helper functions

pub unsafe fn text_ptrs(text: &str) -> (*const c_char, *const c_char) {
    let btxt = text.as_bytes();
    let start = btxt.as_ptr() as *const c_char;
    let end = unsafe { start.add(btxt.len()) };
    ( start, end )

}
pub unsafe fn font_ptr(font: FontId) -> *mut ImFont {
    let io = &*ImGui_GetIO();
    let fonts = &*io.Fonts;
    fonts.Fonts[font.0]
}

pub struct Ui<'ctx, D> {
    data: &'ctx mut D,
    generation: usize,
    callbacks: Vec<Box<dyn FnMut(&'ctx mut D, *mut c_void) + 'ctx>>,
}

macro_rules! with_begin_end {
    ( $name:ident $begin:ident $end:ident ($($arg:ident ($($type:tt)*) { $($init:tt)* } ($pass:expr),)*) ) => {
        pub fn $name<R>(&mut self, $($arg: $($type)*,)* f: impl FnOnce(&mut Self) -> R) -> R {
            $(
                $($init)*
            )*
            unsafe { $begin( $( $pass, )* ) }
            let r = f(self);
            unsafe { $end() }
            r
        }
    };
}

macro_rules! with_begin_end_opt {
    ( $name:ident $begin:ident $end:ident ($($arg:ident ($($type:tt)*) { $($init:tt)* } ($pass:expr),)*) ) => {
        pub fn $name<R>(&mut self, $($arg: $($type)*,)* f: impl FnOnce(&mut Self) -> R) -> Option<R> {
            $(
                $($init)*
            )*
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
                $arg:ident ($($ty:tt)*) ($($init:tt)*) ($pass:expr)
            ,)*
        )
        { $($extra:tt)* }
        { $($constructor:tt)* }
    ) => {
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

macro_rules! decl_builder_setter {
    ($name:ident: $ty:ty) => {
        pub fn $name(mut self, $name: $ty) -> Self {
            self.$name = $name;
            self
        }
    };
}

decl_builder!{ MenuItem -> bool, ImGui_MenuItem () (S1: IntoCStr, S2: IntoCStr)
    (
        label (S1::Temp) (let label = label.into()) (label.as_ptr()),
        shortcut (Option<S2::Temp>) (let shortcut = shortcut.into()) (optional_str(&shortcut)),
        selected (bool) () (selected),
        enabled (bool) () (enabled),
    )
    {
        pub fn shortcut<S3: IntoCStr>(self, shortcut: S3) -> MenuItem<U, S1, S3> {
            MenuItem {
                u: self.u,
                label: self.label,
                shortcut: Some(shortcut.into()),
                selected: self.selected,
                enabled: self.enabled,
            }
        }
        decl_builder_setter!{selected: bool}
        decl_builder_setter!{enabled: bool}
    }
    {
        #[must_use]
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
        label (S::Temp) (let label = label.into()) (label.as_ptr()),
        size (ImVec2) () (&size),
    )
    {
        decl_builder_setter!{size: ImVec2}
    }
    {
        #[must_use]
        pub fn do_button<S: IntoCStr>(&mut self, label: S) -> Button<&mut Self, S> {
            Button {
                u: self,
                label: label.into(),
                size: [0.0, 0.0].into(),
            }
        }
    }
}

decl_builder! { SmallButton -> bool, ImGui_SmallButton () (S: IntoCStr)
    (
        label (S::Temp) (let label = label.into()) (label.as_ptr()),
    )
    {}
    {
        #[must_use]
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
        id (S::Temp) (let id = id.into()) (id.as_ptr()),
        size (ImVec2) () (&size),
        flags (ButtonFlags) () (flags.bits()),
    )
    {
        decl_builder_setter!{size: ImVec2}
        decl_builder_setter!{flags: ButtonFlags}
    }
    {
        #[must_use]
        pub fn do_invisible_button<S: IntoCStr>(&mut self, id: S) -> InvisibleButton<&mut Self, S> {
            InvisibleButton {
                u: self,
                id: id.into(),
                size: [0.0, 0.0].into(),
                flags: ButtonFlags::MouseButtonLeft,
            }
        }
    }
}

decl_builder! { ArrowButton -> bool, ImGui_ArrowButton () (S: IntoCStr)
    (
        id (S::Temp) (let id = id.into()) (id.as_ptr()),
        dir (Dir) () (dir.bits()),
    )
    {}
    {
        #[must_use]
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
        label (S::Temp) (let label = label.into()) (label.as_ptr()),
        value (&'v mut bool) () (value),
    )
    {}
    {
        #[must_use]
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
        label (S::Temp) (let label = label.into()) (label.as_ptr()),
        active (bool) () (active),
    )
    {}
    {
        #[must_use]
        pub fn do_radio_button<S: IntoCStr>(&mut self, label: S, active: bool) -> RadioButton<&mut Self, S> {
            RadioButton {
                u: self,
                label: label.into(),
                active,
            }
        }
    }
}

decl_builder! { Selectable -> bool, ImGui_Selectable () (S: IntoCStr)
    (
        label (S::Temp) (let label = label.into()) (label.as_ptr()),
        selected (bool) () (selected),
        flags (SelectableFlags) () (flags.bits()),
        size (ImVec2) () (&size),
    )
    {
        decl_builder_setter!{selected: bool}
        decl_builder_setter!{flags: SelectableFlags}
        decl_builder_setter!{size: ImVec2}
    }
    {
        #[must_use]
        pub fn do_selectable<S: IntoCStr>(&mut self, label: S) -> Selectable<&mut Self, S> {
            Selectable {
                u: self,
                label: label.into(),
                selected: false,
                flags: SelectableFlags::None,
                size: [0.0, 0.0].into(),
            }
        }
    }
}

macro_rules! decl_builder_drag {
    ($name:ident $func:ident $cfunc:ident $life:lifetime ($argty:ty) ($ty:ty) ($expr:expr)) => {
        decl_builder! { $name -> bool, $cfunc ($life) (S: IntoCStr)
            (
                label (S::Temp) (let label = label.into()) (label.as_ptr()),
                value ($ty) () ($expr(value)),
                speed (f32) () (speed),
                min ($argty) () (min),
                max ($argty) () (max),
                format (&'static CStr) () (format.as_ptr()),
                flags (SliderFlags) () (flags.bits()),
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
                #[must_use]
                pub fn $func<$life, S: IntoCStr>(&mut self, label: S, value: $ty) -> $name<$life, &mut Self, S> {
                    $name {
                        u: self,
                        label: label.into(),
                        value,
                        speed: 1.0,
                        min: <$argty>::default(),
                        max: <$argty>::default(),
                        format: cstr!("%.3f"),
                        flags: SliderFlags::None,
                    }
                }
            }
        }
    };
}

decl_builder_drag!{ DragFloat do_drag_float ImGui_DragFloat 'v (f32) (&'v mut f32) (std::convert::identity)}
decl_builder_drag!{ DragFloat2 do_drag_float_2 ImGui_DragFloat2 'v (f32) (&'v mut [f32; 2]) (<[f32]>::as_mut_ptr)}
decl_builder_drag!{ DragFloat3 do_drag_float_3 ImGui_DragFloat3 'v (f32) (&'v mut [f32; 3]) (<[f32]>::as_mut_ptr)}
decl_builder_drag!{ DragFloat4 do_drag_float_4 ImGui_DragFloat4 'v (f32) (&'v mut [f32; 4]) (<[f32]>::as_mut_ptr)}

decl_builder_drag!{ DragInt do_drag_int ImGui_DragInt 'v (i32) (&'v mut i32) (std::convert::identity)}
decl_builder_drag!{ DragInt2 do_drag_int_2 ImGui_DragInt2 'v (i32) (&'v mut [i32; 2]) (<[i32]>::as_mut_ptr)}
decl_builder_drag!{ DragInt3 do_drag_int_3 ImGui_DragInt3 'v (i32) (&'v mut [i32; 3]) (<[i32]>::as_mut_ptr)}
decl_builder_drag!{ DragInt4 do_drag_int_4 ImGui_DragInt4 'v (i32) (&'v mut [i32; 4]) (<[i32]>::as_mut_ptr)}

macro_rules! decl_builder_slider {
    ($name:ident $func:ident $cfunc:ident $life:lifetime ($argty:ty) ($ty:ty) ($expr:expr)) => {
        decl_builder! { $name -> bool, $cfunc ($life) (S: IntoCStr)
            (
                label (S::Temp) (let label = label.into()) (label.as_ptr()),
                value ($ty) () ($expr(value)),
                min ($argty) () (min),
                max ($argty) () (max),
                format (&'static CStr) () (format.as_ptr()),
                flags (SliderFlags) () (flags.bits()),
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
                #[must_use]
                pub fn $func<$life, S: IntoCStr>(&mut self, label: S, value: $ty) -> $name<$life, &mut Self, S> {
                    $name {
                        u: self,
                        label: label.into(),
                        value,
                        min: <$argty>::default(),
                        max: <$argty>::default(),
                        format: cstr!("%.3f"),
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
        // TODO: doc says BufSize is read-only, but I think it should work
        data.BufSize = this.capacity() as i32;
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
        label (S::Temp) (let label = label.into()) (label.as_ptr()),
        text (&'v mut String) () (text),
        flags (InputTextFlags) () (flags),
    )
    {
        decl_builder_setter!{flags: InputTextFlags}
    }
    {
        #[must_use]
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
        label (S::Temp) (let label = label.into()) (label.as_ptr()),
        text (&'v mut String) () (text),
        size (ImVec2) () (&size),
        flags (InputTextFlags) () (flags),
    )
    {
        decl_builder_setter!{flags: InputTextFlags}
        decl_builder_setter!{size: ImVec2}
    }
    {
        #[must_use]
        pub fn do_input_text_multiline<'v, S: IntoCStr>(&mut self, label: S, text: &'v mut String) -> InputTextMultiline<'v, &mut Self, S> {
            InputTextMultiline {
                u:self,
                label:label.into(),
                text,
                flags: InputTextFlags::None,
                size: [0.0, 0.0].into(),
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
        label (S1::Temp) (let label = label.into()) (label.as_ptr()),
        hint (S2::Temp) (let hint = hint.into()) (hint.as_ptr()),
        text (&'v mut String) () (text),
        flags (InputTextFlags) () (flags),
    )
    {
        decl_builder_setter!{flags: InputTextFlags}
    }
    {
        #[must_use]
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

decl_builder! { InputFloat -> bool, ImGui_InputFloat ('v) (S: IntoCStr)
    (
        label (S::Temp) (let label = label.into()) (label.as_ptr()),
        value (&'v mut f32) () (value),
        step (f32) () (step),
        step_fast (f32) () (step_fast),
        format (&'static CStr) () (format.as_ptr()),
        flags (InputTextFlags) () (flags.bits()),
    )
    {
        decl_builder_setter!{flags: InputTextFlags}
        decl_builder_setter!{step: f32}
        decl_builder_setter!{step_fast: f32}
    }
    {
        #[must_use]
        pub fn do_input_float<'v, S: IntoCStr>(&mut self, label: S, value: &'v mut f32) -> InputFloat<'v, &mut Self, S> {
            InputFloat {
                u:self,
                label:label.into(),
                value,
                step: 0.0,
                step_fast: 0.0,
                format: cstr!("%.3f"),
                flags: InputTextFlags::None,
            }
        }
    }
}

macro_rules! decl_builder_input_f {
    ($name:ident $func:ident $cfunc:ident $len:literal) => {
        decl_builder! { $name -> bool, $cfunc ('v) (S: IntoCStr)
        (
            label (S::Temp) (let label = label.into()) (label.as_ptr()),
            value (&'v mut [f32; $len]) () (value.as_mut_ptr()),
            format (&'static CStr) () (format.as_ptr()),
            flags (InputTextFlags) () (flags.bits()),
        )
        {
            decl_builder_setter!{flags: InputTextFlags}
        }
        {
            #[must_use]
            pub fn $func<'v, S: IntoCStr>(&mut self, label: S, value: &'v mut [f32; $len]) -> $name<'v, &mut Self, S> {
                $name {
                    u:self,
                    label:label.into(),
                    value,
                    format: cstr!("%.3f"),
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

macro_rules! decl_builder_input_i {
    ($name:ident $func:ident $cfunc:ident $len:literal) => {
        decl_builder! { $name -> bool, $cfunc ('v) (S: IntoCStr)
        (
            label (S::Temp) (let label = label.into()) (label.as_ptr()),
            value (&'v mut [i32; $len]) () (value.as_mut_ptr()),
            flags (InputTextFlags) () (flags.bits()),
        )
        {
            decl_builder_setter!{flags: InputTextFlags}
        }
        {
            #[must_use]
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
    #[must_use]
    pub fn do_window<'s, S: IntoCStr>(&'s mut self, name: S, open: Option<&'s mut bool>, flags: WindowFlags) -> Window<&'s mut Self, S> {
        Window {
            u: self,
            name: name.into(),
            open,
            flags,
            push: (),
        }
    }
    pub fn set_next_window_size_constraints_callback(&mut self,
        size_min: &ImVec2,
        size_max: &ImVec2,
        cb: impl FnMut(&'ctx mut D, SizeCallbackData<'_>) + 'ctx,
    )
    {
        unsafe {
            let id = self.push_callback(cb);
            ImGui_SetNextWindowSizeConstraints(
                size_min,
                size_max,
                Some(call_size_callback::<D>),
                id as *mut c_void,
            );
        }
    }
    pub fn set_next_window_size_constraints(&mut self,
        size_min: &ImVec2,
        size_max: &ImVec2,
    )
    {
        unsafe {
            ImGui_SetNextWindowSizeConstraints(
                size_min,
                size_max,
                None,
                null_mut(),
            );
        }
    }
    pub fn styles(&mut self) -> &ImGuiStyle {
        unsafe {
            &*ImGui_GetStyle()
        }
    }
    #[must_use]
    pub fn do_child<'s, S: IntoCStr>(&'s mut self, name: S, size: &'s ImVec2, border: bool, flags: WindowFlags) -> Child<&'s mut Self, S> {
        Child {
            u: self,
            name: name.into(),
            size,
            border,
            flags,
            push: (),
        }
    }

    with_begin_end!{with_group ImGui_BeginGroup ImGui_EndGroup ()}
    with_begin_end_opt!{with_main_menu_bar ImGui_BeginMainMenuBar ImGui_EndMainMenuBar ()}
    with_begin_end_opt!{with_menu_bar ImGui_BeginMenuBar ImGui_EndMenuBar () }
    with_begin_end_opt!{with_menu ImGui_BeginMenu ImGui_EndMenu (
        name (impl IntoCStr) {let name = name.into();} (name.as_ptr()),
        enabled (bool) {} (enabled),
    )}
    with_begin_end_opt!{with_tooltip ImGui_BeginTooltip ImGui_EndTooltip () }
    with_begin_end_opt!{with_item_tooltip ImGui_BeginItemTooltip ImGui_EndTooltip () }
    with_begin_end!{with_disabled ImGui_BeginDisabled ImGui_EndDisabled (
        disabled (bool) {} (disabled),
    ) }
    with_begin_end!{with_clip_rect ImGui_PushClipRect ImGui_PopClipRect (
        clip_rect_min (&ImVec2) {} (clip_rect_min),
        clip_rect_max (&ImVec2) {} (clip_rect_max),
        intersect_with_current_clip_rect (bool) {} (intersect_with_current_clip_rect),
    ) }

    pub fn push<R>(&mut self, style: impl Pushable, f: impl FnOnce(&mut Self) -> R) -> R {
        let r;
        unsafe {
            style.push();
            r = f(self);
            style.pop();
        }
        r
    }
    pub fn pushes<'a>(&mut self, styles: impl AsRef<[&'a dyn Pushable]>, f: impl FnOnce(&mut Self)) {
        // &[&dyn Pushable] implements Pushable
        self.push(styles.as_ref(), f);
    }
    pub fn show_demo_window(&mut self, show: &mut bool) {
        unsafe {
            ImGui_ShowDemoWindow(show);
        }
    }
    pub fn set_next_window_pos(&mut self, pos: &ImVec2, cond: Cond, pivot: &ImVec2) {
        unsafe {
            ImGui_SetNextWindowPos(pos, cond.bits(), pivot);
        }
    }
    pub fn set_next_window_size(&mut self, size: &ImVec2, cond: Cond) {
        unsafe {
            ImGui_SetNextWindowSize(size, cond.bits());
        }
    }

    pub fn set_next_window_content_size(&mut self, size: &ImVec2) {
        unsafe {
            ImGui_SetNextWindowContentSize(size);
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

    pub fn set_next_window_scroll(&mut self, scroll: &ImVec2) {
        unsafe {
            ImGui_SetNextWindowScroll(scroll);
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
    pub fn text_colored(&mut self, color: Color, text: impl IntoCStr) {
        let text = text.into();
        unsafe {
            ImGui_TextColored(&color.color_vec(), cstr!("%s").as_ptr(), text.as_ptr())
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
    #[must_use]
    pub fn do_combo<S1: IntoCStr, S2: IntoCStr>(&mut self, label: S1, preview_value: S2) -> Combo<&mut Self, S1, S2> {
        Combo {
            u: self,
            label: label.into(),
            preview_value: preview_value.into(),
            flags: ComboFlags::None,
        }
    }
    // Helper functions, should it be here?
    pub fn combo(&mut self, label: impl IntoCStr, values: &[&str], selection: &mut usize) -> bool {
        let mut changed = false;
        self.do_combo(label, values[*selection]).with(|ui| {
            for (idx, value) in values.into_iter().enumerate() {
                let selected = idx == *selection;
                ui.push(ItemId(idx), |ui| {
                    if ui.do_selectable(*value).selected(selected).build() {
                        *selection = idx;
                        changed = true;
                    }
                });
                if selected {
                    ui.set_item_default_focus();
                }
            }
        });
        changed
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
    pub fn get_item_rect_min(&mut self) -> ImVec2 {
        unsafe {
            ImGui_GetItemRectMin()
        }
    }
    pub fn get_item_rect_max(&mut self) -> ImVec2 {
        unsafe {
            ImGui_GetItemRectMax()
        }
    }
    pub fn get_item_rect_size(&mut self) -> ImVec2 {
        unsafe {
            ImGui_GetItemRectSize()
        }
    }
    pub fn get_main_viewport<'s>(&'s mut self) -> Viewport<'s> {
        unsafe {
            Viewport {
                ptr: &*ImGui_GetMainViewport()
            }
        }
    }
}

pub struct Window<'a, U, S: IntoCStr, P: Pushable = ()> {
    u: U,
    name: S::Temp,
    open: Option<&'a mut bool>,
    flags: WindowFlags,
    push: P,
}

impl<'a, U, S: IntoCStr, P: Pushable> Window<'a, U, S, P> {
    pub fn push_for_begin<P2: Pushable>(self, push: P2) -> Window<'a, U, S, (P, P2)> {
        Window {
            u: self.u,
            name: self.name,
            open: self.open,
            flags: self.flags,
            push: (self.push, push),
        }
    }
    pub fn with<R>(self, f: impl FnOnce(U) -> R) -> Option<R> {
        let bres;
        unsafe {
            self.push.push();
            bres = ImGui_Begin(self.name.as_ptr(), self.open.map(|x| x as *mut bool).unwrap_or(null_mut()), self.flags.bits());
            self.push.pop();
        };
        let r = if bres {
            Some(f(self.u))
        } else {
            None
        };
        unsafe {
            ImGui_End();
        }
        r
    }
}

pub struct Child<'a, U, S: IntoCStr, P: Pushable = ()> {
    u: U,
    name: S::Temp,
    size: &'a ImVec2,
    border: bool,
    flags: WindowFlags,
    push: P,
}

impl <'a, U, S: IntoCStr, P: Pushable> Child<'a, U, S, P> {
    pub fn push_for_begin<P2: Pushable>(self, push: P2) -> Child<'a, U, S, (P, P2)> {
        Child {
            u: self.u,
            name: self.name,
            size: self.size,
            border: self.border,
            flags: self.flags,
            push: (self.push, push),
        }
    }
    pub fn with<R>(self, f: impl FnOnce(U) -> R) -> Option<R> {
        let bres;
        unsafe {
            self.push.push();
            bres = ImGui_BeginChild(self.name.as_ptr(), self.size, self.border, self.flags.bits());
            self.push.pop();
        };
        let r = if bres {
            Some(f(self.u))
        } else {
            None
        };
        unsafe {
            ImGui_EndChild();
        }
        r
    }
}

pub struct Combo<U, S1: IntoCStr, S2: IntoCStr> {
    u: U,
    label: S1::Temp,
    preview_value: S2::Temp,
    flags: ComboFlags,
}

impl<U, S1: IntoCStr, S2: IntoCStr> Combo<U, S1, S2> {
    pub fn flags(mut self, flags: ComboFlags) -> Self {
        self.flags = flags;
        self
    }
    pub fn with<R>(self, f: impl FnOnce(U) -> R) -> Option<R> {
        let bres = unsafe {
            ImGui_BeginCombo(self.label.as_ptr(), self.preview_value.as_ptr(), self.flags.bits())
        };
        if !bres {
            return None;
        }
        let r = f(self.u);
        unsafe {
            ImGui_EndCombo();
        }
        Some(r)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FontId(usize);

#[derive(Debug)]
pub struct SizeCallbackData<'a> {
    ptr: &'a mut ImGuiSizeCallbackData,
}

impl SizeCallbackData<'_> {
    pub fn pos(&self) -> ImVec2 {
        self.ptr.Pos
    }
    pub fn current_size(&self) -> ImVec2 {
        self.ptr.CurrentSize
    }
    pub fn desired_size(&self) -> ImVec2 {
        self.ptr.DesiredSize
    }
    pub fn set_desired_size(&mut self, sz: &ImVec2) {
        self.ptr.DesiredSize = *sz;
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

impl<'a, 'ctx, D> WindowDrawList<'a, 'ctx, D> {
    pub fn add_line(&mut self, p1: &ImVec2, p2: &ImVec2, color: Color, thickness: f32) {
        unsafe {
            ImDrawList_AddLine(self.ptr, p1, p2, color.color(), thickness);
        }
    }
    pub fn add_rect(&mut self, p_min: &ImVec2, p_max: &ImVec2, color: Color, rounding: f32, flags: DrawFlags, thickness: f32) {
        unsafe {
            ImDrawList_AddRect(self.ptr, p_min, p_max, color.color(), rounding, flags.bits(), thickness);
        }
    }
    pub fn add_rect_filled(&mut self, p_min: &ImVec2, p_max: &ImVec2, color: Color, rounding: f32, flags: DrawFlags) {
        unsafe {
            ImDrawList_AddRectFilled(self.ptr, p_min, p_max, color.color(), rounding, flags.bits());
        }
    }
    pub fn add_rect_filled_multicolor(&mut self, p_min: &ImVec2, p_max: &ImVec2, col_upr_left: Color, col_upr_right: Color, col_bot_right: Color, col_bot_left: Color) {
        unsafe {
            ImDrawList_AddRectFilledMultiColor(self.ptr, p_min, p_max, col_upr_left.color(), col_upr_right.color(), col_bot_right.color(), col_bot_left.color());
        }
    }
    pub fn add_quad(&mut self, p1: &ImVec2, p2: &ImVec2, p3: &ImVec2, p4: &ImVec2, color: Color, thickness: f32) {
        unsafe {
            ImDrawList_AddQuad(self.ptr, p1, p2, p3, p4, color.color(), thickness);
        }
    }
    pub fn add_quad_filled(&mut self, p1: &ImVec2, p2: &ImVec2, p3: &ImVec2, p4: &ImVec2, color: Color) {
        unsafe {
            ImDrawList_AddQuadFilled(self.ptr, p1, p2, p3, p4, color.color());
        }
    }
    pub fn add_triangle(&mut self, p1: &ImVec2, p2: &ImVec2, p3: &ImVec2, color: Color, thickness: f32) {
        unsafe {
            ImDrawList_AddTriangle(self.ptr, p1, p2, p3, color.color(), thickness);
        }
    }
    pub fn add_triangle_filled(&mut self, p1: &ImVec2, p2: &ImVec2, p3: &ImVec2, color: Color) {
        unsafe {
            ImDrawList_AddTriangleFilled(self.ptr, p1, p2, p3, color.color());
        }
    }
    pub fn add_circle(&mut self, center: &ImVec2, radius: f32, color: Color, num_segments: i32, thickness: f32) {
        unsafe {
            ImDrawList_AddCircle(self.ptr, center, radius, color.color(), num_segments, thickness);
        }
    }
    pub fn add_circle_filled(&mut self, center: &ImVec2, radius: f32, color: Color, num_segments: i32) {
        unsafe {
            ImDrawList_AddCircleFilled(self.ptr, center, radius, color.color(), num_segments);
        }
    }
    pub fn add_ngon(&mut self, center: &ImVec2, radius: f32, color: Color, num_segments: i32, thickness: f32) {
        unsafe {
            ImDrawList_AddNgon(self.ptr, center, radius, color.color(), num_segments, thickness);
        }
    }
    pub fn add_ngon_filled(&mut self, center: &ImVec2, radius: f32, color: Color, num_segments: i32) {
        unsafe {
            ImDrawList_AddNgonFilled(self.ptr, center, radius, color.color(), num_segments);
        }
    }
    pub fn add_text(&mut self, pos: &ImVec2, color: Color, text: &str) {
        unsafe {
            let (start, end) = text_ptrs(text);
            ImDrawList_AddText(self.ptr, pos, color.color(), start, end);
        }
    }
    pub fn add_text_ex(&mut self, font: FontId, font_size: f32, pos: &ImVec2, color: Color, text: &str, wrap_width: f32, cpu_fine_clip_rect: Option<ImVec4>) {
        unsafe {
            let (start, end) = text_ptrs(text);
            ImDrawList_AddText1(
                self.ptr, font_ptr(font), font_size, pos, color.color(), start, end,
                wrap_width, cpu_fine_clip_rect.as_ref().map(|x| x as *const _).unwrap_or(null())
            );
        }
    }
    pub fn add_polyline(&mut self, points: &[ImVec2], color: Color, flags: DrawFlags, thickness: f32) {
        unsafe {
            ImDrawList_AddPolyline(self.ptr, points.as_ptr(), points.len() as i32, color.color(), flags.bits(), thickness);
        }
    }
    pub fn add_convex_poly_filled(&mut self, points: &[ImVec2], color: Color) {
        unsafe {
            ImDrawList_AddConvexPolyFilled(self.ptr, points.as_ptr(), points.len() as i32, color.color());
        }
    }
    pub fn add_bezier_cubic(&mut self, p1: &ImVec2, p2: &ImVec2, p3: &ImVec2, p4: &ImVec2, color: Color, thickness: f32, num_segments: i32) {
        unsafe {
            ImDrawList_AddBezierCubic(self.ptr, p1, p2, p3, p4, color.color(), thickness, num_segments);
        }
    }
    pub fn add_bezier_quadratic(&mut self, p1: &ImVec2, p2: &ImVec2, p3: &ImVec2, color: Color, thickness: f32, num_segments: i32) {
        unsafe {
            ImDrawList_AddBezierQuadratic(self.ptr, p1, p2, p3, color.color(), thickness, num_segments);
        }
    }
    pub fn add_image(&mut self, user_texture_id: ImTextureID, p_min: &ImVec2, p_max: &ImVec2, uv_min: &ImVec2, uv_max: &ImVec2, color: Color) {
        unsafe {
            ImDrawList_AddImage(self.ptr, user_texture_id, p_min, p_max, uv_min, uv_max, color.color());
        }
    }
    pub fn add_image_quad(&mut self, user_texture_id: ImTextureID, p1: &ImVec2, p2: &ImVec2, p3: &ImVec2, p4: &ImVec2, uv1: &ImVec2, uv2: &ImVec2, uv3: &ImVec2, uv4: &ImVec2, color: Color) {
        unsafe {
            ImDrawList_AddImageQuad(self.ptr, user_texture_id, p1, p2, p3, p4, uv1, uv2, uv3, uv4, color.color());
        }
    }
    pub fn add_image_rounded(&mut self, user_texture_id: ImTextureID, p_min: &ImVec2, p_max: &ImVec2, uv_min: &ImVec2, uv_max: &ImVec2, color: Color, rounding: f32, flags: DrawFlags) {
        unsafe {
            ImDrawList_AddImageRounded(self.ptr, user_texture_id, p_min, p_max, uv_min, uv_max, color.color(), rounding, flags.bits());
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

//#[derive(Debug, Copy, Clone)]
//pub struct StyleColor(pub ColorId, pub Color);
pub type StyleColor = (ColorId, Color);

impl Pushable for StyleColor {
    unsafe fn push(&self) {
        ImGui_PushStyleColor(self.0.bits(), self.1.color());
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

#[derive(Debug, Copy, Clone)]
pub enum StyleValue {
    F32(f32),
    Vec2(ImVec2),
}

//#[derive(Debug, Copy, Clone)]
//pub struct Style(pub StyleVar, pub StyleValue);
pub type Style = (StyleVar, StyleValue);

impl Pushable for Style {
    unsafe fn push(&self) {
        match self.1 {
            StyleValue::F32(f) => ImGui_PushStyleVar(self.0.bits(), f),
            StyleValue::Vec2(v) => ImGui_PushStyleVar1(self.0.bits(), &v),
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
    pub fn pos(&self) -> &ImVec2 {
        &self.ptr.Pos
    }
    pub fn size(&self) -> &ImVec2 {
        &self.ptr.Size
    }
    pub fn work_pos(&self) -> &ImVec2 {
        &self.ptr.WorkPos
    }
    pub fn work_size(&self) -> &ImVec2 {
        &self.ptr.WorkSize
    }
}