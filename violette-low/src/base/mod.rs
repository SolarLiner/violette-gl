use duplicate::duplicate;
use gl::types::*;

pub mod bindable;

pub trait GlType {
    const GL_TYPE: GLenum;
    const NUM_COMPONENTS: usize;
    const NORMALIZED: bool;
    const STRIDE: usize;
}

#[duplicate(
rust_t      n       gl_t;
[f32]       [1]     [gl::FLOAT];
[f64]       [1]     [gl::DOUBLE];
[u8]        [1]     [gl::UNSIGNED_BYTE];
[i8]        [1]     [gl::BYTE];
[u16]       [1]     [gl::UNSIGNED_SHORT];
[i16]       [1]     [gl::SHORT];
[u32]       [1]     [gl::UNSIGNED_INT];
[i32]       [1]     [gl::INT];
[[f32; 2]]  [2]     [gl::FLOAT];
[[f64; 2]]  [2]     [gl::DOUBLE];
[[u8; 2]]   [2]     [gl::UNSIGNED_BYTE];
[[i8; 2]]   [2]     [gl::BYTE];
[[u16; 2]]  [2]     [gl::UNSIGNED_SHORT];
[[i16; 2]]  [2]     [gl::SHORT];
[[u32; 2]]  [2]     [gl::UNSIGNED_INT];
[[i32; 2]]  [2]     [gl::INT];
[[f32; 3]]  [3]     [gl::FLOAT];
[[f64; 3]]  [3]     [gl::DOUBLE];
[[u8; 3]]   [3]     [gl::UNSIGNED_BYTE];
[[i8; 3]]   [3]     [gl::BYTE];
[[u16; 3]]  [3]     [gl::UNSIGNED_SHORT];
[[i16; 3]]  [3]     [gl::SHORT];
[[u32; 3]]  [3]     [gl::UNSIGNED_INT];
[[i32; 3]]  [3]     [gl::INT];
[[f32; 4]]  [4]     [gl::FLOAT];
[[f64; 4]]  [4]     [gl::DOUBLE];
[[u8; 4]]   [4]     [gl::UNSIGNED_BYTE];
[[i8; 4]]   [4]     [gl::BYTE];
[[u16; 4]]  [4]     [gl::UNSIGNED_SHORT];
[[i16; 4]]  [4]     [gl::SHORT];
[[u32; 4]]  [4]     [gl::UNSIGNED_INT];
[[i32; 4]]  [4]     [gl::INT];
)]
impl GlType for rust_t {
    const GL_TYPE: GLenum = gl_t;
    const NUM_COMPONENTS: usize = n;
    const NORMALIZED: bool = false;
    const STRIDE: usize = std::mem::size_of::<Self>();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Normalized<T>(pub T);

impl<T: GlType> GlType for Normalized<T> {
    const GL_TYPE: GLenum = T::GL_TYPE;
    const NUM_COMPONENTS: usize = T::NUM_COMPONENTS;
    const NORMALIZED: bool = true;
    const STRIDE: usize = T::STRIDE;
}

#[cfg(feature = "vertex-glam")]
#[duplicate(
rust_t          n       gl_t;
[glam::Vec2]    [2]     [gl::FLOAT];
[glam::DVec2]   [2]     [gl::DOUBLE];
[glam::UVec2]   [2]     [gl::UNSIGNED_INT];
[glam::IVec2]   [2]     [gl::INT];
[glam::Vec3]    [3]     [gl::FLOAT];
[glam::DVec3]   [3]     [gl::DOUBLE];
[glam::UVec3]   [3]     [gl::UNSIGNED_INT];
[glam::IVec3]   [3]     [gl::INT];
[glam::Vec4]    [4]     [gl::FLOAT];
[glam::DVec4]   [4]     [gl::DOUBLE];
[glam::UVec4]   [4]     [gl::UNSIGNED_INT];
[glam::IVec4]   [4]     [gl::INT];
)]
impl GlType for rust_t {
    const GL_TYPE: GLenum = gl_t;
    const NUM_COMPONENTS: usize = n;
    const NORMALIZED: bool = false;
    const STRIDE: usize = std::mem::size_of::<Self>();
}
