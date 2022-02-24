use std::ffi::CStr;

use anyhow::Context;
use gl::types::{GLchar, GLsizei};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use thiserror::Error;

/// Helper for converting OpenGL string messages into Rust's String type.
pub(crate) fn gl_string(
    planned_length: Option<usize>,
    getter: impl FnOnce(usize, *mut GLsizei, *mut GLchar),
) -> Option<String> {
    let capacity = planned_length.unwrap_or(1024);
    let mut data = vec![0u8; capacity];
    let mut length = 0;
    getter(capacity, &mut length, data.as_mut_ptr() as *mut _);

    if length == 0 {
        return None;
    }
    Some(
        CStr::from_bytes_with_nul(&data)
            .expect("OpenGL failure: corrupted string message")
            .to_string_lossy()
            .to_string(),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error, FromPrimitive)]
#[repr(u32)]
/// Rust Error type for OpenGL error sources
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

/// Utility function to catch errors as raised by OpenGL
pub(crate) fn gl_error() -> anyhow::Result<()> {
    let error = unsafe { gl::GetError() };
    if error != gl::NO_ERROR {
        Err(GlError::from_u32(error)
            .map(|err| anyhow::anyhow!("OpenGL Error: {} (check debug log for more details)", err))
            .unwrap_or(anyhow::anyhow!("Unknown OpenGL error (check debug log for more details)")))
    } else {
        Ok(())
    }
}

/// Utility to run a closure, checking for any OpenGL errors before returning the result
pub fn gl_error_guard<T, F: FnOnce() -> T>(run: F) -> anyhow::Result<T> {
    let ret = run();
    gl_error()?;
    Ok(ret)
}
