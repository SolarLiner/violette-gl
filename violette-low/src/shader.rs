use std::{
    ffi::{CStr, CString},
    num::NonZeroU32,
    path::Path,
};

use anyhow::Context;
use gl::types::GLuint;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

use crate::utils::{gl_error_guard, gl_string};

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive)]
#[repr(u32)]
#[non_exhaustive]
/// Possible shader stages that can be put into an OpenGL pipeline.
pub enum ShaderStage {
    Vertex = gl::VERTEX_SHADER,
    Fragment = gl::FRAGMENT_SHADER,
    Geometry = gl::GEOMETRY_SHADER,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
/// Shader ID newtype. Guaranteed to be non-zero if it exists. Allows `Option<ShaderId>` to coerce
/// into a single `u32` into memory.
pub struct ShaderId(NonZeroU32);

impl std::ops::Deref for ShaderId {
    type Target = NonZeroU32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ShaderId {
    fn new(id: u32) -> Option<Self> {
        Some(Self(NonZeroU32::new(id)?))
    }
}

#[derive(Debug)]
/// OpenGL shader, a unit of work in an OpenGL pipeline.
pub struct Shader {
    /// Shader ID. Guaranteed to be non-zero, as ID 0 is reserved for unbinding shaders.
    pub id: ShaderId,
    /// Stage associated with this shader.
    pub stage: ShaderStage,
}

impl Drop for Shader {
    fn drop(&mut self) {
        tracing::trace!("glDeleteShader({})", self.id.get());
        unsafe {
            gl::DeleteShader(self.id.get());
        }
    }
}

impl Shader {
    /// Initializes a shader object from an existing shader.
    pub fn from_id(id: GLuint) -> anyhow::Result<Self> {
        anyhow::ensure!(id > 0, "Shader IDs need to be strictly positive");
        let stage = gl_error_guard(|| unsafe {
            let mut kind = 0;
            gl::GetShaderiv(id, gl::SHADER_TYPE, &mut kind);
            ShaderStage::from_u32(kind as _).unwrap()
        })?;
        Ok(Self {
            id: ShaderId::new(id).unwrap(),
            stage,
        })
    }

    /// Create a shader from the provided source. The shader will be compiled and verified within
    /// this method call.
    #[tracing::instrument(skip(source))]
    pub fn new(stage: ShaderStage, source: &str) -> anyhow::Result<Self> {
        let id = unsafe { gl::CreateShader(stage as _) };
        tracing::trace!("glCreateShader({:?}) -> {}", stage, id);
        let success = unsafe {
            let source = CString::new(source).unwrap();
            gl::ShaderSource(id, 1, &source.as_ptr(), std::ptr::null());
            gl::CompileShader(id);
            let mut success = 0;
            gl::GetShaderiv(id, gl::COMPILE_STATUS, &mut success);
            success == 1
        };
        if !success {
            let error = unsafe {
                let mut length = 0;
                gl::GetShaderiv(id, gl::INFO_LOG_LENGTH, &mut length);
                gl_string(Some(length as _), |len, len_ptr, ptr| {
                    gl::GetShaderInfoLog(id, len as _, len_ptr, ptr)
                })
                .unwrap()
            };
            anyhow::bail!(error);
        } else {
            Ok(Self {
                id: ShaderId::new(id).unwrap(),
                stage,
            })
        }
    }

    pub fn load(stage: ShaderStage, path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let source = std::fs::read_to_string(path).context("Cannot read shader source")?;
        Self::new(stage, &source)
    }
}
