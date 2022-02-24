use bitflags::bitflags;
use std::num::NonZeroU32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FramebufferId(NonZeroU32);

impl std::ops::Deref for FramebufferId {
    type Target = NonZeroU32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FramebufferId {
    pub fn new(id: u32) -> Option<Self> {
        Some(FramebufferId(NonZeroU32::new(id)?))
    }
}

bitflags! {
    pub struct ClearBuffer: u32 {
        const COLOR = gl::COLOR_BUFFER_BIT;
        const DEPTH = gl::DEPTH_BUFFER_BIT;
        const STENCIL = gl::STENCIL_BUFFER_BIT;
    }
}

pub struct Backbuffer;

impl Backbuffer {
    pub fn clear_color(&self, [r, g, b]: [f32; 3]) {
        unsafe {
            gl::ClearColor(r, g, b, 1.0);
        }
    }

    pub fn clear_depth(&self, depth: f64) {
        unsafe {
            gl::ClearDepth(depth);
        }
    }

    pub fn viewport(&self, x: usize, y: usize, width: usize, height: usize) {
        unsafe {
            gl::Viewport(x as _, y as _, width as _, height as _);
        }
    }

    pub fn clear(&self, buffers: ClearBuffer) {
        unsafe {
            gl::Clear(buffers.bits);
        }
    }
}
