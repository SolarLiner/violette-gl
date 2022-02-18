use std::{
    ffi::{CStr, CString},
    path::Path,
};

use gl::types::GLuint;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ShaderStage {
    Vertex = gl::VERTEX_SHADER,
    Fragment = gl::FRAGMENT_SHADER,
    Geometry = gl::GEOMETRY_SHADER,
}

impl ShaderStage {
    fn from_raw(value: GLuint) -> Option<Self> {
        Some(match value {
            gl::VERTEX_SHADER => Self::Vertex,
            gl::FRAGMENT_SHADER => Self::Fragment,
            gl::GEOMETRY_SHADER => Self::Geometry,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShaderId(pub(crate) GLuint);

#[derive(Debug)]
pub struct Shader {
    pub id: ShaderId,
    pub stage: ShaderStage,
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteShader(self.id.0);
        }
    }
}

impl Shader {
    pub(crate) unsafe fn from_raw(id: GLuint) -> Self {
        let mut shader_type = 0;
        gl::GetShaderiv(id, gl::SHADER_TYPE, &mut shader_type);
        Self {
            id: ShaderId(id),
            stage: ShaderStage::from_raw(shader_type as _).unwrap(),
        }
    }

    pub fn new(stage: ShaderStage, source: &str) -> anyhow::Result<Self> {
        let id = unsafe { gl::CreateShader(stage as _) };
        let has_errors = unsafe {
            let source = CString::new(source).unwrap();
            gl::ShaderSource(id, 1, &source.as_ptr(), std::ptr::null());
            gl::CompileShader(id);
            let mut success = 0;
            gl::GetShaderiv(id, gl::COMPILE_STATUS, &mut success);
            success == 1
        };
        if has_errors {
            let error = unsafe {
                let mut res = vec![0u8; 1024];
                gl::GetShaderInfoLog(
                    id,
                    res.len() as _,
                    std::ptr::null_mut(),
                    res.as_mut_ptr() as *mut _,
                );
                CStr::from_ptr(res.as_ptr() as *const _)
                    .to_string_lossy()
                    .to_owned()
            };
            anyhow::bail!(error);
        } else {
            Ok(Self {
                id: ShaderId(id),
                stage,
            })
        }
    }
}
