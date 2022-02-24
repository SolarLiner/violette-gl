use std::ffi::c_void;

pub use crevice::glsl;
pub use crevice::std140;

pub mod base;
pub mod buffer;
pub mod debug;
pub mod program;
pub mod shader;
mod utils;
pub mod framebuffer;
pub mod vertex;
pub mod texture;

pub fn load_with(loader: impl FnMut(&'static str) -> *const c_void) {
    gl::load_with(loader)
}
