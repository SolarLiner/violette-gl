use anyhow::Context;
use gl::types::{GLchar, GLsizei};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use thiserror::Error;

pub fn get_string(getter: impl FnOnce(usize, *mut GLsizei, *mut GLchar)) -> String {
    const BUFFER_LENGTH: usize = 1024;
    let mut data = vec![0u8; BUFFER_LENGTH];
    let mut length = 0;
    getter(BUFFER_LENGTH, &mut length, data.as_mut_ptr() as *mut _);
    String::from_utf8_lossy(&data[..length as usize]).to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error, FromPrimitive)]
#[repr(u32)]
pub enum GlError {
    #[error("Provided enum value is not valid")]
    InvalidEnum = gl::INVALID_ENUM,
    #[error("Provided value is not valid")]
    InvalidValue = gl::INVALID_VALUE,
    #[error("Invalid OpenGL operation")]
    InvalidOperation = gl::INVALID_OPERATION,
    #[error("Stack Overflow")]
    StackOverflow = gl::STACK_OVERFLOW,
    #[error("Stack Underflow")]
    StackUnderflow = gl::STACK_UNDERFLOW,
    #[error("Out of memory")]
    OutOfMemory = gl::OUT_OF_MEMORY,
    #[error("Invalid OpenGL operation on the framebuffer")]
    InvalidFramebufferOperation = gl::INVALID_FRAMEBUFFER_OPERATION,
    #[error("Context lost")]
    ContextLost = gl::CONTEXT_LOST,
}

pub fn gl_error() -> anyhow::Result<()> {
    let error = unsafe { gl::GetError() };
    if error != gl::NO_ERROR {
        GlError::from_u32(error)
            .map(|err| Err(err).context("OpenGL error"))
            .unwrap_or(Err(anyhow::anyhow!("Unknown OpenGL error")))
    } else {
        Ok(())
    }
}

pub fn gl_error_guard<T, F: FnOnce() -> T>(run: F) -> anyhow::Result<T> {
    let ret = run();
    gl_error()?;
    Ok(ret)
}
