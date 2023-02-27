use std::fmt::Formatter;
use std::{ffi::CString, fmt, num::NonZeroU32, path::Path};
use std::marker::PhantomData;

use eyre::{Context, Result};

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

use crate::utils::gl_string;

pub type VertexShader = Shader<{ gl::VERTEX_SHADER }>;
pub type FragmentShader = Shader<{ gl::FRAGMENT_SHADER }>;
pub type GeometryShader = Shader<{ gl::GEOMETRY_SHADER }>;

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
pub struct ShaderId<const K: u32>(NonZeroU32);

impl<const K: u32> fmt::Display for ShaderId<K> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.get())
    }
}

impl<const K: u32> std::ops::Deref for ShaderId<K> {
    type Target = NonZeroU32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const K: u32> ShaderId<K> {
    fn new(id: u32) -> Option<Self> {
        Some(Self(NonZeroU32::new(id)?))
    }
}

#[derive(Debug)]
/// OpenGL shader, a unit of work in an OpenGL pipeline.
pub struct Shader<const K: u32> {
    __non_send: PhantomData<*mut ()>,
    /// Shader ID. Guaranteed to be non-zero, as ID 0 is reserved for unbinding shaders.
    pub id: ShaderId<K>,
}

impl<const K: u32> Drop for Shader<K> {
    fn drop(&mut self) {
        tracing::trace!("glDeleteShader({})", self.id.get());
        unsafe {
            gl::DeleteShader(self.id.get());
        }
    }
}

impl<const K: u32> Shader<K> {
    /// Create a shader from the provided source. The shader will be compiled and verified within
    /// this method call.
    #[tracing::instrument(skip(source))]
    pub fn new(source: &str) -> Result<Self> {
        tracing::trace!("{}", source);
        let id = unsafe { gl::CreateShader(K) };
        tracing::trace!("glCreateShader({:?}) -> {}", K, id);
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
            };
            eyre::bail!(error);
        } else {
            Ok(Self {
                __non_send: PhantomData,
                id: ShaderId::new(id).unwrap(),
            })
        }
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let source = std::fs::read_to_string(path).context("Cannot read shader source")?;
        Self::new(&source)
    }
}
