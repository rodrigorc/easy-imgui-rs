use std::mem::size_of;

use crate::glow::{self, HasContext};
use anyhow::{anyhow, Result};
use cgmath::{EuclideanSpace, Matrix3, Point2, Transform};
use easy_imgui::{self as imgui};
use easy_imgui_opengl as glr;
use easy_imgui_sys::*;
use imgui::{Color, TextureId, Vector2};

/// The main `Renderer` type.
pub struct Renderer {
    imgui: imgui::Context,
    gl: glr::GlContext,
    bg_color: Option<imgui::Color>,
    matrix: Option<Matrix3<f32>>,
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
    /// Creates a new renderer object.
    ///
    /// You need to provide the OpenGL context yourself.
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

        let mut imgui = unsafe { imgui::Context::new() };

        unsafe {
            if !cfg!(target_arch = "wasm32") {
                imgui.io_mut().inner().add_backend_flags(
                    imgui::BackendFlags::HasMouseCursors
                        | imgui::BackendFlags::HasSetMousePos
                        | imgui::BackendFlags::RendererHasVtxOffset,
                );
            }

            atlas = glr::Texture::generate(&gl)?;
            let glsl_version = if cfg!(not(target_arch = "wasm32")) {
                "#version 150\n"
            } else {
                "#version 300 es\n"
            };
            program = gl_program_from_source(&gl, Some(glsl_version), include_str!("shader.glsl"))?;
            vao = glr::VertexArray::generate(&gl)?;
            gl.bind_vertex_array(Some(vao.id()));

            let a_pos = program.attrib_by_name("pos").unwrap();
            a_pos_location = a_pos.location();
            gl.enable_vertex_attrib_array(a_pos_location);

            let a_uv = program.attrib_by_name("uv").unwrap();
            a_uv_location = a_uv.location();
            gl.enable_vertex_attrib_array(a_uv_location);

            let a_color = program.attrib_by_name("color").unwrap();
            a_color_location = a_color.location();
            gl.enable_vertex_attrib_array(a_color_location);

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
            bg_color: Some(Color::new(0.45, 0.55, 0.60, 1.0)),
            matrix: None,
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
            },
        })
    }
    /// Gets a reference to the OpenGL context.
    pub fn gl_context(&self) -> &glr::GlContext {
        &self.gl
    }
    /// Sets the default background color.
    ///
    /// The set color will be used for `glClear(GL_COLOR_BUFFER_BIT)`.
    /// Set to `None` to avoid this, and use [`easy_imgui::UiBuilder::pre_render`] to do whatever clearing
    /// you need, if anything.
    pub fn set_background_color(&mut self, color: Option<Color>) {
        self.bg_color = color;
    }
    /// Sets the 2D (3x3) matrix transformation for the UI display.
    ///
    /// If you set this matrix to `Some` then it is your responsibility to also call the appropriate `gl.viewport()`.
    pub fn set_matrix(&mut self, matrix: Option<Matrix3<f32>>) {
        self.matrix = matrix;
    }
    /// Gets the background color.
    pub fn background_color(&self) -> Option<Color> {
        self.bg_color
    }
    /// Gets the stored Dear ImGui context.
    pub fn imgui(&mut self) -> &mut imgui::Context {
        &mut self.imgui
    }
    /// Sets the UI size, in logical units, and the scale factor.
    pub fn set_size(&mut self, size: Vector2, scale: f32) {
        unsafe {
            self.imgui.set_size(size, scale);
        }
    }
    /// Gets the UI size, in logical units.
    pub fn size(&mut self) -> Vector2 {
        self.imgui.io().display_size()
    }
    /// Builds and renders a UI frame, using the `app` [`easy_imgui::UiBuilder`].
    pub fn do_frame<A: imgui::UiBuilder>(&mut self, app: &mut A) {
        unsafe {
            let mut imgui = self.imgui.set_current();

            if imgui.update_atlas(app) {
                Self::update_atlas(
                    imgui.io_mut().inner().font_atlas_mut(),
                    &self.gl,
                    &self.objs.atlas,
                );
            }
            imgui.do_frame(
                app,
                |ctx| {
                    let display_size = ctx.io().display_size();
                    let scale = ctx.io().display_scale();
                    if self.matrix.is_none() {
                        self.gl.viewport(
                            0,
                            0,
                            (display_size.x * scale) as i32,
                            (display_size.y * scale) as i32,
                        );
                    }
                    if let Some(bg) = self.bg_color {
                        self.gl.clear_color(bg.r, bg.g, bg.b, bg.a);
                        self.gl.clear(glow::COLOR_BUFFER_BIT);
                    }
                },
                |draw_data| {
                    Self::render(&self.gl, &self.objs, draw_data, self.matrix.as_ref());
                },
            );
        }
    }
    unsafe fn update_atlas(
        font_atlas: &mut ImFontAtlas,
        gl: &glr::GlContext,
        atlas_tex: &glr::Texture,
    ) {
        let mut data = std::ptr::null_mut();
        let mut width = 0;
        let mut height = 0;
        let mut pixel_size = 0;
        font_atlas.GetTexDataAsRGBA32(&mut data, &mut width, &mut height, &mut pixel_size);

        gl.bind_texture(glow::TEXTURE_2D, Some(atlas_tex.id()));

        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_S,
            glow::CLAMP_TO_EDGE as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_T,
            glow::CLAMP_TO_EDGE as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MIN_FILTER,
            glow::LINEAR as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MAG_FILTER,
            glow::LINEAR as i32,
        );
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAX_LEVEL, 0);
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGBA as i32, //glow::RED as i32,
            width,
            height,
            0,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            //glow::RED, glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(Some(std::slice::from_raw_parts(
                data,
                (width * height * pixel_size) as usize,
            ))),
        );
        gl.bind_texture(glow::TEXTURE_2D, None);

        // bindgen: ImFontAtlas_SetTexID is inline
        font_atlas.TexID = Self::map_tex(atlas_tex.id()).id();

        // We keep this, no need for imgui to hold a copy
        font_atlas.ClearTexData();
    }
    unsafe fn render(
        gl: &glow::Context,
        objs: &GlObjects,
        draw_data: &ImDrawData,
        matrix: Option<&Matrix3<f32>>,
    ) {
        enum ScissorViewportMatrix {
            Default,
            Custom(Matrix3<f32>),
            None,
        }
        let default_matrix;
        let (matrix, viewport_matrix) = match matrix {
            None => {
                let ImVec2 { x: left, y: top } = draw_data.DisplayPos;
                let ImVec2 {
                    x: width,
                    y: height,
                } = draw_data.DisplaySize;
                let right = left + width;
                let bottom = top + height;
                gl.enable(glow::SCISSOR_TEST);
                default_matrix = Matrix3::new(
                    2.0 / width,
                    0.0,
                    0.0,
                    0.0,
                    -2.0 / height,
                    0.0,
                    -(right + left) / width,
                    (top + bottom) / height,
                    1.0,
                );
                (&default_matrix, ScissorViewportMatrix::Default)
            }
            Some(matrix) => {
                // If there is a custom matrix we have to compute the scissor rectangle in viewport coordinates.
                // This only works if the transformed scissor rectangle is axis aligned, ie the rotation is 0°, 90°, 180° or 270°.
                // TODO: for other angles a fragment shader would be needed, maybe with a `discard`.
                // A rotation of multiple of 90° always has two 0 in the matrix:
                // * 0° and 180°: at (0,0) and (1,1), the sines.
                // * 90° and 270°: at (0,1) and (1,0), the cosines.
                if (matrix[0][0].abs() < f32::EPSILON && matrix[1][1].abs() < f32::EPSILON)
                    || (matrix[1][0].abs() < f32::EPSILON && matrix[0][1].abs() < f32::EPSILON)
                {
                    let mut viewport = [0; 4];
                    gl.get_parameter_i32_slice(glow::VIEWPORT, &mut viewport);
                    let viewport_x = viewport[0] as f32;
                    let viewport_y = viewport[1] as f32;
                    let viewport_w2 = viewport[2] as f32 / 2.0;
                    let viewport_h2 = viewport[3] as f32 / 2.0;
                    let vm = Matrix3::new(
                        viewport_w2,
                        0.0,
                        0.0,
                        0.0,
                        viewport_h2,
                        0.0,
                        viewport_x + viewport_w2,
                        viewport_y + viewport_h2,
                        1.0,
                    );
                    gl.enable(glow::SCISSOR_TEST);
                    (matrix, ScissorViewportMatrix::Custom(vm * matrix))
                } else {
                    gl.disable(glow::SCISSOR_TEST);
                    (matrix, ScissorViewportMatrix::None)
                }
            }
        };

        gl.bind_vertex_array(Some(objs.vao.id()));
        gl.use_program(Some(objs.program.id()));
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(objs.vbuf.id()));
        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(objs.ibuf.id()));
        gl.enable(glow::BLEND);
        gl.blend_func_separate(
            glow::SRC_ALPHA,
            glow::ONE_MINUS_SRC_ALPHA,
            glow::ONE,
            glow::ONE_MINUS_SRC_ALPHA,
        );
        gl.disable(glow::CULL_FACE);
        gl.disable(glow::DEPTH_TEST);

        gl.active_texture(glow::TEXTURE0);
        gl.uniform_1_i32(Some(&objs.u_tex_location), 0);

        gl.uniform_matrix_3_f32_slice(
            Some(&objs.u_matrix_location),
            false,
            AsRef::<[f32; 9]>::as_ref(matrix),
        );

        for cmd_list in &draw_data.CmdLists {
            let cmd_list = &**cmd_list;

            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                glr::as_u8_slice(&cmd_list.VtxBuffer),
                glow::DYNAMIC_DRAW,
            );
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                glr::as_u8_slice(&cmd_list.IdxBuffer),
                glow::DYNAMIC_DRAW,
            );
            let stride = size_of::<ImDrawVert>() as i32;
            gl.vertex_attrib_pointer_f32(
                objs.a_pos_location,
                2, /*xy*/
                glow::FLOAT,
                false,
                stride,
                0,
            );
            gl.vertex_attrib_pointer_f32(
                objs.a_uv_location,
                2, /*xy*/
                glow::FLOAT,
                false,
                stride,
                8,
            );
            gl.vertex_attrib_pointer_f32(
                objs.a_color_location,
                4, /*rgba*/
                glow::UNSIGNED_BYTE,
                true,
                stride,
                16,
            );

            for cmd in &cmd_list.CmdBuffer {
                match viewport_matrix {
                    ScissorViewportMatrix::Default => {
                        let clip_x = cmd.ClipRect.x - draw_data.DisplayPos.x;
                        let clip_y = cmd.ClipRect.y - draw_data.DisplayPos.y;
                        let clip_w = cmd.ClipRect.z - cmd.ClipRect.x;
                        let clip_h = cmd.ClipRect.w - cmd.ClipRect.y;
                        let scale = draw_data.FramebufferScale.x;
                        gl.scissor(
                            (clip_x * scale) as i32,
                            ((draw_data.DisplaySize.y - (clip_y + clip_h)) * scale) as i32,
                            (clip_w * scale) as i32,
                            (clip_h * scale) as i32,
                        );
                    }
                    ScissorViewportMatrix::Custom(vm) => {
                        let pos = Vector2::new(draw_data.DisplayPos.x, draw_data.DisplayPos.y);
                        let clip_aa = Vector2::new(cmd.ClipRect.x, cmd.ClipRect.y) - pos;
                        let clip_bb = Vector2::new(cmd.ClipRect.z, cmd.ClipRect.w) - pos;
                        let clip_aa = vm.transform_point(Point2::from_vec(clip_aa));
                        let clip_bb = vm.transform_point(Point2::from_vec(clip_bb));
                        gl.scissor(
                            clip_aa.x.min(clip_bb.x).round() as i32,
                            clip_aa.y.min(clip_bb.y).round() as i32,
                            (clip_bb.x - clip_aa.x).abs().round() as i32,
                            (clip_bb.y - clip_aa.y).abs().round() as i32,
                        );
                    }
                    ScissorViewportMatrix::None => {}
                }

                match cmd.UserCallback {
                    Some(cb) => {
                        cb(cmd_list, cmd);
                    }
                    None => {
                        gl.bind_texture(
                            glow::TEXTURE_2D,
                            Self::unmap_tex(TextureId::from_id(cmd.TextureId)),
                        );

                        if cfg!(target_arch = "wasm32") {
                            gl.draw_elements(
                                glow::TRIANGLES,
                                cmd.ElemCount as i32,
                                if size_of::<ImDrawIdx>() == 2 {
                                    glow::UNSIGNED_SHORT
                                } else {
                                    glow::UNSIGNED_INT
                                },
                                (size_of::<ImDrawIdx>() * cmd.IdxOffset as usize) as i32,
                            );
                        } else {
                            gl.draw_elements_base_vertex(
                                glow::TRIANGLES,
                                cmd.ElemCount as i32,
                                if size_of::<ImDrawIdx>() == 2 {
                                    glow::UNSIGNED_SHORT
                                } else {
                                    glow::UNSIGNED_INT
                                },
                                (size_of::<ImDrawIdx>() * cmd.IdxOffset as usize) as i32,
                                cmd.VtxOffset as i32,
                            );
                        }
                    }
                }
            }
        }
        gl.use_program(None);
        gl.bind_vertex_array(None);
        gl.disable(glow::SCISSOR_TEST);
    }
    /// Maps an OpenGL texture to an ImGui texture.
    pub fn map_tex(ntex: glow::Texture) -> TextureId {
        #[cfg(target_arch = "wasm32")]
        {
            let mut tex_map = WASM_TEX_MAP.lock().unwrap();
            let id = tex_map.len();
            tex_map.push(ntex);
            unsafe { TextureId::from_id(id as *mut std::ffi::c_void) }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            unsafe { TextureId::from_id(ntex.0.get() as ImTextureID) }
        }
    }
    /// Gets an OpenGL texture from an ImGui texture.
    pub fn unmap_tex(tex: TextureId) -> Option<glow::Texture> {
        #[cfg(target_arch = "wasm32")]
        {
            let tex_map = WASM_TEX_MAP.lock().unwrap();
            let id = tex.id() as usize;
            tex_map.get(id).cloned()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            Some(glow::NativeTexture(std::num::NonZeroU32::new(
                tex.id() as u32
            )?))
        }
    }
}

#[cfg(target_arch = "wasm32")]
static WASM_TEX_MAP: std::sync::Mutex<Vec<glow::Texture>> = std::sync::Mutex::new(Vec::new());

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            self.imgui().io_mut().inner().font_atlas_mut().Clear();
        }
    }
}

pub fn gl_program_from_source(
    gl: &glr::GlContext,
    prefix: Option<&str>,
    shaders: &str,
) -> Result<glr::Program> {
    let split = shaders
        .find("###")
        .ok_or_else(|| anyhow!("shader marker not found"))?;
    let vertex = &shaders[..split];
    let frag = &shaders[split..];
    let split_2 = frag
        .find('\n')
        .ok_or_else(|| anyhow!("shader marker not valid"))?;

    let mut frag = &frag[split_2 + 1..];

    let geom = if let Some(split) = frag.find("###") {
        let geom = &frag[split..];
        frag = &frag[..split];
        let split_2 = geom
            .find('\n')
            .ok_or_else(|| anyhow!("shader marker not valid"))?;
        Some(&geom[split_2 + 1..])
    } else {
        None
    };

    use std::borrow::Cow;

    let (vertex, frag, geom) = match prefix {
        None => (
            Cow::Borrowed(vertex),
            Cow::Borrowed(frag),
            geom.map(Cow::Borrowed),
        ),
        Some(prefix) => (
            Cow::Owned(format!("{prefix}{vertex}")),
            Cow::Owned(format!("{prefix}{frag}")),
            geom.map(|s| Cow::Owned(format!("{prefix}{s}"))),
        ),
    };
    let prg = glr::Program::from_source(gl, &vertex, &frag, geom.as_deref())?;
    Ok(prg)
}
