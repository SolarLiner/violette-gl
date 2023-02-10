use duplicate::duplicate_item;
use gl::types::*;

pub mod bindable;

pub trait GlType {
    const GL_TYPE: GLenum;
    const NUM_COMPONENTS: usize;
    const NORMALIZED: bool;
    const STRIDE: usize;
}

#[duplicate_item(
rust_t      gl_t;
[f32]       [gl::FLOAT];
[f64]       [gl::DOUBLE];
[u8]        [gl::UNSIGNED_BYTE];
[i8]        [gl::BYTE];
[u16]       [gl::UNSIGNED_SHORT];
[i16]       [gl::SHORT];
[u32]       [gl::UNSIGNED_INT];
[i32]       [gl::INT];
)]
impl GlType for rust_t {
    const GL_TYPE: GLenum = gl_t;
    const NUM_COMPONENTS: usize = 1;
    const NORMALIZED: bool = false;
    const STRIDE: usize = std::mem::size_of::<Self>();
}

#[duplicate_item(
n; [2]; [3]; [4];
)]
#[duplicate_item(
rust_t      gl_t;
[[f32; n]]  [gl::FLOAT];
[[f64; n]]  [gl::DOUBLE];
[[u8; n]]   [gl::UNSIGNED_BYTE];
[[i8; n]]   [gl::BYTE];
[[u16; n]]  [gl::UNSIGNED_SHORT];
[[i16; n]]  [gl::SHORT];
[[u32; n]]  [gl::UNSIGNED_INT];
[[i32; n]]  [gl::INT];
)]
impl GlType for rust_t {
    const GL_TYPE: GLenum = gl_t;
    const NUM_COMPONENTS: usize = n;
    const NORMALIZED: bool = false;
    const STRIDE: usize = std::mem::size_of::<Self>();
}

#[duplicate_item(
n; [2]; [3]; [4];
)]
#[duplicate_item(
m; [2]; [3]; [4];
)]
#[duplicate_item(
rust_t              gl_t;
[[[f32; n]; m]]     [gl::FLOAT];
[[[f64; n]; m]]     [gl::DOUBLE];
[[[u8; n]; m]]      [gl::UNSIGNED_BYTE];
[[[i8; n]; m]]      [gl::BYTE];
[[[u16; n]; m]]     [gl::UNSIGNED_SHORT];
[[[i16; n]; m]]     [gl::SHORT];
[[[u32; n]; m]]     [gl::UNSIGNED_INT];
[[[i32; n]; m]]     [gl::INT];
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
#[duplicate_item(
rust_t          n       gl_t;
[glam::Vec2]    [2]     [gl::FLOAT];
[glam::DVec2]   [2]     [gl::DOUBLE];
[glam::UVec2]   [2]     [gl::UNSIGNED_INT];
[glam::IVec2]   [2]     [gl::INT];
[glam::Vec3]    [3]     [gl::FLOAT];
[glam::Vec3A]   [3]     [gl::FLOAT];
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

#[cfg(feature = "vertex-glam")]
#[duplicate_item(
rust_t          n       gl_t;
[glam::Mat2]    [2]     [gl::FLOAT];
[glam::Mat3]    [3]     [gl::FLOAT];
[glam::Mat4]    [4]     [gl::FLOAT];
[glam::DMat2]   [2]     [gl::DOUBLE];
[glam::DMat3]   [3]     [gl::DOUBLE];
[glam::DMat4]   [4]     [gl::DOUBLE];
)]
impl GlType for rust_t {
    const GL_TYPE: GLenum = gl_t;
    const NUM_COMPONENTS: usize = n;
    const NORMALIZED: bool = false;
    const STRIDE: usize = std::mem::size_of::<Self>();
}