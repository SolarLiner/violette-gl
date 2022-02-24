use std::fmt::Debug;
use std::path::Path;
use std::{ffi::CString, marker::PhantomData, num::NonZeroU32};

use duplicate::duplicate;
use gl::types::{GLdouble, GLfloat, GLint, GLuint};

use crate::base::bindable::{Binding, Resource};
use crate::shader::Shader;
use crate::utils::gl_error_guard;
use crate::{shader::ShaderId, utils::gl_string};

/// Trait of types that can be written into shader uniforms. This allows polymorphic use of the
/// methods on [`ActiveProgram`](struct::ActiveProgram);
pub trait Uniform: Sized {
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

#[cfg(feature="uniforms-glam")]
#[duplicate(
    glam_t      ;
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

#[cfg(feature="uniforms-glam")]
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

#[derive(Debug)]
/// Structure allowing uniforms to be written into a program.
pub struct UniformLocation<'a, Type> {
    ty: PhantomData<&'a Type>,
    location: GLuint,
}

impl<'a, Type: Uniform> UniformLocation<'a, Type> {
    pub fn set(&self, value: Type) -> anyhow::Result<()> {
        gl_error_guard(|| unsafe {
            value.write_uniform(self.location as _);
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
/// Program ID newtype. Guaranteed to be non-zero if it exists. Allows `Option<ProgramId>` to coerce
/// into a single `u32` into memory.
pub struct ProgramId(pub(crate) NonZeroU32);

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
/// unlinked program produces a `Program<Linked>` structure.
pub struct Unlinked;
#[derive(Debug)]
/// Linked program. A linked program can be used, and uniforms set.
pub struct Linked;

#[derive(Debug)]
/// A shader program. Linkage status is tracked at compile-time.
pub struct Program<Status> {
    __status: Status,
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
    pub fn validate(&self) -> anyhow::Result<()> {
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
            })
            .unwrap();
            anyhow::bail!(error);
        }
        Ok(())
    }
}

impl Program<Unlinked> {
    /// Create a new, empty program.
    pub fn new() -> Self {
        let id = unsafe { gl::CreateProgram() };
        Self {
            id: ProgramId(NonZeroU32::new(id).unwrap()),
            __status: Unlinked,
        }
    }

    ///Add a compiled shader into the current program.
    pub fn add_shader(&mut self, id: ShaderId) {
        tracing::trace!("glAttachShader({}, {})", self.id.get(), id.get());
        unsafe { gl::AttachShader(self.id.get(), id.get()) }
    }

    /// Link the program.
    pub fn link(self) -> anyhow::Result<Program<Linked>> {
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
            assert_eq!(unsafe { gl::IsProgram(id) }, gl::TRUE);
            Ok(Program {
                id: ProgramId::new(id).unwrap(),
                __status: Linked,
            })
        } else {
            let error = unsafe {
                let mut length = 0;
                gl::GetProgramiv(id, gl::INFO_LOG_LENGTH, &mut length);
                gl_string(Some(length as _), |len, len_ptr, ptr| {
                    gl::GetProgramInfoLog(id, len as _, len_ptr, ptr)
                })
                .unwrap()
            };
            anyhow::bail!(error);
        }
    }
}

impl<'a> Resource<'a> for Program<Linked> {
    type Id = ProgramId;
    type Kind = ();
    type Bound = ActiveProgram<'a>;

    fn current(_: Self::Kind) -> Option<Self::Id> {
        let mut id = 0;
        unsafe { gl::GetIntegerv(gl::CURRENT_PROGRAM, &mut id) }
        Self::Id::new(id as _)
    }

    fn kind(&self) -> Self::Kind {
        ()
    }

    fn make_binding(&'a mut self) -> anyhow::Result<Self::Bound> {
        tracing::trace!("glUseProgram({})", self.id.get());
        unsafe {
            gl::UseProgram(self.id.get());
        }
        Ok(ActiveProgram { program: self })
    }
}

impl Program<Linked> {
    /// Create a program from the provided shaders. The resulting program will be linked and ready
    /// to use.
    pub fn from_shaders(shaders: impl IntoIterator<Item = ShaderId>) -> anyhow::Result<Self> {
        let mut program = Program::new();
        shaders
            .into_iter()
            .for_each(|sh_id| program.add_shader(sh_id));
        program.link()
    }

    pub fn from_sources<'vs, 'fs, 'gs>(
        vertex_shader: &'vs str,
        fragment_shader: impl Into<Option<&'fs str>>,
        geometry_shader: impl Into<Option<&'gs str>>,
    ) -> anyhow::Result<Self> {
        let vertex = Shader::new(crate::shader::ShaderStage::Vertex, vertex_shader)?;
        let fragment = if let Some(source) = fragment_shader.into() {
            Some(Shader::new(crate::shader::ShaderStage::Fragment, source)?)
        } else {
            None
        };
        let geometry = if let Some(source) = geometry_shader.into() {
            Some(Shader::new(crate::shader::ShaderStage::Geometry, source)?)
        } else {
            None
        };
        Self::from_shaders(
            std::iter::once(vertex.id)
                .chain(fragment.as_ref().map(|s| s.id))
                .chain(geometry.as_ref().map(|s| s.id)),
        )
    }

    pub fn load(
        vertex: impl AsRef<Path>,
        fragment: Option<impl AsRef<Path>>,
        geometry: Option<impl AsRef<Path>>,
    ) -> anyhow::Result<Self> {
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
}

/// An active program. The program gets bound when this gets constructed, and unbound when the
/// variable goes out of scope.
pub struct ActiveProgram<'a> {
    program: &'a Program<Linked>,
}

impl<'a> std::ops::Deref for ActiveProgram<'a> {
    type Target = Program<Linked>;

    fn deref(&self) -> &Self::Target {
        self.program
    }
}

impl<'a> Binding<'a> for ActiveProgram<'a> {
    type Parent = Program<Linked>;

    fn unbind(&mut self, previous: Option<<Program<Linked> as Resource>::Id>) {
        let prev_id = previous.map(|id| id.get()).unwrap_or(0);
        unsafe {
            gl::UseProgram(prev_id);
        }
    }
}

impl<'a> ActiveProgram<'a> {
    /// Select an uniform from the program. Returns `None` if the uniform doesn't exist.
    pub fn uniform<Type: Uniform>(&self, name: &str) -> Option<UniformLocation<Type>> {
        let location = unsafe {
            let name = CString::new(name).unwrap();
            gl::GetUniformLocation(self.program.id.get(), name.as_ptr() as *const _)
        };
        tracing::trace!(
            "glGetUniformLocation({}, {}) -> {}",
            self.id.get(),
            name,
            location
        );
        if location >= 0 {
            Some(UniformLocation {
                ty: PhantomData,
                location: location as _,
            })
        } else {
            None
        }
    }
}

pub fn current_program() -> Option<ProgramId> {
    ProgramId::new(unsafe {
        let mut current_program = 0;
        gl::GetIntegerv(gl::CURRENT_PROGRAM, &mut current_program);
        current_program as _
    })
}
