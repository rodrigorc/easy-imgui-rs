use std::{ffi::{CString, c_void, c_char, CStr}, mem::size_of};
use std::num::NonZeroU32;

use dear_imgui_sys::*;
use clipboard::{ClipboardProvider, ClipboardContext};
use anyhow::{Result, anyhow};
use glow::HasContext;

use dear_imgui as imgui;
use crate::glr;

pub trait Application: imgui::UiBuilder {
    fn do_background(&mut self);
}

pub struct Renderer {
    imgui: imgui::Context,
    gl: glr::GlContext,
    objs: GlObjects,
}

struct GlObjects {
    atlas: glr::Texture,
    program: glr::Program,
    vao: glr::VertexArray,
    vbuf: glr::Buffer,
    ibuf: glr::Buffer,
    a_pos_location: u32,
    a_uv_location: u32,
    a_color_location: u32,
    u_matrix_location: glow::UniformLocation,
    u_tex_location: glow::UniformLocation,
}

impl Renderer {
    pub fn new(gl: glr::GlContext) -> Result<Renderer> {
        let atlas;
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

            atlas = glr::Texture::generate(&gl)?;
            program = gl_program_from_source(&gl, include_str!("shader.glsl"))?;
            vao = glr::VertexArray::generate(&gl)?;
            gl.bind_vertex_array(Some(vao.id()));

            let a_pos = program.attrib_by_name("pos").unwrap();
            a_pos_location = a_pos.location();
            gl.enable_vertex_attrib_array(a_pos_location as u32);

            let a_uv = program.attrib_by_name("uv").unwrap();
            a_uv_location = a_uv.location();
            gl.enable_vertex_attrib_array(a_uv_location as u32);

            let a_color = program.attrib_by_name("color").unwrap();
            a_color_location = a_color.location();
            gl.enable_vertex_attrib_array(a_color_location as u32);

            let u_matrix = program.uniform_by_name("matrix").unwrap();
            u_matrix_location = u_matrix.location();

            let u_tex = program.uniform_by_name("tex").unwrap();
            u_tex_location = u_tex.location();

            vbuf = glr::Buffer::generate(&gl)?;
            ibuf = glr::Buffer::generate(&gl)?;
        }
        Ok(Renderer {
            imgui,
            gl,
            objs: GlObjects {
                atlas,
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

                    self.gl.viewport(
                        0, 0,
                        (io.DisplaySize.x * io.DisplayFramebufferScale.x) as i32,
                        (io.DisplaySize.y * io.DisplayFramebufferScale.y) as i32
                    );

                    app.do_background();
                    let draw_data = ImGui_GetDrawData();
                    Self::render(&self.gl, &self.objs, draw_data);
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
        self.gl.bind_texture(glow::TEXTURE_2D, Some(self.objs.atlas.id()));

        self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
        self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
        self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
        self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
        self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAX_LEVEL, 0);
        self.gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RED as i32,
                       width, height, 0,
                       glow::RED, glow::UNSIGNED_BYTE, Some(std::slice::from_raw_parts(data, (width * height * pixel_size) as usize)));
        self.gl.bind_texture(glow::TEXTURE_2D, None);

        // bindgen: ImFontAtlas_SetTexID is inline
        (*io.Fonts).TexID = Self::map_tex(self.objs.atlas.id());

        // We keep this, no need for imgui to hold a copy
        ImFontAtlas_ClearTexData(io.Fonts);
    }
    unsafe fn render(gl: &glow::Context, objs: &GlObjects, draw_data: *mut ImDrawData) {
        let draw_data = &*draw_data;

        gl.bind_vertex_array(Some(objs.vao.id()));
        gl.use_program(Some(objs.program.id()));
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(objs.vbuf.id()));
        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(objs.ibuf.id()));
        gl.enable(glow::BLEND);
        gl.blend_func_separate(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA, glow::ONE, glow::ONE_MINUS_SRC_ALPHA);
        gl.disable(glow::CULL_FACE);
        gl.disable(glow::DEPTH_TEST);
        gl.enable(glow::SCISSOR_TEST);

        gl.active_texture(glow::TEXTURE0);
        gl.uniform_1_i32(Some(&objs.u_tex_location), 0);

        let ImVec2 { x: left, y: top } = draw_data.DisplayPos;
        let ImVec2 { x: width, y: height } = draw_data.DisplaySize;
        let right = left + width;
        let bottom = top + height;
        gl.uniform_matrix_3_f32_slice(Some(&objs.u_matrix_location), false,
            &[
                2.0 / width, 0.0, 0.0,
                0.0, -2.0 / height, 0.0,
                -(right + left) / width, (top + bottom) / height, 1.0,
            ]
        );

        for cmd_list in &draw_data.CmdLists {
            let cmd_list = &**cmd_list;

            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                glr::as_u8_slice(&cmd_list.VtxBuffer),
                glow::DYNAMIC_DRAW
                );
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                glr::as_u8_slice(&cmd_list.IdxBuffer),
                glow::DYNAMIC_DRAW
                );
            let stride = size_of::<ImDrawVert>() as i32;
            gl.vertex_attrib_pointer_f32(
                objs.a_pos_location,
                2 /*xy*/,
                glow::FLOAT,
                false,
                stride,
                0
            );
            gl.vertex_attrib_pointer_f32(
                objs.a_uv_location as u32,
                2 /*xy*/,
                glow::FLOAT,
                false,
                stride,
                8,
            );
            gl.vertex_attrib_pointer_f32(
                objs.a_color_location as u32,
                4 /*rgba*/,
                glow::UNSIGNED_BYTE,
                true,
                stride,
                16,
            );

            for cmd in &cmd_list.CmdBuffer {
                let clip_x = cmd.ClipRect.x - left;
                let clip_y = cmd.ClipRect.y - top;
                let clip_w = cmd.ClipRect.z - cmd.ClipRect.x;
                let clip_h = cmd.ClipRect.w - cmd.ClipRect.y;
                gl.scissor(
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
                        gl.bind_texture(glow::TEXTURE_2D, Self::unmap_tex(cmd.TextureId));

                        gl.draw_elements_base_vertex(
                            glow::TRIANGLES,
                            cmd.ElemCount as i32,
                            if size_of::<ImDrawIdx>() == 2 { glow::UNSIGNED_SHORT } else { glow::UNSIGNED_INT },
                            (size_of::<ImDrawIdx>() * cmd.IdxOffset as usize) as i32,
                            cmd.VtxOffset as i32,
                        );
                    }
                }
            }

        }
        gl.use_program(None);
        gl.bind_vertex_array(None);
        gl.disable(glow::SCISSOR_TEST);
    }
    fn map_tex(ntex: glow::Texture) -> ImTextureID {
        ntex.0.get() as ImTextureID
    }
    fn unmap_tex(tex: ImTextureID) -> Option<glow::Texture> {
        Some(glow::NativeTexture(NonZeroU32::new(tex as u32)?))
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

pub fn gl_program_from_source(gl: &glr::GlContext, shaders: &str) -> Result<glr::Program> {
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

    let prg = glr::Program::from_source(gl, vertex, frag, geom)?;
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

