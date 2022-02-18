use std::{
    ffi::{CStr, CString},
    marker::PhantomData,
    num::NonZeroU32,
};

use duplicate::duplicate;
use gl::types::{GLdouble, GLfloat, GLint, GLuint};

use crate::shader::ShaderId;

pub trait Uniform: Sized {
    unsafe fn write_uniform(&self, location: GLint);
}

#[duplicate(
    gl_t        uniform;
    [GLint]     [Uniform1i];
    [GLuint]    [Uniform1ui];
    [GLfloat]   [Uniform1f];
    [GLdouble]  [Uniform1d];
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

#[derive(Debug)]
pub struct UniformLocation<'a, Type> {
    ty: PhantomData<&'a Type>,
    location: GLuint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
pub struct Unlinked;
#[derive(Debug)]
pub struct Linked;

#[derive(Debug)]
pub struct Program<Status> {
    __status: Status,
    pub id: ProgramId,
}

impl Program<Unlinked> {
    pub fn new() -> Self {
        let id = unsafe { gl::CreateProgram() };
        Self {
            id: ProgramId(NonZeroU32::new(id).unwrap()),
            __status: Unlinked,
        }
    }

    pub fn add_shader(&mut self, ShaderId(id): ShaderId) {
        unsafe { gl::AttachShader(self.id.get(), id) }
    }

    pub fn link(self) -> anyhow::Result<Program<Linked>> {
        let is_success = unsafe {
            gl::LinkProgram(self.id.get());
            let mut success = 0;
            gl::GetProgramiv(self.id.get(), gl::LINK_STATUS, &mut success);
            success == 1
        };
        if is_success {
            Ok(Program {
                id: self.id,
                __status: Linked,
            })
        } else {
            let error = unsafe {
                let mut buf = vec![0u8; 1024];
                gl::GetProgramInfoLog(
                    self.id.get(),
                    buf.len() as _,
                    std::ptr::null_mut(),
                    buf.as_mut_ptr() as *mut _,
                );
                CStr::from_ptr(buf.as_ptr() as *const _)
                    .to_string_lossy()
                    .to_owned()
            };
            anyhow::bail!(error);
        }
    }
}

impl Program<Linked> {
    pub(crate) unsafe fn from_raw(id: GLuint) -> Option<Self> {
        Some(Self {
            id: ProgramId::new(id)?,
            __status: Linked,
        })
    }

    pub fn from_shaders(shaders: impl IntoIterator<Item = ShaderId>) -> anyhow::Result<Self> {
        let mut program = Program::new();
        shaders
            .into_iter()
            .for_each(|sh_id| program.add_shader(sh_id));
        program.link()
    }

    pub fn activate(&self) -> ActiveProgram {
        ActiveProgram::from(self)
    }
}

pub struct ActiveProgram<'a> {
    program: &'a Program<Linked>,
    previous_program: Option<ProgramId>,
}

impl<'a> From<&'a Program<Linked>> for ActiveProgram<'a> {
    fn from(program: &'a Program<Linked>) -> Self {
        unsafe { gl::UseProgram(program.id.get()) }
        Self { program, previous_program: current_program() }
    }
}

impl<'a> Drop for ActiveProgram<'a> {
    fn drop(&mut self) {
        unsafe {
            match self.previous_program {
                Some(id) => gl::UseProgram(id.get()),
                None => gl::UseProgram(0),
            }
        }
    }
}

impl<'a> ActiveProgram<'a> {
    pub fn uniform<Type: Uniform>(&self, name: &str) -> Option<UniformLocation<Type>> {
        let location = unsafe {
            let name = CString::new(name).unwrap();
            gl::GetUniformLocation(self.program.id.get(), name.as_ptr() as *const _)
        };
        if location > 0 {
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
