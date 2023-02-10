use std::{
    ffi::{c_void, CStr},
    ops::Not,
};

use anyhow::ContextCompat;
use gl::types::GLenum;
use num_derive::FromPrimitive;

use utils::gl_error_guard;

pub use gl;

pub mod base;
pub mod buffer;
pub mod debug;
pub mod framebuffer;
pub mod program;
pub mod shader;
pub mod texture;
mod utils;
pub mod vertex;

pub fn load_with(loader: impl FnMut(&'static str) -> *const c_void) {
    gl::load_with(loader)
}

pub fn line_width() -> f32 {
    unsafe {
        let mut value = 0.;
        gl::GetFloatv(gl::LINE_WIDTH, &mut value);
        value
    }
}

pub fn set_line_width(width: f32) {
    unsafe { gl::LineWidth(width) }
}

pub fn set_line_smooth(smooth: bool) {
    if smooth {
        unsafe { gl::Enable(gl::LINE_SMOOTH) }
    } else {
        unsafe { gl::Disable(gl::LINE_SMOOTH) }
    }
}

pub fn point_size() -> f32 {
    unsafe {
        let mut value = 0.;
        gl::GetFloatv(gl::POINT_SIZE, &mut value);
        value
    }
}

pub fn set_point_size(size: f32) {
    unsafe { gl::PointSize(size) }
}

pub fn get_string(of: GLenum) -> anyhow::Result<String> {
    gl_error_guard(|| unsafe {
        let ret = gl::GetString(of);
        ret.is_null()
            .not()
            .then(|| CStr::from_ptr(ret.cast()).to_string_lossy().to_string())
            .context("No string returned from OpenGL")
    })
    .and_then(|res| res)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive)]
#[repr(u32)]
pub enum Cull {
    Front = gl::FRONT,
    Back = gl::BACK,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive)]
#[repr(u32)]
pub enum FrontFace {
    Clockwise = gl::CW,
    CounterClockwise = gl::CCW,
}

pub fn culling(mode: impl Into<Option<Cull>>) {
    gl_error_guard(|| match mode.into() {
        Some(mode) => unsafe {
            gl::Enable(gl::CULL_FACE);
            gl::CullFace(mode as _);
        },
        None => unsafe {
            gl::CullFace(gl::FRONT_AND_BACK);
            gl::Disable(gl::CULL_FACE);
        },
    })
    .unwrap();
}

pub fn set_front_face(front_face: FrontFace) {
    gl_error_guard(|| unsafe { gl::FrontFace(front_face as _) }).unwrap();
}
