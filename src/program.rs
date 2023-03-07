use std::{
    borrow::Cow,
    ffi::CString,
    fmt::{self, Debug, Formatter},
    num::NonZeroU32,
    path::Path,
};
use std::marker::PhantomData;

use duplicate::duplicate_item as duplicate;
use either::Either;
use eyre::{Context, Result};
use gl::types::{GLdouble, GLenum, GLfloat, GLint, GLuint};

use crate::{
    base::{
        GlType,
        resource::{Resource, ResourceExt},
    },
    buffer::BufferSlice,
    shader::{FragmentShader, GeometryShader, ShaderId, VertexShader},
    utils::{gl_error_guard, gl_string},
};

/// Trait of types that can be written into shader uniforms. This allows polymorphic use of the
/// methods on [`ActiveProgram`](struct::ActiveProgram);
pub trait Uniform {
    unsafe fn write_uniform(&self, location: GLint);
}

#[duplicate(
gl_t            uniform;
[GLint]         [Uniform1i];
[GLuint]        [Uniform1ui];
[GLfloat]       [Uniform1f];
[GLdouble]      [Uniform1d];
)]
impl Uniform for gl_t {
    unsafe fn write_uniform(&self, location: GLint) {
        gl::uniform(location, *self)
    }
}

#[duplicate(
gl_t        uniform;
[GLint]     [Uniform2i];
[GLuint]    [Uniform2ui];
[GLfloat]   [Uniform2f];
[GLdouble]  [Uniform2d];
)]
impl Uniform for [gl_t; 2] {
    unsafe fn write_uniform(&self, location: GLint) {
        let [x, y] = *self;
        gl::uniform(location, x, y);
    }
}

#[duplicate(
gl_t        uniform;
[GLint]     [Uniform3i];
[GLuint]    [Uniform3ui];
[GLfloat]   [Uniform3f];
[GLdouble]  [Uniform3d];
)]
impl Uniform for [gl_t; 3] {
    unsafe fn write_uniform(&self, location: GLint) {
        let [x, y, z] = *self;
        gl::uniform(location, x, y, z);
    }
}

#[duplicate(
gl_t        uniform;
[GLint]     [Uniform4i];
[GLuint]    [Uniform4ui];
[GLfloat]   [Uniform4f];
[GLdouble]  [Uniform4d];
)]
impl Uniform for [gl_t; 4] {
    unsafe fn write_uniform(&self, location: GLint) {
        let [x, y, z, w] = *self;
        gl::uniform(location, x, y, z, w);
    }
}

#[duplicate(
gl_t        uniform;
[GLfloat]   [UniformMatrix2fv];
[GLdouble]  [UniformMatrix2dv];
)]
impl Uniform for [[gl_t; 2]; 2] {
    unsafe fn write_uniform(&self, location: GLint) {
        gl::uniform(location, 1, gl::FALSE as _, self.as_ptr() as *const _);
    }
}

#[duplicate(
gl_t        uniform;
[GLfloat]   [UniformMatrix3fv];
[GLdouble]  [UniformMatrix3dv];
)]
impl Uniform for [[gl_t; 3]; 3] {
    unsafe fn write_uniform(&self, location: GLint) {
        gl::uniform(location, 1, gl::FALSE as _, self.as_ptr() as *const _);
    }
}

#[duplicate(
gl_t        uniform;
[GLfloat]   [UniformMatrix4fv];
[GLdouble]  [UniformMatrix4dv];
)]
impl Uniform for [[gl_t; 4]; 4] {
    unsafe fn write_uniform(&self, location: GLint) {
        gl::uniform(location, 1, gl::FALSE as _, self.as_ptr() as *const _);
    }
}

#[cfg(feature = "uniforms-glam")]
#[duplicate(
glam_t;
[glam::Vec2];
[glam::DVec2];
[glam::Vec3];
[glam::Vec3A];
[glam::DVec3];
[glam::Vec4];
[glam::DVec4];
)]
impl Uniform for glam_t {
    unsafe fn write_uniform(&self, location: GLint) {
        self.to_array().write_uniform(location);
    }
}

#[cfg(feature = "uniforms-glam")]
#[duplicate(
glam_t;
[glam::Mat2];
[glam::Mat3];
[glam::Mat4];
[glam::DMat2];
[glam::DMat3];
[glam::DMat4];
)]
impl Uniform for glam_t {
    unsafe fn write_uniform(&self, location: GLint) {
        self.to_cols_array_2d().write_uniform(location);
    }
}

impl<L: Uniform, R: Uniform> Uniform for Either<L, R> {
    unsafe fn write_uniform(&self, location: GLint) {
        match self {
            Self::Left(left) => left.write_uniform(location),
            Self::Right(right) => right.write_uniform(location),
        }
    }
}

impl<T: Uniform> Uniform for Option<T> {
    unsafe fn write_uniform(&self, location: GLint) {
        if let Some(inner) = self {
            inner.write_uniform(location)
        }
    }
}

#[derive(Debug, Clone, Copy)]
/// Structure allowing uniforms to be written into a program.
pub struct UniformLocation {
    program: ProgramId,
    location: u32,
    desc: UniformDesc,
}

impl UniformLocation {
    pub fn is_in_program(&self, program: &Program) -> bool {
        self.program == program.id
    }

    pub fn desc(&self) -> &UniformDesc {
        &self.desc
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UniformBlockIndex {
    program: ProgramId,
    binding: u32,
    block_index: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
/// Program ID newtype. Guaranteed to be non-zero if it exists. Allows `Option<ProgramId>` to coerce
/// into a single `u32` into memory.
pub struct ProgramId(pub(crate) NonZeroU32);

impl fmt::Display for ProgramId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.get())
    }
}

impl ProgramId {
    pub fn new(id: GLuint) -> Option<Self> {
        NonZeroU32::new(id).map(Self)
    }
}

impl std::ops::Deref for ProgramId {
    type Target = NonZeroU32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
/// Unlinked program typestate. Unlinked programs can have shaders added to them. Linking an
/// unlinked program produces a `Program` structure.
pub struct Unlinked;

#[derive(Debug)]
/// Linked program. A linked program can be used, and uniforms set.
pub struct Linked;

#[derive(Debug)]
/// A shader program. Linkage status is tracked at compile-time.
pub struct Program<Status = Linked> {
    __status: Status,
    __non_send: PhantomData<*mut ()>,
    pub id: ProgramId,
}

impl<Status> Drop for Program<Status> {
    fn drop(&mut self) {
        tracing::trace!("glDeleteProgram({})", self.id.get());
        unsafe {
            gl::DeleteProgram(self.id.get());
        }
    }
}

impl<Status: Debug> Program<Status> {
    #[tracing::instrument]
    pub fn validate(&self) -> Result<()> {
        tracing::trace!("glValidateProgram({})", self.id.get());
        let is_valid = unsafe {
            gl::ValidateProgram(self.id.get());
            let mut status = 0;
            gl::GetProgramiv(self.id.get(), gl::VALIDATE_STATUS, &mut status);
            status == gl::TRUE as _
        };
        if !is_valid {
            let length = unsafe {
                let mut length = 0;
                gl::GetProgramiv(self.id.get(), gl::INFO_LOG_LENGTH, &mut length);
                length as _
            };
            let error = gl_string(Some(length), |len, ptr_len, ptr| unsafe {
                gl::GetProgramInfoLog(self.id.get(), len as _, ptr_len, ptr);
            });
            eyre::bail!(error);
        }
        Ok(())
    }
}

#[allow(clippy::new_without_default)]
impl Program<Unlinked> {
    /// Create a new, empty program.
    pub fn new() -> Self {
        let id = unsafe { gl::CreateProgram() };
        Self {
            id: ProgramId(NonZeroU32::new(id).unwrap()),
            __non_send: PhantomData,
            __status: Unlinked,
        }
    }

    ///Add a compiled shader into the current program.
    pub fn add_shader<const K: u32>(&mut self, id: ShaderId<K>) {
        tracing::trace!("glAttachShader({}, {})", self.id.get(), id.get());
        unsafe { gl::AttachShader(self.id.get(), id.get()) };
    }

    ///Add a compiled shader into the current program.
    pub fn with_shader<const K: u32>(mut self, id: ShaderId<K>) -> Self {
        self.add_shader(id);
        self
    }

    /// Link the program.
    pub fn link(self) -> Result<Program> {
        let id = self.id.get();
        // Forget `self` to prevent running its destructor and call `glDeleteShader`.
        std::mem::forget(self);

        let is_success = unsafe {
            gl::LinkProgram(id);
            let mut success = 0;
            gl::GetProgramiv(id, gl::LINK_STATUS, &mut success);
            success == gl::TRUE as _
        };
        tracing::trace!("glLinkProgram({}) -> success: {}", id, is_success);
        if is_success {
            Ok(Program {
                id: ProgramId::new(id).unwrap(),
                __non_send: PhantomData,
                __status: Linked,
            })
        } else {
            let error = unsafe {
                let mut length = 0;
                gl::GetProgramiv(id, gl::INFO_LOG_LENGTH, &mut length);
                gl_string(Some(length as _), |len, len_ptr, ptr| {
                    gl::GetProgramInfoLog(id, len as _, len_ptr, ptr)
                })
            };
            eyre::bail!(error);
        }
    }
}

impl<'a> Resource<'a> for Program {
    type Id = ProgramId;

    fn id(&self) -> Self::Id {
        self.id
    }

    fn current() -> Option<Self::Id> {
        let mut id = 0;
        unsafe { gl::GetIntegerv(gl::CURRENT_PROGRAM, &mut id) }
        Self::Id::new(id as _)
    }

    fn bind(&self) {
        unsafe { gl::UseProgram(self.id.get() as _) }
    }

    fn unbind(&self) {
        unsafe { gl::UseProgram(0) }
    }
}

impl Program<Linked> {
    /// Load sources and create program from paths to a vertex, optional fragment and optional geometry shaders.
    pub fn from_sources<'vs, 'fs, 'gs>(
        vertex_shader: &'vs str,
        fragment_shader: impl Into<Option<&'fs str>>,
        geometry_shader: impl Into<Option<&'gs str>>,
    ) -> Result<Self> {
        let vertex = VertexShader::new(vertex_shader).context("Cannot parse vertex shader")?;
        let fragment = if let Some(source) = fragment_shader.into() {
            Some(FragmentShader::new(source).context("Cannot parse fragment shader")?)
        } else {
            None
        };
        let geometry = if let Some(source) = geometry_shader.into() {
            Some(GeometryShader::new(source).context("Cannot parse geometry shader")?)
        } else {
            None
        };
        let mut program = Program::new();
        program.add_shader(vertex.id);
        if let Some(fragment) = fragment {
            program.add_shader(fragment.id);
        }
        if let Some(geometry) = geometry {
            program.add_shader(geometry.id);
        }
        program.link()
    }

    /// Load a program from a vertex, optional fragment and optional geometry shaders sources.
    pub fn load(
        vertex: impl AsRef<Path>,
        fragment: Option<impl AsRef<Path>>,
        geometry: Option<impl AsRef<Path>>,
    ) -> Result<Self> {
        let vertex = std::fs::read_to_string(vertex)?;
        let fragment = if let Some(path) = fragment {
            Some(std::fs::read_to_string(path)?)
        } else {
            None
        };
        let geometry = if let Some(path) = geometry {
            Some(std::fs::read_to_string(path)?)
        } else {
            None
        };
        Self::from_sources(&vertex, fragment.as_deref(), geometry.as_deref())
    }

    pub fn num_uniforms(&self) -> usize {
        let mut num_uniforms = 0;
        unsafe {
            gl::GetProgramInterfaceiv(
                self.id.get(),
                gl::UNIFORM,
                gl::ACTIVE_RESOURCES,
                &mut num_uniforms,
            );
        }
        num_uniforms as _
    }

    /// Iterate over uniforms in this linked program.
    pub fn get_uniforms(&self) -> impl Iterator<Item = UniformDesc> {
        let num_uniforms = self.num_uniforms();
        let program_id = self.id;
        (0..num_uniforms as u32).map(move |ix| UniformDesc::for_uniform_at_location(program_id, ix))
    }

    /// Select an uniform from the program. Returns `None` if the uniform doesn't exist.
    pub fn uniform(&self, name: &str) -> Option<UniformLocation> {
        // Leave it as i32 because it can return -1 for errors
        let location = unsafe {
            let name = CString::new(name).unwrap();
            gl::GetUniformLocation(self.id.get(), name.as_ptr() as *const _)
        };
        tracing::trace!(
            "glGetUniformLocation({}, {}) -> {}",
            self.id.get(),
            name,
            location
        );
        if location >= 0 {
            Some(UniformLocation {
                program: self.id,
                location: location as _,
                desc: UniformDesc::for_uniform_at_location(self.id, location as _),
            })
        } else {
            None
        }
    }

    pub fn uniform_block(&self, name: &str, binding: u32) -> Result<UniformBlockIndex> {
        let block_index = gl_error_guard(|| unsafe {
            let name = CString::new(name).unwrap();
            gl::GetUniformBlockIndex(self.id.get(), name.as_ptr() as *const _)
        })?;
        tracing::trace!(
            "glGetUniformBlockIndex({}, {}) -> {}",
            self.id.get(),
            name,
            block_index
        );
        Ok(UniformBlockIndex {
            block_index,
            binding,
            program: self.id,
        })
    }

    pub fn num_attributes(&self) -> usize {
        let mut ret = 0;
        unsafe {
            gl::GetProgramiv(self.id.get(), gl::ACTIVE_ATTRIBUTES, &mut ret);
        }
        ret as _
    }

    pub fn get_attributes(&self) -> impl Iterator<Item = AttributeDesc> {
        let num_attributes = self.num_attributes();
        let id = self.id;
        (0..num_attributes as _).map(move |attr| AttributeDesc::for_attribute(id, attr))
    }

    pub fn attribute(&self, name: &str) -> Result<AttributeDesc> {
        let attr = gl_error_guard(|| unsafe {
            let name = CString::new(name).unwrap();
            gl::GetAttribLocation(self.id.get(), name.as_ptr())
        })?;
        eyre::ensure!(attr > 0, "Attribute does not exist");

        Ok(AttributeDesc::for_attribute(self.id, attr as _))
    }

    pub fn set_uniform<T: Uniform>(&self, location: UniformLocation, value: T) -> Result<()> {
        if self.id != location.program {
            eyre::bail!(
                "Cannot set uniform for program {} as the uniform location is for program {}",
                self.id.get(),
                location.program.get()
            );
        }
        gl_error_guard(|| {
            self.with_binding(|| unsafe { value.write_uniform(location.location as _) })
        })
    }

    pub fn bind_block<T>(
        &self,
        location: UniformBlockIndex,
        buf: &BufferSlice<T, { gl::UNIFORM_BUFFER }>,
    ) -> Result<()> {
        gl_error_guard(|| unsafe {
            gl::BindBufferRange(
                gl::UNIFORM_BUFFER,
                location.binding,
                buf.buffer.id.get(),
                buf.offset,
                buf.size,
            );
            gl::UniformBlockBinding(self.id.get(), location.block_index, location.binding);
            tracing::debug!("Bind buffer slice {} at block index {} at location {}", self.id.get(), location.block_index, location.binding);
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct UniformDesc {
    pub location: u32,
    pub block_index: u32,
    program: ProgramId,
    name_length: usize,
    raw_type: u32,
}

impl UniformDesc {
    const PROG_IFACE_LEN: usize = 4;
    const PROGRAM_INTERFACE: [GLenum; Self::PROG_IFACE_LEN] =
        [gl::NAME_LENGTH, gl::TYPE, gl::BLOCK_INDEX, gl::LOCATION];

    fn for_uniform_at_location(program: ProgramId, location: u32) -> UniformDesc {
        let mut values = [0; Self::PROG_IFACE_LEN];
        unsafe {
            gl::GetProgramResourceiv(
                program.get(),
                gl::UNIFORM,
                location,
                Self::PROG_IFACE_LEN as _,
                Self::PROGRAM_INTERFACE.as_ptr(),
                Self::PROG_IFACE_LEN as _,
                std::ptr::null_mut(),
                values.as_mut_ptr(),
            );
        }
        UniformDesc {
            program,
            location: values[3] as _,
            name_length: values[0] as _,
            block_index: values[2] as _,
            raw_type: values[1] as _,
        }
    }

    pub fn name(&self) -> Cow<str> {
        gl_string(
            Some(self.name_length as _),
            |capacity, len_ptr, str_ptr| unsafe {
                gl::GetProgramResourceName(
                    self.program.get(),
                    gl::UNIFORM,
                    self.location,
                    capacity as _,
                    len_ptr,
                    str_ptr,
                )
            },
        )
    }

    pub fn is_type<T: GlType>(&self) -> bool {
        T::GL_TYPE == self.raw_type
    }
}

pub fn current_program() -> Option<ProgramId> {
    ProgramId::new(unsafe {
        let mut current_program = 0;
        gl::GetIntegerv(gl::CURRENT_PROGRAM, &mut current_program);
        current_program as _
    })
}

#[derive(Debug, Clone)]
pub struct AttributeDesc {
    pub program: ProgramId,
    pub index: u32,
    pub name: Cow<'static, str>,
    pub gl_size: i32,
    raw_type: GLenum,
}

impl AttributeDesc {
    pub fn is<T: GlType>(&self) -> bool {
        self.raw_type == T::GL_TYPE
    }

    fn for_attribute(id: ProgramId, attr: u32) -> Self {
        let mut raw_type = 0;
        let mut gl_size = 0;
        let name = gl_string(None, |cap, len_ptr, str_ptr| unsafe {
            gl::GetActiveAttrib(
                id.get(),
                attr,
                cap as _,
                len_ptr,
                &mut gl_size,
                &mut raw_type,
                str_ptr,
            )
        });
        Self {
            program: id,
            index: attr,
            name,
            gl_size,
            raw_type,
        }
    }
}