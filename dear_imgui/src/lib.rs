use std::ffi::{CString, c_char, CStr, c_void};
use std::ptr::{null, null_mut};
use std::mem::MaybeUninit;
use dear_imgui_sys::*;
use std::borrow::Cow;

pub type Cond = ImGuiCond_;

pub struct Context {
    _imgui: *mut ImGuiContext,
    pending_atlas: bool,
    fonts: Vec<FontInfo>,
}


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
    pub unsafe fn do_frame<'ctx, U>(
        &'ctx mut self,
        user_data: &'ctx mut U,
        do_ui: impl FnOnce(&mut Ui<'ctx, U>),
        do_render: impl FnOnce(),
    )
    {
        let mut ui = Ui {
            _ctx: self,
            user_data,
            callbacks: Vec::new(),
        };

        let io = &mut *ImGui_GetIO();
        io.BackendLanguageUserData = &mut ui as *mut Ui<U> as *mut c_void;
        ImGui_NewFrame();
        do_ui(&mut ui);
        ImGui_Render();
        do_render();
        io.BackendLanguageUserData = null_mut();
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

pub struct Ui<'ctx, U> {
    _ctx: &'ctx mut Context,
    user_data: &'ctx mut U,
    callbacks: Vec<Box<dyn FnMut(&'ctx mut U, *mut c_void) + 'ctx>>,
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

impl<'ctx, U: 'ctx> Ui<'ctx, U> {
    // The callback will be callable until the next call to do_frame()
    unsafe fn push_callback<A>(&mut self, mut cb: impl FnMut(&'ctx mut U, A) + 'ctx) -> usize {
        let cb = Box::new(move |user_data: &'ctx mut U, ptr: *mut c_void| {
            let a = ptr as *mut A;
            cb(user_data, unsafe { std::ptr::read(a) });
        });
        let id = self.callbacks.len();

        self.callbacks.push(cb);
        id
    }
    unsafe fn run_callback<A>(id: usize, a: A) {
        let io = &*ImGui_GetIO();
        let ui = &mut *(io.BackendLanguageUserData as *mut Self);

        // The lifetimes of ui have been erased, but it shouldn't matter
        let cb = &mut ui.callbacks[id];
        let mut a = MaybeUninit::new(a);
        cb(ui.user_data, a.as_mut_ptr() as *mut c_void);
    }
    pub fn user_data(&mut self) -> &mut U {
        self.user_data
    }
    pub fn with_window(&mut self, name: impl IntoCStr, open: Option<&mut bool>, flags: i32, f: impl FnOnce(&mut Self))
    {
        let name = name.into();
        let bres = unsafe {
            ImGui_Begin(name.as_ptr(), open.map(|x| x as *mut bool).unwrap_or(null_mut()), flags)
        };
        if bres {
            f(self);
        }
        unsafe {
            ImGui_End();
        }
    }
    pub fn set_next_window_size_constraints_callback(&mut self,
        size_min: impl Into<ImVec2>,
        size_max: impl Into<ImVec2>,
        cb: impl FnMut(&'ctx mut U, SizeCallbackData<'_>) + 'ctx,
    )
    {
        unsafe {
            let id = self.push_callback(cb);
            ImGui_SetNextWindowSizeConstraints(
                &size_min.into(),
                &size_max.into(),
                Some(call_size_callback::<U>),
                id as *mut c_void,
            );
        }
    }
    pub fn set_next_window_size_constraints(&mut self,
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
    pub fn with_child(&mut self, name: impl IntoCStr, size: impl Into<ImVec2>, border: bool, flags: i32, f: impl FnOnce(&mut Self)) {
        let name = name.into();
        let size = size.into();
        let bres = unsafe {
            ImGui_BeginChild(name.as_ptr(), &size, border, flags)
        };
        if bres {
            f(self);
        }
        unsafe {
            ImGui_EndChild();
        }
    }

    pub fn with_group(&mut self, f: impl FnOnce(&mut Self)) {
        unsafe { ImGui_BeginGroup(); }
        f(self);
        unsafe { ImGui_EndGroup(); }
    }
    pub fn with_font(&mut self, font: FontId, f: impl FnOnce(&mut Self)) {
        unsafe {
            ImGui_PushFont(font_ptr(font));
            f(self);
            ImGui_PopFont();
        }
    }

    pub fn show_demo_window(&mut self, show: &mut bool) {
        unsafe {
            ImGui_ShowDemoWindow(show);
        }
    }
    pub fn set_next_window_pos(&mut self, pos: impl Into<ImVec2>, cond: Cond, pivot: impl Into<ImVec2>) {
        unsafe {
            ImGui_SetNextWindowPos(&pos.into(), cond.0 as i32, &pivot.into());
        }
    }
    pub fn set_next_window_size(&mut self, size: impl Into<ImVec2>, cond: Cond) {
        unsafe {
            ImGui_SetNextWindowSize(&size.into(), cond.0 as i32);
        }
    }

    pub fn set_next_window_content_size(&mut self, size: impl Into<ImVec2>) {
        unsafe {
            ImGui_SetNextWindowContentSize(&size.into());
        }
    }

    pub fn set_next_window_collapsed(&mut self, collapsed: bool, cond: Cond) {
        unsafe {
           ImGui_SetNextWindowCollapsed(collapsed, cond.0 as i32);
        }
    }

    pub fn set_next_window_focus(&mut self) {
        unsafe {
           ImGui_SetNextWindowFocus();
        }
    }

    pub fn set_next_window_scroll(&mut self, scroll: impl Into<ImVec2>) {
        unsafe {
            ImGui_SetNextWindowScroll(&scroll.into());
        }
    }

    pub fn set_next_window_bg_alpha(&mut self, alpha: f32) {
        unsafe {
            ImGui_SetNextWindowBgAlpha(alpha);
        }
    }
    pub fn text_unformatted(&mut self, text: &str) {
        unsafe {
            let (start, end) = text_ptrs(text);
            ImGui_TextUnformatted(start, end);
        }
    }
    pub fn window_draw_list<'a>(&'a mut self) -> WindowDrawList<'a, 'ctx, U> {
        unsafe {
            let ptr = ImGui_GetWindowDrawList();
            WindowDrawList {
                ui: self,
                ptr: &mut *ptr,
            }
        }
    }
    pub fn foreground_draw_list<'a>(&'a mut self) -> WindowDrawList<'a, 'ctx, U> {
        unsafe {
            let ptr = ImGui_GetForegroundDrawList();
            WindowDrawList {
                ui: self,
                ptr: &mut *ptr,
            }
        }
    }
    pub fn background_draw_list<'a>(&'a mut self) -> WindowDrawList<'a, 'ctx, U> {
        unsafe {
            let ptr = ImGui_GetBackgroundDrawList();
            WindowDrawList {
                ui: self,
                ptr: &mut *ptr,
            }
        }
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
    pub fn set_desired_size(&mut self, sz: impl Into<ImVec2>) {
        self.ptr.DesiredSize = sz.into();
    }
}

unsafe extern "C" fn call_size_callback<U>(ptr: *mut ImGuiSizeCallbackData) {
    let ptr = &mut *ptr;
    let id = ptr.UserData as usize;
    let data = SizeCallbackData {
        ptr,
    };
    Ui::<U>::run_callback(id, data);
}

pub struct WindowDrawList<'a, 'ctx, U> {
    ui: &'a mut Ui<'ctx, U>,
    ptr: &'a mut ImDrawList,
}

impl<'a, 'ctx, U> WindowDrawList<'a, 'ctx, U> {
    /*
    void  AddLine(const ImVec2& p1, const ImVec2& p2, ImU32 col, float thickness = 1.0f);
    void  AddRect(const ImVec2& p_min, const ImVec2& p_max, ImU32 col, float rounding = 0.0f, ImDrawFlags flags = 0, float thickness = 1.0f);   // a: upper-left, b: lower-right (== upper-left + size)
    void  AddRectFilled(const ImVec2& p_min, const ImVec2& p_max, ImU32 col, float rounding = 0.0f, ImDrawFlags flags = 0);                     // a: upper-left, b: lower-right (== upper-left + size)
    void  AddRectFilledMultiColor(const ImVec2& p_min, const ImVec2& p_max, ImU32 col_upr_left, ImU32 col_upr_right, ImU32 col_bot_right, ImU32 col_bot_left);
    void  AddQuad(const ImVec2& p1, const ImVec2& p2, const ImVec2& p3, const ImVec2& p4, ImU32 col, float thickness = 1.0f);
    void  AddQuadFilled(const ImVec2& p1, const ImVec2& p2, const ImVec2& p3, const ImVec2& p4, ImU32 col);
    void  AddTriangle(const ImVec2& p1, const ImVec2& p2, const ImVec2& p3, ImU32 col, float thickness = 1.0f);
    void  AddTriangleFilled(const ImVec2& p1, const ImVec2& p2, const ImVec2& p3, ImU32 col);
    - void  AddCircle(const ImVec2& center, float radius, ImU32 col, int num_segments = 0, float thickness = 1.0f);
    - void  AddCircleFilled(const ImVec2& center, float radius, ImU32 col, int num_segments = 0);
    void  AddNgon(const ImVec2& center, float radius, ImU32 col, int num_segments, float thickness = 1.0f);
    void  AddNgonFilled(const ImVec2& center, float radius, ImU32 col, int num_segments);
    - void  AddText(const ImVec2& pos, ImU32 col, const char* text_begin, const char* text_end = NULL);
    - void  AddText(const ImFont* font, float font_size, const ImVec2& pos, ImU32 col, const char* text_begin, const char* text_end = NULL, float wrap_width = 0.0f, const ImVec4* cpu_fine_clip_rect = NULL);
    void  AddPolyline(const ImVec2* points, int num_points, ImU32 col, ImDrawFlags flags, float thickness);
    void  AddConvexPolyFilled(const ImVec2* points, int num_points, ImU32 col);
    void  AddBezierCubic(const ImVec2& p1, const ImVec2& p2, const ImVec2& p3, const ImVec2& p4, ImU32 col, float thickness, int num_segments = 0); // Cubic Bezier (4 control points)
    void  AddBezierQuadratic(const ImVec2& p1, const ImVec2& p2, const ImVec2& p3, ImU32 col, float thickness, int num_segments = 0);               // Quadratic Bezier (3 control points)

    void  AddImage(ImTextureID user_texture_id, const ImVec2& p_min, const ImVec2& p_max, const ImVec2& uv_min = ImVec2(0, 0), const ImVec2& uv_max = ImVec2(1, 1), ImU32 col = IM_COL32_WHITE);
    void  AddImageQuad(ImTextureID user_texture_id, const ImVec2& p1, const ImVec2& p2, const ImVec2& p3, const ImVec2& p4, const ImVec2& uv1 = ImVec2(0, 0), const ImVec2& uv2 = ImVec2(1, 0), const ImVec2& uv3 = ImVec2(1, 1), const ImVec2& uv4 = ImVec2(0, 1), ImU32 col = IM_COL32_WHITE);
    void  AddImageRounded(ImTextureID user_texture_id, const ImVec2& p_min, const ImVec2& p_max, const ImVec2& uv_min, const ImVec2& uv_max, ImU32 col, float rounding, ImDrawFlags flags = 0);

    - void  AddCallback(ImDrawCallback callback, void* callback_data);  // Your rendering function must check for 'UserCallback' in ImDrawCmd and call the function instead of rendering triangles.
    - void  AddDrawCmd();                                               // This is useful if you need to forcefully create a new draw call (to allow for dependent rendering / blending). Otherwise primitives are merged into the same draw-call as much as possible
    */
    pub fn add_circle(&mut self, center: impl Into<ImVec2>, radius: f32, color: impl IntoColor, num_segments: i32, thickness: f32) {
        unsafe {
            ImDrawList_AddCircle(self.ptr, &center.into(), radius, color.into(), num_segments, thickness);
        }
    }
    pub fn add_circle_filled(&mut self, center: impl Into<ImVec2>, radius: f32, color: impl IntoColor, num_segments: i32) {
        unsafe {
            ImDrawList_AddCircleFilled(self.ptr, &center.into(), radius, color.into(), num_segments);
        }
    }
    pub fn add_text(&mut self, pos: impl Into<ImVec2>, color: impl IntoColor, text: &str) {
        unsafe {
            let (start, end) = text_ptrs(text);
            ImDrawList_AddText(self.ptr, &pos.into(), color.into(), start, end);
        }
    }
    pub fn add_text_ex(&mut self, font: FontId, font_size: f32, pos: impl Into<ImVec2>, color: impl IntoColor, text: &str, wrap_width: f32, cpu_fine_clip_rect: Option<ImVec4>) {
        unsafe {
            let (start, end) = text_ptrs(text);
            ImDrawList_AddText1(
                self.ptr, font_ptr(font), font_size, &pos.into(), color.into(), start, end,
                wrap_width, cpu_fine_clip_rect.as_ref().map(|x| x as *const _).unwrap_or(null())
            );
        }
    }
    pub fn add_callback(&mut self, cb: impl FnOnce(&'ctx mut U) + 'ctx) {
        // Callbacks are only called once, convert the FnOnce into an FnMut to register
        let mut cb = Some(cb);
        unsafe {
            let id = self.ui.push_callback(move |u, _: ()| {
                if let Some(cb) = cb.take() {
                    cb(u);
                }
            });
            ImDrawList_AddCallback(self.ptr, Some(call_drawlist_callback::<U>), id as *mut c_void);
        }
    }
    pub fn add_draw_cmd(&mut self) {
        unsafe {
            ImDrawList_AddDrawCmd(self.ptr);
        }

    }
}

unsafe extern "C" fn call_drawlist_callback<U>(_parent_lilst: *const ImDrawList, cmd: *const ImDrawCmd) {
    let id = (*cmd).UserCallbackData as usize;
    Ui::<U>::run_callback(id, ());
}
