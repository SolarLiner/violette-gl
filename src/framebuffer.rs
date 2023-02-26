use std::{
    fmt::{self, Formatter},
    ops::{Range, RangeBounds},
};

use bitflags::bitflags;
use eyre::Result;
use gl::types::*;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

use crate::{
    base::resource::{Resource, ResourceExt},
    program::Program,
    texture::{DepthStencil, Dimension, Texture},
    utils::{gl_error_guard, GlRef},
    vertex::{DrawMode, VertexArray},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FramebufferId(u32);

impl fmt::Display for FramebufferId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

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

    // fn get(&self) -> u32 {
    //     self.0
    // }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive)]
#[repr(u32)]
pub enum FramebufferStatus {
    Undefined = gl::FRAMEBUFFER_UNDEFINED,
    IncompleteAttachment = gl::FRAMEBUFFER_INCOMPLETE_ATTACHMENT,
    MissingAttachment = gl::FRAMEBUFFER_INCOMPLETE_MISSING_ATTACHMENT,
    IncompleteDrawBuffer = gl::FRAMEBUFFER_INCOMPLETE_DRAW_BUFFER,
    IncompleteReadBuffer = gl::FRAMEBUFFER_INCOMPLETE_READ_BUFFER,
    Unsupported = gl::FRAMEBUFFER_UNSUPPORTED,
    IncompleteMultisample = gl::FRAMEBUFFER_INCOMPLETE_MULTISAMPLE,
    IncompleteLayerTargets = gl::FRAMEBUFFER_INCOMPLETE_LAYER_TARGETS,
    Complete = gl::FRAMEBUFFER_COMPLETE,
}

#[derive(Debug)]
pub struct Framebuffer {
    id: FramebufferId,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, FromPrimitive)]
#[repr(u32)]
pub enum BlendFunction {
    Add = gl::FUNC_ADD,
    Subtract = gl::FUNC_SUBTRACT,
    RevSubtract = gl::FUNC_REVERSE_SUBTRACT,
    Min = gl::MIN,
    Max = gl::MAX,
}

impl Framebuffer {
    pub fn blend_equation(func: BlendFunction) {
        unsafe {
            gl::BlendEquation(func as _);
        }
    }
}

impl std::ops::Deref for Framebuffer {
    type Target = FramebufferId;

    fn deref(&self) -> &Self::Target {
        &self.id
    }
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        unsafe { gl::DeleteFramebuffers(1, &*self.id) }
    }
}

impl Framebuffer {
    pub const fn backbuffer() -> GlRef<'static, Self> {
        GlRef::create(Self {
            id: FramebufferId::BACKBUFFER,
        })
    }

    pub fn new() -> Framebuffer {
        let id = unsafe {
            let mut fbo = 0;
            gl::GenFramebuffers(1, &mut fbo);
            fbo
        };
        tracing::debug!("Create framebuffer {}", id);
        Self {
            id: FramebufferId::new(id).unwrap(),
        }
    }
}

impl<'a> Resource<'a> for Framebuffer {
    type Id = FramebufferId;

    fn id(&self) -> Self::Id {
        self.id
    }

    fn current() -> Option<Self::Id> {
        let mut id = 0;
        unsafe {
            gl::GetIntegerv(gl::FRAMEBUFFER_BINDING, &mut id);
        }
        Some(FramebufferId(id as _))
    }

    fn bind(&self) {
        tracing::trace!("Bind framebuffer {}", self.id);
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.id.0 as _);
        }
    }

    fn unbind(&self) {
        if self.id == FramebufferId::BACKBUFFER {
            return;
        }
        tracing::trace!("Unbind framebuffer {}", self.id);
        unsafe { gl::BindFramebuffer(gl::FRAMEBUFFER, 0) }
    }
}

impl Framebuffer {
    pub fn get_viewport() -> [i32; 4] {
        let mut viewport = [0; 4];
        unsafe {
            gl::GetIntegerv(gl::VIEWPORT, viewport.as_mut_ptr());
        }
        viewport
    }
    pub fn viewport(x: i32, y: i32, width: i32, height: i32) {
        unsafe {
            gl::Viewport(x, y, width, height);
        }
    }

    pub fn clear_color([red, green, blue, alpha]: [f32; 4]) {
        unsafe { gl::ClearColor(red, green, blue, alpha) }
    }

    pub fn clear_depth(value: f64) {
        unsafe {
            gl::ClearDepth(value);
        }
    }

    pub fn do_clear(&self, mode: ClearBuffer) {
        self.with_binding(|| unsafe {
            gl::Clear(mode.bits());
        })
    }

    pub fn enable_depth_test(func: DepthTestFunction) {
        unsafe {
            gl::DepthFunc(func as _);
            gl::Enable(gl::DEPTH_TEST);
        }
    }

    pub fn disable_depth_test() {
        unsafe { gl::Disable(gl::DEPTH_TEST) };
    }

    pub fn enable_blending(source: Blend, target: Blend) {
        unsafe {
            gl::BlendFunc(source as _, target as _);
            gl::Enable(gl::BLEND);
        }
    }

    pub fn disable_blending() {
        unsafe {
            gl::BlendFunc(gl::ONE, gl::ZERO);
            gl::Disable(gl::BLEND);
        }
    }

    pub fn enable_scissor(x: i32, y: i32, w: i32, h: i32) {
        unsafe {
            gl::Enable(gl::SCISSOR_TEST);
            gl::Scissor(x, y, w, h);
        }
    }

    pub fn disable_scissor() {
        unsafe { gl::Disable(gl::SCISSOR_TEST) }
    }

    pub fn draw(
        &self,
        program: &Program,
        vao: &VertexArray,
        mode: DrawMode,
        vertices: Range<i32>,
    ) -> Result<()> {
        tracing::debug!(
            "Draw on FBO {} with program {} and VAO {}",
            self.id,
            program.id(),
            vao.id()
        );
        gl_error_guard(|| {
            program.with_binding(|| {
                self.with_binding(|| {
                    vao.with_binding(|| unsafe {
                        gl::DrawArrays(mode as _, vertices.start, vertices.end - vertices.start);
                    })
                })
            })
        })
    }

    pub fn draw_elements(
        &self,
        program: &Program,
        vao: &VertexArray,
        mode: DrawMode,
        slice: Range<i32>,
    ) -> Result<()> {
        let Some(gl_type) = vao.element else { eyre::bail!( "Vertex Array Object needs to be bound to an Element Buffer") };
        tracing::trace!(
            "Draw elements on FBO {} with program {} and VAO {}",
            self.id,
            program.id(),
            vao.id()
        );
        let count = slice.end - slice.start.max(0);
        gl_error_guard(|| {
            self.with_binding(|| {
                program.with_binding(|| {
                    vao.with_binding(|| unsafe {
                        gl::DrawElements(mode as _, count, gl_type, slice.start as _);
                    })
                })
            })
        })
    }

    pub fn attach_color<F>(&self, attachment: u8, texture: &Texture<F>) -> Result<()> {
        tracing::trace!("glFramebufferTexture{}D(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT_{}, GL_TEXTURE_{}D, {}, 0)",
            texture.dimension().num_dimension(), attachment, texture.dimension().num_dimension(), texture.raw_id());
        self.with_binding(|| {
            gl_error_guard(|| unsafe {
                gl::FramebufferTexture(
                    gl::FRAMEBUFFER,
                    gl::COLOR_ATTACHMENT0 + attachment as GLenum,
                    texture.raw_id(),
                    0,
                );
            })
        })
    }

    pub fn attach_depth<D, S>(&self, texture: &Texture<DepthStencil<D, S>>) -> Result<()> {
        tracing::trace!(
            "glFramebufferTexture2D(GL_FRAMEBUFFER, GL_DEPTH_ATTACHMENT, GL_TEXTURE_{}D, {}, 0)",
            texture.dimension().num_dimension(),
            texture.raw_id()
        );
        gl_error_guard(|| {
            self.with_binding(|| unsafe {
                match texture.dimension() {
                    Dimension::D1 => gl::FramebufferTexture2D(
                        gl::FRAMEBUFFER,
                        gl::DEPTH_ATTACHMENT,
                        gl::TEXTURE_1D,
                        texture.raw_id(),
                        0,
                    ),
                    Dimension::D2 => gl::FramebufferTexture2D(
                        gl::FRAMEBUFFER,
                        gl::DEPTH_ATTACHMENT,
                        gl::TEXTURE_2D,
                        texture.raw_id(),
                        0,
                    ),
                    Dimension::D3 => gl::FramebufferTexture3D(
                        gl::FRAMEBUFFER,
                        gl::DEPTH_ATTACHMENT,
                        gl::TEXTURE_3D,
                        texture.raw_id(),
                        0,
                        0,
                    ),
                    _ => panic!("Only 1D, 2D or 3D texture can be attached into the depth slot"),
                }
            })
        })
    }

    pub fn attach_depth_stencil<D, S>(
        &mut self,
        texture: &Texture<DepthStencil<D, S>>,
    ) -> Result<()> {
        gl_error_guard(|| {
            self.with_binding(|| unsafe {
                match texture.dimension() {
                    Dimension::D1 => gl::FramebufferTexture2D(
                        gl::FRAMEBUFFER,
                        gl::DEPTH_STENCIL_ATTACHMENT,
                        gl::TEXTURE_1D,
                        texture.raw_id(),
                        0,
                    ),
                    Dimension::D2 => gl::FramebufferTexture2D(
                        gl::FRAMEBUFFER,
                        gl::DEPTH_STENCIL_ATTACHMENT,
                        gl::TEXTURE_2D,
                        texture.raw_id(),
                        0,
                    ),
                    Dimension::D3 => gl::FramebufferTexture3D(
                        gl::FRAMEBUFFER,
                        gl::DEPTH_STENCIL_ATTACHMENT,
                        gl::TEXTURE_3D,
                        texture.raw_id(),
                        0,
                        0,
                    ),
                    _ => panic!("Only 1D, 2D or 3D texture can be attached into the depth slot"),
                }
            })
        })
    }

    pub fn enable_buffers(&self, attachments: impl IntoIterator<Item = u32>) -> Result<()> {
        let symbols = attachments
            .into_iter()
            .map(|ix| gl::COLOR_ATTACHMENT0 + ix)
            .collect::<Vec<_>>();
        gl_error_guard(|| {
            self.with_binding(|| unsafe {
                gl::DrawBuffers(symbols.len() as _, symbols.as_ptr());
            })
        })
    }

    pub fn check_status(&self) -> FramebufferStatus {
        self.with_binding(|| {
            let value = unsafe { gl::CheckFramebufferStatus(gl::DRAW_FRAMEBUFFER) };
            FramebufferStatus::from_u32(value).unwrap()
        })
    }

    pub fn assert_complete(&self) -> Result<()> {
        match self.check_status() {
            FramebufferStatus::Complete => Ok(()),
            status => eyre::bail!("Framebuffer not valid: {:?}", status),
        }
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
