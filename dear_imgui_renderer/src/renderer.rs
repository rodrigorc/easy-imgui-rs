use std::{ffi::{CString, c_void, c_char, CStr}, mem::size_of};

use dear_imgui_sys::*;
use clipboard::{ClipboardProvider, ClipboardContext};
use anyhow::{Result, anyhow};

use dear_imgui as imgui;
use crate::glr;

pub trait Application: imgui::UiBuilder {
    fn do_background(&mut self);
}

pub struct Renderer {
    imgui: imgui::Context,
    objs: GlObjects,
}

struct GlObjects {
    atlas: glr::Texture,
    program: glr::Program,
    vao: glr::VertexArray,
    vbuf: glr::Buffer,
    ibuf: glr::Buffer,
    a_pos_location: i32,
    a_uv_location: i32,
    a_color_location: i32,
    u_matrix_location: i32,
    u_tex_location: i32,
}

impl Renderer {
    pub fn new() -> Result<Renderer> {
        let tex;
        let program;
        let vao;
        let (vbuf, ibuf);
        let a_pos_location;
        let a_uv_location;
        let a_color_location;
        let u_matrix_location;
        let u_tex_location;

        let imgui = unsafe { imgui::Context::new() };

        unsafe {
            let io = &mut *ImGui_GetIO();
            io.BackendFlags |= (
                ImGuiBackendFlags_::ImGuiBackendFlags_RendererHasVtxOffset |
                ImGuiBackendFlags_::ImGuiBackendFlags_HasMouseCursors |
                ImGuiBackendFlags_::ImGuiBackendFlags_HasSetMousePos
            ).0 as ImGuiBackendFlags;

            if let Ok(ctx) = ClipboardContext::new() {
                let clip = MyClipboard {
                    ctx,
                    text: CString::default(),
                };
                io.ClipboardUserData = Box::into_raw(Box::new(clip)) as *mut c_void;
                io.SetClipboardTextFn = Some(set_clipboard_text);
                io.GetClipboardTextFn = Some(get_clipboard_text);
            }

            tex = glr::Texture::generate();

            program = gl_program_from_source(include_str!("shader.glsl"))?;
            vao = glr::VertexArray::generate();
            gl::BindVertexArray(vao.id());

            let a_pos = program.attrib_by_name("pos").unwrap();
            a_pos_location = a_pos.location();
            gl::EnableVertexAttribArray(a_pos_location as u32);

            let a_uv = program.attrib_by_name("uv").unwrap();
            a_uv_location = a_uv.location();
            gl::EnableVertexAttribArray(a_uv_location as u32);

            let a_color = program.attrib_by_name("color").unwrap();
            a_color_location = a_color.location();
            gl::EnableVertexAttribArray(a_color_location as u32);

            let u_matrix = program.uniform_by_name("matrix").unwrap();
            u_matrix_location = u_matrix.location();

            let u_tex = program.uniform_by_name("tex").unwrap();
            u_tex_location = u_tex.location();

            vbuf = glr::Buffer::generate();
            ibuf = glr::Buffer::generate();
        }
        Ok(Renderer {
            imgui,
            objs: GlObjects {
                atlas: tex,
                program,
                vao,
                vbuf,
                ibuf,
                a_pos_location,
                a_uv_location,
                a_color_location,
                u_matrix_location,
                u_tex_location,
            }
        })
    }
    pub fn imgui(&mut self) -> &mut imgui::Context {
        &mut self.imgui
    }
    /// size in logical units
    pub fn set_size(&mut self, size: ImVec2, scale: f32) {
        unsafe {
            self.imgui.set_current();
            self.imgui.set_size(size, scale);
        }
    }
    pub fn do_frame<A: Application>(&mut self, data: &mut A::Data, app: &mut A) {
        unsafe {
            self.imgui.set_current();
            self.update_atlas();
            self.imgui.do_frame(
                data,
                app,
                |app| {
                    let io = &*ImGui_GetIO();

                    gl::Viewport(
                        0, 0,
                        (io.DisplaySize.x * io.DisplayFramebufferScale.x) as i32,
                        (io.DisplaySize.y * io.DisplayFramebufferScale.y) as i32
                        );

                    app.do_background();
                    let draw_data = ImGui_GetDrawData();
                    Self::render(&self.objs, draw_data);
                }
            );
        }
    }
    unsafe fn update_atlas(&mut self) {
        if !self.imgui.update_atlas() {
            return;
        }

        let io = &mut *ImGui_GetIO();
        let mut data = std::ptr::null_mut();
        let mut width = 0;
        let mut height = 0;
        let mut pixel_size = 0;
        ImFontAtlas_GetTexDataAsAlpha8(io.Fonts, &mut data, &mut width, &mut height, &mut pixel_size);
        gl::BindTexture(gl::TEXTURE_2D, self.objs.atlas.id());

        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAX_LEVEL, 0);
        gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RED as i32,
                       width, height, 0,
                       gl::RED, gl::UNSIGNED_BYTE, data as *const _);
        gl::BindTexture(gl::TEXTURE_2D, 0);

        // bindgen: ImFontAtlas_SetTexID is inline
        (*io.Fonts).TexID = Self::map_tex(self.objs.atlas.id());

        // We keep this, no need for imgui to hold a copy
        ImFontAtlas_ClearTexData(io.Fonts);
        ImFontAtlas_ClearInputData(io.Fonts);
    }
    unsafe fn render(objs: &GlObjects, draw_data: *mut ImDrawData) {
        let draw_data = &*draw_data;

        gl::BindVertexArray(objs.vao.id());
        gl::UseProgram(objs.program.id());
        gl::BindBuffer(gl::ARRAY_BUFFER, objs.vbuf.id());
        gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, objs.ibuf.id());
        gl::Enable(gl::BLEND);
        gl::BlendFuncSeparate(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA, gl::ONE, gl::ONE_MINUS_SRC_ALPHA);
        gl::Disable(gl::CULL_FACE);
        gl::Disable(gl::DEPTH_TEST);
        gl::Enable(gl::SCISSOR_TEST);

        gl::ActiveTexture(gl::TEXTURE0);
        gl::Uniform1i(objs.u_tex_location, 0);

        let ImVec2 { x: left, y: top } = draw_data.DisplayPos;
        let ImVec2 { x: width, y: height } = draw_data.DisplaySize;
        let right = left + width;
        let bottom = top + height;
        gl::UniformMatrix3fv(objs.u_matrix_location, 1, gl::FALSE,
                             [
                             2.0 / width, 0.0, 0.0,
                             0.0, -2.0 / height, 0.0,
                             -(right + left) / width, (top + bottom) / height, 1.0,
                             ].as_ptr()
                            );

        for cmd_list in &draw_data.CmdLists {
            let cmd_list = &**cmd_list;

            gl::BufferData(
                gl::ARRAY_BUFFER,
                (size_of::<ImDrawVert>() * cmd_list.VtxBuffer.Size as usize) as isize,
                cmd_list.VtxBuffer.Data as *const _,
                gl::DYNAMIC_DRAW
                );
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (size_of::<ImDrawIdx>() * cmd_list.IdxBuffer.Size as usize) as isize,
                cmd_list.IdxBuffer.Data as *const _,
                gl::DYNAMIC_DRAW
                );
            #[allow(clippy::zero_ptr)]
            gl::VertexAttribPointer(
                objs.a_pos_location as u32,
                2 /*xy*/,
                gl::FLOAT,
                gl::FALSE,
                size_of::<ImDrawVert>() as i32,
                0 as *const _
                );
            gl::VertexAttribPointer(
                objs.a_uv_location as u32,
                2 /*xy*/,
                gl::FLOAT,
                gl::FALSE,
                size_of::<ImDrawVert>() as i32,
                8 as *const _
                );
            gl::VertexAttribPointer(
                objs.a_color_location as u32,
                4 /*rgba*/,
                gl::UNSIGNED_BYTE,
                gl::TRUE,
                size_of::<ImDrawVert>() as i32,
                16 as *const _
                );

            for cmd in &cmd_list.CmdBuffer {
                let clip_x = cmd.ClipRect.x - left;
                let clip_y = cmd.ClipRect.y - top;
                let clip_w = cmd.ClipRect.z - cmd.ClipRect.x;
                let clip_h = cmd.ClipRect.w - cmd.ClipRect.y;
                gl::Scissor(
                    (clip_x * draw_data.FramebufferScale.x) as i32,
                    ((height - (clip_y + clip_h)) * draw_data.FramebufferScale.y) as i32,
                    (clip_w * draw_data.FramebufferScale.x) as i32,
                    (clip_h * draw_data.FramebufferScale.y) as i32
                    );


                match cmd.UserCallback {
                    Some(cb) => {
                        cb(cmd_list, cmd);
                    }
                    None => {
                        gl::BindTexture(gl::TEXTURE_2D, Self::unmap_tex(cmd.TextureId));

                        gl::DrawElementsBaseVertex(
                            gl::TRIANGLES,
                            cmd.ElemCount as i32,
                            if size_of::<ImDrawIdx>() == 2 { gl::UNSIGNED_SHORT } else { gl::UNSIGNED_INT },
                            (size_of::<ImDrawIdx>() * cmd.IdxOffset as usize) as *const _,
                            cmd.VtxOffset as i32,
                            );
                    }
                }
            }

        }
        gl::UseProgram(0);
        gl::BindVertexArray(0);
        gl::Disable(gl::SCISSOR_TEST);
    }
    fn map_tex(ntex: u32) -> ImTextureID {
        ntex as ImTextureID
    }
    fn unmap_tex(tex: ImTextureID) -> u32 {
        tex as u32
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            let io = &mut *ImGui_GetIO();
            ImFontAtlas_Clear(io.Fonts);
        }
    }
}

pub fn gl_program_from_source(shaders: &str) -> Result<glr::Program> {
    let split = shaders.find("###").ok_or_else(|| anyhow!("shader marker not found"))?;
    let vertex = &shaders[.. split];
    let frag = &shaders[split ..];
    let split_2 = frag.find('\n').ok_or_else(|| anyhow!("shader marker not valid"))?;

    let mut frag = &frag[split_2 ..];

    let geom = if let Some(split) = frag.find("###") {
        let geom = &frag[split ..];
        frag = &frag[.. split];
        let split_2 = geom.find('\n').ok_or_else(|| anyhow!("shader marker not valid"))?;
        Some(&geom[split_2 ..])
    } else {
        None
    };

    let prg = glr::Program::from_source(vertex, frag, geom)?;
    Ok(prg)
}

unsafe extern "C" fn set_clipboard_text(user: *mut c_void, text: *const c_char) {
    let clip = &mut *(user as *mut MyClipboard);
    if text.is_null() {
        let _ = clip.ctx.set_contents(String::new());
    } else {
        let cstr = CStr::from_ptr(text);
        let str = String::from_utf8_lossy(cstr.to_bytes()).to_string();
        let _ = clip.ctx.set_contents(str);
    }
}

// The returned pointer should be valid for a while...
unsafe extern "C" fn get_clipboard_text(user: *mut c_void) -> *const c_char {
    let clip = &mut *(user as *mut MyClipboard);
    let Ok(text) = clip.ctx.get_contents() else {
        return std::ptr::null();
    };
    let Ok(text) = CString::new(text) else {
        return std::ptr::null();
    };
    clip.text = text;
    clip.text.as_ptr()
}

struct MyClipboard {
    ctx: ClipboardContext,
    text: CString,
}

