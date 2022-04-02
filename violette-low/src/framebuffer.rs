use std::{
    num::NonZeroU32,
    ops::{Range, RangeBounds},
};

use bitflags::bitflags;
use gl::types::GLuint;

use crate::vertex::BoundVao;
use crate::{
    base::{
        bindable::{Binding, Resource},
        GlType,
    },
    buffer::BoundBuffer,
    program::ActiveProgram,
    texture::{DepthStencil, Dimension, Texture},
    utils::gl_error_guard,
    vertex::DrawMode,
};
use crate::texture::BoundTexture;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FramebufferId(u32);

impl std::ops::Deref for FramebufferId {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FramebufferId {
    const BACKBUFFER: FramebufferId = FramebufferId(0);

    pub fn new(id: u32) -> Option<Self> {
        if id == 0 {
            return None;
        }
        Some(FramebufferId(id))
    }
}

bitflags! {
    pub struct ClearBuffer: u32 {
        const COLOR = gl::COLOR_BUFFER_BIT;
        const DEPTH = gl::DEPTH_BUFFER_BIT;
        const STENCIL = gl::STENCIL_BUFFER_BIT;
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u32)]
pub enum Blend {
    Zero = gl::ZERO,
    One = gl::ONE,
    SrcColor = gl::SRC_COLOR,
    OneMinusSrcColor = gl::ONE_MINUS_SRC_COLOR,
    DstColor = gl::DST_COLOR,
    OneMinusDstColor = gl::ONE_MINUS_DST_COLOR,
    SrcAlpha = gl::SRC_ALPHA,
    OneMinusSrcAlpha = gl::ONE_MINUS_SRC_ALPHA,
    DstAlpha = gl::DST_ALPHA,
    OneMinusDstAlpha = gl::ONE_MINUS_DST_ALPHA,
    ConstantColor = gl::CONSTANT_COLOR,
    OneMinusConstantColor = gl::ONE_MINUS_CONSTANT_COLOR,
    ConstantAlpha = gl::CONSTANT_ALPHA,
    OneMinusConstantAlpha = gl::ONE_MINUS_CONSTANT_ALPHA,
    SrcAlphaSaturate = gl::SRC_ALPHA_SATURATE,
    Src1Color = gl::SRC1_COLOR,
    OneMinusSrc1Color = gl::ONE_MINUS_SRC1_COLOR,
    Src1Alpha = gl::SRC1_ALPHA,
    OneMinusSrc1Alpha = gl::ONE_MINUS_SRC1_ALPHA,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u32)]
pub enum DepthTestFunction {
    Less = gl::LESS,
    Equal = gl::EQUAL,
    LEqual = gl::LEQUAL,
    Greater = gl::GREATER,
    NotEqual = gl::NOTEQUAL,
    GEqual = gl::GEQUAL,
    Always = gl::ALWAYS,
}

#[derive(Debug, Copy, Clone)]
pub enum FramebufferFeature {
    DepthTest(DepthTestFunction),
}

impl FramebufferFeature {
    unsafe fn enable(&self) {
        match self {
            &Self::DepthTest(func) => {
                gl::Enable(gl::DEPTH_TEST);
                gl::DepthFunc(func as _)
            }
        }
    }

    unsafe fn disable(&self) {
        gl::Disable(match self {
            Self::DepthTest(_) => gl::DEPTH_TEST,
        })
    }
}

#[derive(Debug)]
pub struct Framebuffer {
    id: FramebufferId,
}

impl std::ops::Deref for Framebuffer {
    type Target = FramebufferId;

    fn deref(&self) -> &Self::Target {
        &self.id
    }
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        if self.id.0 > 0 {
            unsafe { gl::DeleteFramebuffers(1, &*self.id) }
        }
    }
}

impl Framebuffer {
    pub const fn backbuffer() -> Self {
        Self {
            id: FramebufferId::BACKBUFFER,
        }
    }

    pub fn new() -> Framebuffer {
        let id = unsafe {
            let mut fbo = 0;
            gl::GenFramebuffers(1, &mut fbo);
            fbo
        };
        Self {
            id: FramebufferId::new(id).unwrap(),
        }
    }
}

impl<'a> Resource<'a> for Framebuffer {
    type Id = FramebufferId;

    type Kind = ();

    type Bound = BoundFB<'a>;

    fn current((): ()) -> Option<Self::Id> {
        let mut id = 0;
        unsafe {
            gl::GetIntegerv(gl::FRAMEBUFFER_BINDING, &mut id);
        }
        Some(FramebufferId(id as _))
    }

    fn kind(&self) -> Self::Kind {
        ()
    }

    fn make_binding(&'a mut self) -> anyhow::Result<Self::Bound> {
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.id.0 as _);
        }
        Ok(BoundFB { fb: self })
    }
}

pub struct BoundFB<'a> {
    fb: &'a Framebuffer,
}

impl<'a> std::ops::Deref for BoundFB<'a> {
    type Target = Framebuffer;

    fn deref(&self) -> &Self::Target {
        self.fb
    }
}

impl<'a> Binding<'a> for BoundFB<'a> {
    type Parent = Framebuffer;

    fn unbind(&mut self, previous: Option<FramebufferId>) {
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, previous.map(|id| id.0).unwrap_or(0));
        }
    }
}

impl<'a> BoundFB<'a> {
    pub fn viewport(&self, x: usize, y: usize, width: usize, height: usize) {
        unsafe {
            gl::Viewport(x as _, y as _, width as _, height as _);
        }
    }

    pub fn clear_color(&mut self, [red, green, blue, alpha]: [f32; 4]) {
        unsafe { gl::ClearColor(red, green, blue, alpha) }
    }

    pub fn clear_depth(&mut self, value: f64) {
        unsafe {
            gl::ClearDepth(value);
        }
    }

    pub fn do_clear(&mut self, mode: ClearBuffer) {
        unsafe {
            gl::Clear(mode.bits());
        }
    }

    pub fn enable_feature(&mut self, feature: FramebufferFeature) -> anyhow::Result<()> {
        gl_error_guard(|| unsafe {
            feature.enable();
        })
    }

    pub fn disable_feature(&mut self, feature: FramebufferFeature) -> anyhow::Result<()> {
        gl_error_guard(|| unsafe {
            feature.disable();
        })
    }

    pub fn set_blending(&mut self, blend_source: Blend, blend_dest: Blend) -> anyhow::Result<()> {
        gl_error_guard(|| unsafe { gl::BlendFunc(blend_source as _, blend_dest as _) })
    }

    pub fn draw(
        &mut self,
        _vao_binding: &mut BoundVao,
        mode: DrawMode,
        vertices: Range<i32>,
    ) -> anyhow::Result<()> {
        gl_error_guard(|| unsafe {
            gl::DrawArrays(mode as _, vertices.start, vertices.end - vertices.start);
        })
    }

    pub fn draw_elements<I: GlType, B: RangeBounds<i32>>(
        &mut self,
        _vao_binding: &mut BoundVao,
        buffer: &BoundBuffer<I>,
        mode: DrawMode,
        slice: B,
    ) -> anyhow::Result<()> {
        let slice = normalize_range(slice, 0..buffer.len() as _);
        let count = slice.end - slice.start;
        gl_error_guard(|| unsafe {
            gl::DrawElements(mode as _, count, I::GL_TYPE, std::ptr::null());
        })
    }

    pub fn attach_color<F>(&mut self, attachment: u8, texture: &BoundTexture<F>) -> anyhow::Result<()> {
        gl_error_guard(|| unsafe {
            match texture.dimension() {
                Dimension::D1 => gl::FramebufferTexture1D(
                    gl::FRAMEBUFFER,
                    gl::COLOR_ATTACHMENT0 + attachment as GLuint,
                    gl::TEXTURE_1D,
                    texture.id(),
                    0,
                ),
                Dimension::D2 => gl::FramebufferTexture2D(
                    gl::FRAMEBUFFER,
                    gl::COLOR_ATTACHMENT0 + attachment as GLuint,
                    gl::TEXTURE_2D,
                    texture.id(),
                    0,
                ),
                Dimension::D3 => gl::FramebufferTexture3D(
                    gl::FRAMEBUFFER,
                    gl::COLOR_ATTACHMENT0 + attachment as GLuint,
                    gl::TEXTURE_3D,
                    texture.id(),
                    0,
                    0,
                ),
                _ => panic!("Only 1D, 2D or 3D textures can be bound to framebuffers"),
            }
        })
    }

    pub fn attach_depth<D>(
        &mut self,
        texture: &BoundTexture<DepthStencil<D, ()>>,
    ) -> anyhow::Result<()> {
        gl_error_guard(|| unsafe {
            match texture.dimension() {
                Dimension::D1 => gl::FramebufferTexture2D(
                    gl::FRAMEBUFFER,
                    gl::DEPTH_ATTACHMENT,
                    gl::TEXTURE_1D,
                    texture.id(),
                    0,
                ),
                Dimension::D2 => gl::FramebufferTexture2D(
                    gl::FRAMEBUFFER,
                    gl::DEPTH_ATTACHMENT,
                    gl::TEXTURE_2D,
                    texture.id(),
                    0,
                ),
                Dimension::D3 => gl::FramebufferTexture3D(
                    gl::FRAMEBUFFER,
                    gl::DEPTH_ATTACHMENT,
                    gl::TEXTURE_3D,
                    texture.id(),
                    0,
                    0,
                ),
                _ => panic!("Only 1D, 2D or 3D texture can be attached into the depth slot"),
            }
        })
    }

    pub fn attach_depth_stencil<D, S>(
        &mut self,
        texture: &BoundTexture<DepthStencil<D, S>>,
    ) -> anyhow::Result<()> {
        gl_error_guard(|| unsafe {
            match texture.dimension() {
                Dimension::D1 => gl::FramebufferTexture2D(
                    gl::FRAMEBUFFER,
                    gl::DEPTH_STENCIL_ATTACHMENT,
                    gl::TEXTURE_1D,
                    texture.id(),
                    0,
                ),
                Dimension::D2 => gl::FramebufferTexture2D(
                    gl::FRAMEBUFFER,
                    gl::DEPTH_STENCIL_ATTACHMENT,
                    gl::TEXTURE_2D,
                    texture.id(),
                    0,
                ),
                Dimension::D3 => gl::FramebufferTexture3D(
                    gl::FRAMEBUFFER,
                    gl::DEPTH_STENCIL_ATTACHMENT,
                    gl::TEXTURE_3D,
                    texture.id(),
                    0,
                    0,
                ),
                _ => panic!("Only 1D, 2D or 3D texture can be attached into the depth slot"),
            }
        })
    }
}

fn normalize_range<B: RangeBounds<i32>>(bounds: B, limit: Range<i32>) -> Range<i32> {
    use std::ops::Bound;

    let start = match bounds.start_bound() {
        Bound::Included(&i) => i,
        Bound::Excluded(&i) => i + 1,
        Bound::Unbounded => limit.start,
    };
    let end = match bounds.end_bound() {
        Bound::Included(&i) => i + 1,
        Bound::Excluded(&i) => i,
        Bound::Unbounded => limit.end,
    };
    (start.max(limit.start))..(end.min(limit.end))
}
