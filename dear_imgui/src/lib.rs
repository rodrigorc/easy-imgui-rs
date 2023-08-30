use std::ffi::{CString, c_char, CStr, c_void};
use std::ptr::{null, null_mut};
use std::mem::MaybeUninit;
use std::cell::UnsafeCell;
use dear_imgui_sys::*;
use std::borrow::Cow;

pub type Cond = ImGuiCond_;

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
    pub fn with_child(&mut self, name: impl IntoCStr, size: &ImVec2, border: bool, flags: i32, f: impl FnOnce(&mut Self)) {
        let name = name.into();
        let bres = unsafe {
            ImGui_BeginChild(name.as_ptr(), size, border, flags)
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
    pub fn set_next_window_pos(&mut self, pos: &ImVec2, cond: Cond, pivot: &ImVec2) {
        unsafe {
            ImGui_SetNextWindowPos(pos, cond.0 as i32, pivot);
        }
    }
    pub fn set_next_window_size(&mut self, size: &ImVec2, cond: Cond) {
        unsafe {
            ImGui_SetNextWindowSize(size, cond.0 as i32);
        }
    }

    pub fn set_next_window_content_size(&mut self, size: &ImVec2) {
        unsafe {
            ImGui_SetNextWindowContentSize(size);
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
    pub fn text_unformatted(&mut self, text: &str) {
        unsafe {
            let (start, end) = text_ptrs(text);
            ImGui_TextUnformatted(start, end);
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
    pub fn add_rect(&mut self, p_min: &ImVec2, p_max: &ImVec2, color: Color, rounding: f32, flags: ImDrawFlags_, thickness: f32) {
        unsafe {
            ImDrawList_AddRect(self.ptr, p_min, p_max, color.color(), rounding, flags.0 as i32, thickness);
        }
    }
    pub fn add_rect_filled(&mut self, p_min: &ImVec2, p_max: &ImVec2, color: Color, rounding: f32, flags: ImDrawFlags_) {
        unsafe {
            ImDrawList_AddRectFilled(self.ptr, p_min, p_max, color.color(), rounding, flags.0 as i32);
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
    pub fn add_polyline(&mut self, points: &[ImVec2], color: Color, flags: ImDrawFlags_, thickness: f32) {
        unsafe {
            ImDrawList_AddPolyline(self.ptr, points.as_ptr(), points.len() as i32, color.color(), flags.0 as i32, thickness);
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
    pub fn add_image_rounded(&mut self, user_texture_id: ImTextureID, p_min: &ImVec2, p_max: &ImVec2, uv_min: &ImVec2, uv_max: &ImVec2, color: Color, rounding: f32, flags: ImDrawFlags_) {
        unsafe {
            ImDrawList_AddImageRounded(self.ptr, user_texture_id, p_min, p_max, uv_min, uv_max, color.color(), rounding, flags.0 as i32);
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
