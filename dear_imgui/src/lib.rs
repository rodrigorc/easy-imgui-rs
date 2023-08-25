use std::ffi::{CString, c_char, CStr, c_void};
use std::ptr::{null, null_mut};
use std::mem::MaybeUninit;
use std::marker::PhantomData;
use dear_imgui_sys::*;
use std::borrow::Cow;
pub struct Context {
    _imgui: *mut ImGuiContext,
    pending_atlas: bool,
    fonts: Vec<FontInfo>,
}

pub struct UiHandle<'cb, 'ctx> {
    _ctx: &'ctx mut Context,
    callbacks: Vec<Box<dyn FnMut(*mut c_void) + 'cb>>,
}

impl<'cb, 'ctx> UiHandle<'cb, 'ctx> {
    // The callback will be callable until the next call to do_frame()
    unsafe fn push_callback<A>(&mut self, mut cb: impl FnMut(A) + 'cb) -> usize {
        let cb = Box::new(move |ptr: *mut c_void| {
            let a = ptr as *mut A;
            cb(unsafe { std::ptr::read(a) });
        });
        let id = self.callbacks.len();

        self.callbacks.push(cb);
        id
    }
    unsafe fn run_callback<A>(id: usize, a: A) {
        let io = &*ImGui_GetIO();
        let ui = &mut *(io.BackendLanguageUserData as *mut Self);

        // The lifetimes of ui have been erased, but it shouldn't matter
        let cb = &mut (*ui).callbacks[id];
        let mut a = MaybeUninit::new(a);
        cb(a.as_mut_ptr() as *mut c_void);
    }
}

// Ui must be invariant with respect to 'cb
pub type Ui<'cb, 'ctx, 'r> = &'r mut UiHandle<'cb, 'ctx>;

impl Context {
    pub fn new() -> Context {
        let imgui = unsafe {
            let imgui = ImGui_CreateContext(null_mut());

            let io = &mut *ImGui_GetIO();
            io.IniFilename = null();
            //TODO: clipboard should go here?
            //io.FontAllowUserScaling = true;
            //ImGui_StyleColorsDark(null_mut());
            imgui
        };
        Context {
            _imgui: imgui,
            pending_atlas: true,
            fonts: Vec::new(),
        }
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
    pub unsafe fn do_frame<'ctx, 'cb>(
        &'ctx mut self,
        do_ui: impl FnOnce(Ui<'cb, 'ctx, '_>),
        do_render: impl FnOnce(),
    )
    {
        let mut ui = UiHandle {
            _ctx: self,
            callbacks: Vec::new(),
        };

        let io = &mut *ImGui_GetIO();
        ImGui_NewFrame();
        io.BackendLanguageUserData = &mut ui as *mut UiHandle as *mut c_void;
        do_ui(&mut ui);
        io.BackendLanguageUserData = null_mut();
        ImGui_Render();
        do_render();
    }

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
    type Temp: std::ops::Deref<Target = CStr>;
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

pub fn with_window(ui: Ui, name: impl IntoCStr, open: Option<&mut bool>, flags: i32, f: impl FnOnce(Ui))
{
    let name = name.into();
    let bres = unsafe {
        ImGui_Begin(name.as_ptr(), open.map(|x| x as *mut bool).unwrap_or(null_mut()), flags)
    };
    if bres {
        f(ui);
    }
    unsafe {
        ImGui_End();
    }
}
pub fn with_child(ui: Ui, name: impl IntoCStr, size: impl Into<ImVec2>, border: bool, flags: i32, f: impl FnOnce(Ui)) {
    let name = name.into();
    let size = size.into();
    let bres = unsafe {
        ImGui_BeginChild(name.as_ptr(), &size, border, flags)
    };
    if bres {
        f(ui);
    }
    unsafe {
        ImGui_EndChild();
    }
}

pub fn with_group(ui: Ui, f: impl FnOnce(Ui)) {
    unsafe { ImGui_BeginGroup(); }
    f(ui);
    unsafe { ImGui_EndGroup(); }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FontId(usize);

pub fn with_font(ui: Ui, font: FontId, f: impl FnOnce(Ui)) {
    unsafe {
        let io = &*ImGui_GetIO();
        let fonts = &*io.Fonts;
        let font = fonts.Fonts[font.0];
        ImGui_PushFont(font);
    }
    f(ui);
    unsafe {
        ImGui_PopFont();
    }
}

pub fn show_demo_window(_ui: Ui, show: &mut bool) {
    unsafe {
        ImGui_ShowDemoWindow(show);
    }
}

pub fn set_next_window_pos(_ui: Ui, pos: impl Into<ImVec2>, cond: ImGuiCond_, pivot: impl Into<ImVec2>) {
    unsafe {
        ImGui_SetNextWindowPos(&pos.into(), cond.0 as i32, &pivot.into());
    }
}

pub fn set_next_window_size(_ui: Ui, size: impl Into<ImVec2>, cond: ImGuiCond_) {
    unsafe {
        ImGui_SetNextWindowSize(&size.into(), cond.0 as i32);
    }
}
pub fn set_next_window_size_constraints(_ui: Ui,
    size_min: impl Into<ImVec2>,
    size_max: impl Into<ImVec2>,
)
{
    unsafe {
        ImGui_SetNextWindowSizeConstraints(
            &size_min.into(),
            &size_max.into(),
            None,
            null_mut(),
        );
    }
}

pub fn set_next_window_content_size(_ui: Ui, size: impl Into<ImVec2>) {
    unsafe {
        ImGui_SetNextWindowContentSize(&size.into());
    }
}

pub fn set_next_window_collapsed(_ui: Ui, collapsed: bool, cond: ImGuiCond_) {
    unsafe {
       ImGui_SetNextWindowCollapsed(collapsed, cond.0 as i32);
    }
}

pub fn set_next_window_focus(_ui: Ui) {
    unsafe {
       ImGui_SetNextWindowFocus();
    }
}

pub fn set_next_window_scroll(_ui: Ui, scroll: impl Into<ImVec2>) {
    unsafe {
        ImGui_SetNextWindowScroll(&scroll.into());
    }
}

pub fn set_next_window_bg_alpha(_ui: Ui, alpha: f32) {
    unsafe {
        ImGui_SetNextWindowBgAlpha(alpha);
    }
}

pub struct SizeCallbackData<'a> {
    ptr: *mut ImGuiSizeCallbackData,
    _pd: PhantomData<*mut &'a()>,
}

impl SizeCallbackData<'_> {
    pub fn pos(&self) -> ImVec2 {
        unsafe { (*self.ptr).Pos }
    }
    pub fn current_size(&self) -> ImVec2 {
        unsafe { (*self.ptr).CurrentSize }
    }
    pub fn desired_size(&self) -> ImVec2 {
        unsafe { (*self.ptr).DesiredSize }
    }
    pub fn set_desired_size(&mut self, sz: impl Into<ImVec2>) {
        unsafe { (*self.ptr).DesiredSize = sz.into(); }
    }
}

impl std::fmt::Debug for SizeCallbackData<'_> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.debug_struct("SizeCallbackData")
            .field("pos", &self.pos())
            .field("curent_size", &self.current_size())
            .field("desired_size", &self.desired_size())
            .finish()
    }
}

pub fn set_next_window_size_constraints_cb<'cb>(ui: Ui<'cb, '_, '_>,
    size_min: impl Into<ImVec2>,
    size_max: impl Into<ImVec2>,
    cb: impl FnMut(SizeCallbackData<'_>) + 'cb,
)
{
    unsafe {
        let id = ui.push_callback(cb);
        ImGui_SetNextWindowSizeConstraints(
            &size_min.into(),
            &size_max.into(),
            Some(call_size_callback),
            id as *mut c_void,
        );
    }
}

unsafe extern "C" fn call_size_callback(data: *mut ImGuiSizeCallbackData) {
    let id = (*data).UserData as usize;
    let data = SizeCallbackData {
        ptr: data,
        _pd: PhantomData,
    };
    UiHandle::run_callback(id, data);
}

pub fn text_unformatted(_ui: Ui, txt: &str) {
    let btxt = txt.as_bytes();
    unsafe {
        let start = btxt.as_ptr();
        let end = start.add(btxt.len());
        ImGui_TextUnformatted(start as *const c_char, end as *const c_char);
    }
}
