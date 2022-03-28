use std::ffi::c_void;

pub use crevice::glsl;
pub use crevice::std140;

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
