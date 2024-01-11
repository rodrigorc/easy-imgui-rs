/*!
* Helper module to wrap calls to `glow` OpenGL functions.
*/
// Too many unsafes ahead
#![allow(clippy::missing_safety_doc)]

use std::{cell::Cell, marker::PhantomData};
use std::rc::Rc;

use glow::{HasContext, UniformLocation};
use smallvec::SmallVec;

#[derive(Debug, Clone)]
pub struct GLError(u32);

impl std::error::Error for GLError {
}
impl std::fmt::Display for GLError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:x}", self.0)
    }
}

pub type Result<T> = std::result::Result<T, GLError>;

/// A shareable OpenGL context.
pub type GlContext = Rc<glow::Context>;

pub fn check_gl(gl: &GlContext) -> std::result::Result<(), GLError> {
    let err = unsafe { gl.get_error() };
    if err == glow::NO_ERROR {
        Ok(())
    } else {
        Err(GLError(err))
    }
}

pub fn to_gl_err(gl: &GlContext) -> GLError {
    unsafe { GLError(gl.get_error()) }
}

pub struct Texture {
    gl: GlContext,
    id: glow::Texture,
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_texture(self.id);
        }
    }
}

impl Texture {
    pub fn gl(&self) -> &GlContext { &self.gl }
    pub fn generate(gl: &GlContext) -> Result<Texture> {
        unsafe {
            let id = gl.create_texture()
                .map_err(|_| to_gl_err(gl))?;
            Ok(Texture {
                gl: gl.clone(),
                id,
            })
        }
    }
    pub fn id(&self) -> glow::Texture {
        self.id
    }
    pub fn into_id(self) -> glow::Texture {
        let id = self.id;
        std::mem::forget(self);
        id
    }
}


pub struct EnablerVertexAttribArray {
    gl: GlContext,
    id: u32,
}

impl EnablerVertexAttribArray {
    fn enable(gl: &GlContext, id: u32) -> EnablerVertexAttribArray {
        unsafe {
            gl.enable_vertex_attrib_array(id);
        }
         EnablerVertexAttribArray {
            gl: gl.clone(),
            id,
         }
    }
}

impl Drop for EnablerVertexAttribArray {
    fn drop(&mut self) {
        unsafe {
            self.gl.disable_vertex_attrib_array(self.id);
        }
    }
}

pub struct PushViewport {
    gl: GlContext,
    prev: [i32; 4],
}

impl PushViewport {
    pub fn new(gl: &GlContext) -> PushViewport {
        unsafe {
            let mut prev = [0; 4];
            gl.get_parameter_i32_slice(glow::VIEWPORT, &mut prev);
            PushViewport {
                gl: gl.clone(),
                prev,
            }
        }
    }
    pub fn push(gl: &GlContext, x: i32, y: i32, width: i32, height: i32) -> PushViewport {
        let pv = Self::new(gl);
        pv.viewport(x, y, width, height);
        pv
    }
    pub fn viewport(&self, x: i32, y: i32, width: i32, height: i32) {
        unsafe {
            self.gl.viewport(x, y, width, height);
        }
    }
}

impl Drop for PushViewport {
    fn drop(&mut self) {
        unsafe {
            self.gl.viewport(self.prev[0], self.prev[1], self.prev[2], self.prev[3]);
        }
    }
}

pub struct Program {
    gl: GlContext,
    id: glow::Program,
    uniforms: Vec<Uniform>,
    attribs: Vec<Attribute>,
}

impl Drop for Program {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_program(self.id);
        }
    }
}

impl Program {
    pub fn from_source(gl: &GlContext, vertex: &str, fragment: &str, geometry: Option<&str>) -> Result<Program> {
        unsafe {
            // Purge error status
            gl.get_error();
            let vsh = Shader::compile(gl, glow::VERTEX_SHADER, vertex)?;
            let fsh = Shader::compile(gl, glow::FRAGMENT_SHADER, fragment)?;
            let gsh = match geometry {
                Some(source) => Some(Shader::compile(gl, glow::GEOMETRY_SHADER, source)?),
                None => None,
            };
            let id = gl.create_program()
                .map_err(|_| to_gl_err(gl))?;
            let mut prg = Program {
                gl: gl.clone(),
                id,
                uniforms: Vec::new(),
                attribs: Vec::new(),
            };
            gl.attach_shader(prg.id, vsh.id);
            gl.attach_shader(prg.id, fsh.id);
            if let Some(g) = gsh {
                gl.attach_shader(prg.id, g.id);
            }
            gl.link_program(prg.id);

            let st = gl.get_program_link_status(prg.id);
            if !st {
                let msg = gl.get_program_info_log(prg.id);
                log::error!("{msg}");
                return Err(GLError(gl.get_error()));
            }

            let nu = gl.get_active_uniforms(prg.id);
            prg.uniforms = Vec::with_capacity(nu as usize);
            for u in 0..nu {
                let Some(ac) = gl.get_active_uniform(prg.id, u) else { continue; };
                let Some(location) = gl.get_uniform_location(prg.id, &ac.name) else { continue; };

                let u = Uniform {
                    name: ac.name,
                    location,
                    _size: ac.size,
                    _type: ac.utype,
                };
                prg.uniforms.push(u);
            }
            let na = gl.get_active_attributes(prg.id);
            prg.attribs = Vec::with_capacity(na as usize);
            for a in 0..na {
                let Some(aa) = gl.get_active_attribute(prg.id, a) else { continue; };
                let Some(location) = gl.get_attrib_location(prg.id, &aa.name) else { continue; };

                let a = Attribute {
                    name: aa.name,
                    location,
                    _size: aa.size,
                    _type: aa.atype,
                };
                prg.attribs.push(a);
            }

            Ok(prg)
        }
    }
    pub fn id(&self) -> glow::Program {
        self.id
    }
    pub fn attrib_by_name(&self, name: &str) -> Option<&Attribute> {
        self.attribs.iter().find(|a| a.name == name)
    }
    pub fn uniform_by_name(&self, name: &str) -> Option<&Uniform> {
        self.uniforms.iter().find(|u| u.name == name)
    }
    pub fn draw<U, AS>(&self, uniforms: &U, attribs: AS, primitive: u32)
        where
            U: UniformProvider,
            AS: AttribProviderList,
    {
        if attribs.is_empty() {
            return;
        }
        unsafe {
            self.gl.use_program(Some(self.id));

            for u in &self.uniforms {
                uniforms.apply(&self.gl, u);
            }

            let _bufs = attribs.bind(self);
            self.gl.draw_arrays(primitive, 0, attribs.len() as i32);
            if let Err(e) = check_gl(&self.gl) {
                log::error!("Error {e:?}");
            }
        }
    }
}

struct Shader {
    gl: GlContext,
    id: glow::Shader,
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_shader(self.id);
        }
    }
}

impl Shader {
    fn compile(gl: &GlContext, ty: u32, source: &str) -> Result<Shader> {
        unsafe {
            let id = gl.create_shader(ty)
                .map_err(|_| to_gl_err(gl))?;
            let sh = Shader{
                gl: gl.clone(),
                id,
            };
            //multiline
            gl.shader_source(sh.id, source);
            gl.compile_shader(sh.id);
            let st = gl.get_shader_compile_status(sh.id);
            if !st {
                //TODO: get errors
                let msg = gl.get_shader_info_log(sh.id);
                log::error!("{msg}");
                return Err(GLError(gl.get_error()));
            }
            Ok(sh)
        }
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Rgba {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Rgba {
        Rgba { r, g, b, a }
    }
}

#[derive(Debug)]
pub struct Uniform {
    name: String,
    location: glow::UniformLocation,
    _size: i32,
    _type: u32,
}

impl Uniform {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn location(&self) -> glow::UniformLocation {
        // UniformLocation is usually Copy, but not for WebAssembly, for some reason
        #[allow(clippy::clone_on_copy)]
        self.location.clone()
    }
}

#[derive(Debug)]
pub struct Attribute {
    name: String,
    location: u32,
    _size: i32,
    _type: u32,
}

impl Attribute {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn location(&self) -> u32 {
        self.location
    }
}

pub trait UniformProvider {
    fn apply(&self, gl: &GlContext, u: &Uniform);
}

impl UniformProvider for () {
    fn apply(&self, _gl: &GlContext, _u: &Uniform) {
    }
}

/// # Safety
///
/// This trait returns offsets from Self that will be used to index the raw memory of a
/// VertexAttribBuffer. Better implemented using the `attrib!` macro.
pub unsafe trait AttribProvider: Copy {
    fn apply(gl: &GlContext, a: &Attribute) -> Option<(usize, u32, usize)>;
}

pub trait AttribProviderList {
    type KeepType;
    fn len(&self) -> usize;
    fn bind(&self, p: &Program) -> Self::KeepType;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// This vertex attrib provides the given count of vertices, but without data.
///
/// It is useful if all the vertex data is included in the shader and anly the gl_VertexID is
/// needed.
#[derive(Debug, Copy, Clone)]
pub struct NilVertexAttrib(pub usize);

impl AttribProviderList for NilVertexAttrib {
    type KeepType = ();
    fn len(&self) -> usize {
        self.0
    }
    fn bind(&self, _p: &Program) {
    }
}

/// Uses a normal array as attrib provider.
///
/// This is quite inefficient, but easy to use.
impl<A: AttribProvider> AttribProviderList for &[A] {
    type KeepType = (Buffer, SmallVec<[EnablerVertexAttribArray; 8]>);

    fn len(&self) -> usize {
        <[A]>::len(self)
    }
    fn bind(&self, p: &Program) -> (Buffer, SmallVec<[EnablerVertexAttribArray; 8]>) {
        let buf = Buffer::generate(&p.gl).unwrap();
        let mut vas = SmallVec::new();
        unsafe {
            p.gl.bind_buffer(glow::ARRAY_BUFFER, Some(buf.id()));
            p.gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, as_u8_slice(self), glow::STATIC_DRAW);
            for a in &p.attribs {
                if let Some((size, ty, offs)) = A::apply(&p.gl, a) {
                    let loc = a.location();
                    vas.push(EnablerVertexAttribArray::enable(&p.gl, loc));
                    p.gl.vertex_attrib_pointer_f32(loc, size as i32, ty, false, std::mem::size_of::<A>() as i32, offs as i32);
                }
            }
        }
        (buf, vas)
    }
}

/// # Safety
///
/// Returned information will be used to index the raw memory of a VertexAttribBuffer. Returning
/// wrong information will cause seg faults.
pub unsafe trait AttribField {
    fn detail() -> (usize, u32);
}

unsafe impl AttribField for f32 {
    fn detail() -> (usize, u32) {
        (1, glow::FLOAT)
    }
}
unsafe impl AttribField for u8 {
    fn detail() -> (usize, u32) {
        (1, glow::BYTE)
    }
}
unsafe impl AttribField for u32 {
    fn detail() -> (usize, u32) {
        (1, glow::UNSIGNED_INT)
    }
}
unsafe impl AttribField for i32 {
    fn detail() -> (usize, u32) {
        (1, glow::INT)
    }
}
unsafe impl AttribField for Rgba {
    fn detail() -> (usize, u32) {
        (4, glow::FLOAT)
    }
}
unsafe impl<F: AttribField, const N: usize> AttribField for [F; N] {
    fn detail() -> (usize, u32) {
        let (d, t) = F::detail();
        (N * d, t)
    }
}
unsafe impl<F: AttribField> AttribField for cgmath::Vector2<F> {
    fn detail() -> (usize, u32) {
        let (d, t) = F::detail();
        (2 * d, t)
    }
}
unsafe impl<F: AttribField> AttribField for cgmath::Vector3<F> {
    fn detail() -> (usize, u32) {
        let (d, t) = F::detail();
        (3 * d, t)
    }
}

/// # Safety
///
/// This trait returns pointers and size information to OpenGL, if it is wrong it will read out of bounds
pub unsafe trait UniformField {
    fn apply(&self, gl: &GlContext, location: UniformLocation) {
        unsafe {
            self.apply_array(gl, 1, location);
        }
    }
    unsafe fn apply_array(&self, gl: &GlContext, count: usize, location: UniformLocation);
}

unsafe impl UniformField for cgmath::Matrix4<f32> {
    unsafe fn apply_array(&self, gl: &GlContext, count: usize, location: UniformLocation) {
        unsafe {
            let slice: &[f32; 16] = self.as_ref();
            let slice = std::slice::from_raw_parts(slice.as_ptr(), slice.len() * count);
            gl.uniform_matrix_4_f32_slice(Some(&location), false, slice);
        }
    }
}

unsafe impl UniformField for cgmath::Matrix3<f32> {
    unsafe fn apply_array(&self, gl: &GlContext, count: usize, location: UniformLocation) {
        unsafe {
            let slice: &[f32; 9] = self.as_ref();
            let slice = std::slice::from_raw_parts(slice.as_ptr(), slice.len() * count);
            gl.uniform_matrix_3_f32_slice(Some(&location), false, slice);
        }
    }
}

unsafe impl UniformField for cgmath::Vector3<f32> {
    fn apply(&self, gl: &GlContext, location: UniformLocation) {
        unsafe {
            gl.uniform_3_f32(Some(&location), self.x, self.y, self.z);
        }
    }
    unsafe fn apply_array(&self, gl: &GlContext, count: usize, location: UniformLocation) {
        unsafe {
            let slice: &[f32; 3] = self.as_ref();
            let slice = std::slice::from_raw_parts(slice.as_ptr(), slice.len() * count);
            gl.uniform_3_f32_slice(Some(&location), slice);
        }
    }
}

unsafe impl UniformField for i32 {
    fn apply(&self, gl: &GlContext, location: UniformLocation) {
        unsafe {
            gl.uniform_1_i32(Some(&location), *self);
        }
    }
    unsafe fn apply_array(&self, gl: &GlContext, count: usize, location: UniformLocation) {
        unsafe {
            let slice = std::slice::from_raw_parts(self, count);
            gl.uniform_1_i32_slice(Some(&location), slice);
        }
    }
}

unsafe impl UniformField for f32 {
    fn apply(&self, gl: &GlContext, location: UniformLocation) {
        unsafe {
            gl.uniform_1_f32(Some(&location), *self);
        }
    }
    unsafe fn apply_array(&self, gl: &GlContext, count: usize, location: UniformLocation) {
        unsafe {
            let slice = std::slice::from_raw_parts(self, count);
            gl.uniform_1_f32_slice(Some(&location), slice);
        }
    }
}

unsafe impl UniformField for Rgba {
    fn apply(&self, gl: &GlContext, location: UniformLocation) {
        unsafe {
            gl.uniform_4_f32(Some(&location), self.r, self.g, self.b, self.a);
        }
    }
    unsafe fn apply_array(&self, gl: &GlContext, count: usize, location: UniformLocation) {
        unsafe {
            let slice = std::slice::from_raw_parts(&self.r, 4 * count);
            gl.uniform_4_f32_slice(Some(&location), slice);
        }
    }
}

unsafe impl<T: UniformField, const N: usize> UniformField for [T; N] {
    unsafe fn apply_array(&self, gl: &GlContext, count: usize, location: UniformLocation) {
        self[0].apply_array(gl, N * count, location);
    }
}

impl<A0: AttribProviderList, A1: AttribProviderList> AttribProviderList for (A0, A1) {
    type KeepType = (A0::KeepType, A1::KeepType);
    fn len(&self) -> usize {
        self.0.len().min(self.1.len())
    }
    fn bind(&self, p: &Program) -> (A0::KeepType, A1::KeepType) {
        let k0 = self.0.bind(p);
        let k1 = self.1.bind(p);
        (k0, k1)
    }
}

pub struct DynamicVertexArray<A> {
    data: Vec<A>,
    buf: Buffer,
    buf_len: Cell<usize>,
    dirty: Cell<bool>,
}

impl<A: AttribProvider> DynamicVertexArray<A> {
    pub fn new(gl: &GlContext) -> Result<Self> {
        Self::from_data(gl, Vec::new())
    }
    pub fn from_data(gl: &GlContext, data: Vec<A>) -> Result<Self> {
        Ok(DynamicVertexArray {
            data,
            buf: Buffer::generate(gl)?,
            buf_len: Cell::new(0),
            dirty: Cell::new(true),
        })
    }
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
    pub fn len(&self) -> usize {
        self.data.len()
    }
    pub fn set(&mut self, data: impl Into<Vec<A>>) {
        self.dirty.set(true);
        self.data = data.into();
    }
    pub fn data(&self) -> &[A] {
        &self.data[..]
    }
    pub fn sub(&self, range: std::ops::Range<usize>) -> DynamicVertexArraySub<'_, A> {
        DynamicVertexArraySub {
            array: self,
            range,
        }
    }
    pub fn bind_buffer(&self) {
        if self.data.is_empty() {
            return;
        }
        unsafe {
            self.buf.gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.buf.id()));
            if self.dirty.get() {
                if self.data.len() > self.buf_len.get() {
                    self.buf.gl.buffer_data_u8_slice(glow::ARRAY_BUFFER,
                        as_u8_slice(&self.data),
                        glow::DYNAMIC_DRAW
                    );
                    self.buf_len.set(self.data.len());
                } else {
                    self.buf.gl.buffer_sub_data_u8_slice(glow::ARRAY_BUFFER,
                        0,
                        as_u8_slice(&self.data)
                    );
                }
                self.dirty.set(false);
            }
        }
    }
}

impl<A: AttribProvider> std::ops::Index<usize> for DynamicVertexArray<A> {
    type Output = A;

    fn index(&self, index: usize) -> &A {
        &self.data[index]
    }
}

impl<A: AttribProvider> std::ops::IndexMut<usize> for DynamicVertexArray<A> {
    fn index_mut(&mut self, index: usize) -> &mut A {
        self.dirty.set(true);
        &mut self.data[index]
    }
}

impl<A: AttribProvider> AttribProviderList for &DynamicVertexArray<A> {
    type KeepType = SmallVec<[EnablerVertexAttribArray; 8]>;

    fn len(&self) -> usize {
        self.data.len()
    }

    fn bind(&self, p: &Program) -> SmallVec<[EnablerVertexAttribArray; 8]> {
        let mut vas = SmallVec::new();
        unsafe {
            self.bind_buffer();
            for a in &p.attribs {
                if let Some((size, ty, offs)) = A::apply(&p.gl, a) {
                    let loc = a.location();
                    vas.push(EnablerVertexAttribArray::enable(&p.gl, loc));
                    p.gl.vertex_attrib_pointer_f32(loc, size as i32, ty, false, std::mem::size_of::<A>() as i32, offs as i32);
                }
            }
        }
        vas
    }
}

pub struct DynamicVertexArraySub<'a, A> {
    array: &'a DynamicVertexArray<A>,
    range: std::ops::Range<usize>,
}

impl<A: AttribProvider> AttribProviderList for DynamicVertexArraySub<'_, A> {
    type KeepType = SmallVec<[EnablerVertexAttribArray; 8]>;

    fn len(&self) -> usize {
        self.range.len()
    }

    fn bind(&self, p: &Program) -> Self::KeepType {
        let mut vas = SmallVec::new();
        unsafe {
            self.array.bind_buffer();
            for a in &p.attribs {
                if let Some((size, ty, offs)) = A::apply(&p.gl, a) {
                    let loc = a.location();
                    vas.push(EnablerVertexAttribArray::enable(&p.gl, loc));
                    let offs = offs + std::mem::size_of::<A>() * self.range.start;
                    p.gl.vertex_attrib_pointer_f32(loc, size as i32, ty, false, std::mem::size_of::<A>() as i32, offs as i32);
                }
            }
        }
        vas

    }
}

pub struct Buffer {
    gl: GlContext,
    id: glow::Buffer,
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_buffer(self.id);
        }
    }
}

impl Buffer {
    pub fn generate(gl: &GlContext) -> Result<Buffer> {
        unsafe {
            let id = gl.create_buffer()
                .map_err(|_| to_gl_err(gl))?;
            Ok(Buffer {
                gl: gl.clone(),
                id,
            })
        }
    }
    pub fn id(&self) -> glow::Buffer {
        self.id
    }
}

pub struct VertexArray {
    gl: GlContext,
    id: glow::VertexArray,
}

impl Drop for VertexArray {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_vertex_array(self.id);
        }
    }
}

impl VertexArray {
    pub fn generate(gl: &GlContext) -> Result<VertexArray> {
        unsafe {
            let id = gl.create_vertex_array()
                .map_err(|_| to_gl_err(gl))?;
            Ok(VertexArray {
                gl: gl.clone(),
                id,
            })
        }
    }
    pub fn id(&self) -> glow::VertexArray {
        self.id
    }
}

pub struct Renderbuffer {
    gl: GlContext,
    id: glow::Renderbuffer,
}

impl Drop for Renderbuffer {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_renderbuffer(self.id);
        }
    }
}

impl Renderbuffer {
    pub fn generate(gl: &GlContext) -> Result<Renderbuffer> {
        unsafe {
            let id = gl.create_renderbuffer()
                .map_err(|_| to_gl_err(gl))?;
            Ok(Renderbuffer {
                gl: gl.clone(),
                id,
            })
        }
    }
    pub fn id(&self) -> glow::Renderbuffer {
        self.id
    }
}

pub struct BinderRenderbuffer(GlContext);

impl BinderRenderbuffer {
    pub fn bind(rb: &Renderbuffer) -> BinderRenderbuffer {
        unsafe {
            rb.gl.bind_renderbuffer(glow::RENDERBUFFER, Some(rb.id));
        }
        BinderRenderbuffer(rb.gl.clone())
    }
    pub fn target(&self) -> u32 {
        glow::RENDERBUFFER
    }
    pub fn rebind(&self, rb: &Renderbuffer) {
        unsafe {
            rb.gl.bind_renderbuffer(glow::RENDERBUFFER, Some(rb.id));
        }
    }
}
impl Drop for BinderRenderbuffer {
    fn drop(&mut self) {
        unsafe {
            self.0.bind_renderbuffer(glow::RENDERBUFFER, None);
        }
    }
}

pub struct Framebuffer {
    gl: GlContext,
    id: glow::Framebuffer,
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_framebuffer(self.id);
        }
    }
}

impl Framebuffer {
    pub fn generate(gl: &GlContext) -> Result<Framebuffer> {
        unsafe {
            let id = gl.create_framebuffer()
                .map_err(|_| to_gl_err(gl))?;
            Ok(Framebuffer {
                gl: gl.clone(),
                id
            })
        }
    }
    pub fn id(&self) -> glow::Framebuffer {
        self.id
    }
}


pub trait BinderFBOTarget {
    const TARGET: u32;
    const GET_BINDING: u32;
}

pub struct BinderFramebuffer<TGT: BinderFBOTarget> {
    gl: GlContext,
    id: Option<glow::Framebuffer>,
    _pd: PhantomData<TGT>,
}

impl<TGT: BinderFBOTarget> BinderFramebuffer<TGT> {
    pub fn new(gl: &GlContext) -> Self {
        #[cfg(not(target_arch="wasm32"))]
        let id = unsafe {
            let id = gl.get_parameter_i32(TGT::GET_BINDING) as u32;
            std::num::NonZeroU32::new(id).map(glow::NativeFramebuffer)
        };
        #[cfg(target_arch="wasm32")]
        let id = None;
        BinderFramebuffer {
            gl: gl.clone(),
            id,
            _pd: PhantomData
        }
    }
    pub fn target(&self) -> u32 {
        TGT::TARGET
    }
    pub fn bind(fb: &Framebuffer) -> Self {
        unsafe {
            fb.gl.bind_framebuffer(TGT::TARGET, Some(fb.id));
        }
        BinderFramebuffer {
            gl: fb.gl.clone(),
            id: None,
            _pd: PhantomData
        }
    }
    pub fn rebind(&self, fb: &Framebuffer) {
        unsafe {
            fb.gl.bind_framebuffer(TGT::TARGET, Some(fb.id));
        }
    }
}

impl<TGT: BinderFBOTarget> Drop for BinderFramebuffer<TGT> {
    fn drop(&mut self) {
        unsafe {
            self.gl.bind_framebuffer(TGT::TARGET, self.id);
        }
    }
}

pub struct BinderFBODraw;

impl BinderFBOTarget for BinderFBODraw {
    const TARGET: u32 = glow::DRAW_FRAMEBUFFER;
    const GET_BINDING: u32 = glow::DRAW_FRAMEBUFFER_BINDING;
}

pub type BinderDrawFramebuffer = BinderFramebuffer<BinderFBODraw>;

pub struct BinderFBORead;

impl BinderFBOTarget for BinderFBORead {
    const TARGET: u32 = glow::READ_FRAMEBUFFER;
    const GET_BINDING: u32 = glow::READ_FRAMEBUFFER_BINDING;
}

pub type BinderReadFramebuffer = BinderFramebuffer<BinderFBORead>;

pub unsafe fn as_u8_slice<T>(data: &[T]) -> &[u8] {
    std::slice::from_raw_parts(data.as_ptr() as *const u8, std::mem::size_of_val(data))
}
