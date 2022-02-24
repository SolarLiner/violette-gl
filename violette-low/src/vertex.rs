use std::{
    marker::PhantomData,
    mem::ManuallyDrop,
    num::NonZeroU32,
    ops::{Range, RangeBounds},
};

use crate::{
    base::{
        bindable::{BindableExt, Binding, Resource},
        GlType,
    },
    buffer::Buffer,
    program::ActiveProgram,
    utils::gl_error_guard,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct VaoId(NonZeroU32);

impl std::ops::Deref for VaoId {
    type Target = NonZeroU32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl VaoId {
    pub fn new(id: u32) -> Option<Self> {
        Some(Self(NonZeroU32::new(id)?))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
#[non_exhaustive]
pub enum DrawMode {
    TrianglesList = gl::TRIANGLES,
}

#[derive(Debug)]
pub struct VertexArray {
    draw_mode: DrawMode,
    id: VaoId,
}

impl<'a> Resource<'a> for VertexArray {
    type Id = VaoId;
    type Kind = ();
    type Bound = BoundVao<'a>;

    fn current(_: Self::Kind) -> Option<Self::Id> {
        let mut id = 0;
        unsafe {
            gl::GetIntegerv(gl::VERTEX_ARRAY_BINDING, &mut id);
        }
        VaoId::new(id as _)
    }

    fn kind(&self) -> Self::Kind {
        ()
    }

    fn make_binding(&'a mut self) -> anyhow::Result<Self::Bound> {
        tracing::trace!("glBindVertexArray({})", self.id.get());
        unsafe {
            gl::BindVertexArray(self.id.get());
        }
        Ok(BoundVao { vao: self })
    }
}

impl VertexArray {
    pub fn new(draw_mode: DrawMode) -> Self {
        let mut id = 0;
        unsafe {
            gl::GenVertexArrays(1, &mut id);
        }
        Self {
            id: VaoId::new(id).unwrap(),
            draw_mode,
        }
    }
}

#[derive(Debug)]
pub struct BoundVao<'a> {
    vao: &'a mut VertexArray,
}

impl<'a> std::ops::Deref for BoundVao<'a> {
    type Target = VertexArray;

    fn deref(&self) -> &Self::Target {
        self.vao
    }
}

impl<'a> std::ops::DerefMut for BoundVao<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.vao
    }
}

impl<'a> Binding<'a> for BoundVao<'a> {
    type Parent = VertexArray;

    fn unbind(&mut self, previous: Option<<VertexArray as Resource<'a>>::Id>) {
        tracing::trace!("glBindVertexArray(<previous>)");
        unsafe {
            gl::BindVertexArray(previous.map(|id| id.get()).unwrap_or(0));
        }
    }
}

impl<'a> BoundVao<'a> {
    pub fn set_vertex_attributes<V: VertexAttributes>(&mut self) {
        unsafe { V::vertex_attributes() }
    }

    pub fn enable_vertex_attribute(&mut self, index: usize) {
        unsafe {
            gl::EnableVertexAttribArray(index as _);
        }
    }

    pub fn with_vertex_buffer<V: AsVertexAttributes>(
        &mut self,
        vertex_buffer: Buffer<V>,
    ) -> anyhow::Result<()> {
        let mut vertex_buffer = ManuallyDrop::new(vertex_buffer);
        gl_error_guard(|| {
            let _vbuf_bind = vertex_buffer.bind();
            self.set_vertex_attributes::<V::Attr>();
            for i in 0..V::Attr::COUNT {
                self.enable_vertex_attribute(i as _);
            }
        })
    }

    pub fn draw(&mut self, _prog_binding: &ActiveProgram, range: Range<i32>) {
        tracing::trace!(
            "glDrawArrays({:?}, {}, {})",
            self.draw_mode,
            range.start,
            range.end
        );
        unsafe {
            gl::DrawArrays(self.draw_mode as _, range.start, range.end);
        }
    }

    pub fn draw_indexed<I: GlType>(
        &mut self,
        _prog_binding: &ActiveProgram,
        _ibuf_binding: &Buffer<I>,
    ) {
        tracing::trace!(
            "glDrawElements({:?}, {}, {:x} ({}), 0)",
            self.draw_mode,
            _ibuf_binding.len(),
            I::GL_TYPE,
            std::any::type_name::<I>()
        );
        unsafe {
            gl::DrawElements(
                self.draw_mode as _,
                _ibuf_binding.len() as _,
                I::GL_TYPE,
                std::ptr::null(),
            );
        }
    }
}

pub trait VertexAttributes {
    const COUNT: usize;

    unsafe fn vertex_attributes();
}

impl<T: GlType> VertexAttributes for T {
    const COUNT: usize = 1;

    unsafe fn vertex_attributes() {
        gl::VertexAttribPointer(
            0,
            T::NUM_COMPONENTS as _,
            T::GL_TYPE,
            if T::NORMALIZED { gl::TRUE } else { gl::FALSE },
            T::STRIDE as _,
            std::ptr::null(),
        );
    }
}

impl<A: GlType, B: GlType> VertexAttributes for (A, B) {
    const COUNT: usize = 2;

    unsafe fn vertex_attributes() {
        gl::VertexAttribPointer(
            0,
            A::NUM_COMPONENTS as _,
            A::GL_TYPE,
            if A::NORMALIZED { gl::TRUE } else { gl::FALSE },
            (A::STRIDE + B::STRIDE) as _,
            std::ptr::null(),
        );
        gl::VertexAttribPointer(
            1,
            B::NUM_COMPONENTS as _,
            B::GL_TYPE,
            if B::NORMALIZED { gl::TRUE } else { gl::FALSE },
            (A::STRIDE + B::STRIDE) as _,
            A::STRIDE as _,
        );
    }
}

impl<A: GlType, B: GlType, C: GlType> VertexAttributes for (A, B, C) {
    const COUNT: usize = 3;

    unsafe fn vertex_attributes() {
        gl::VertexAttribPointer(
            0,
            A::NUM_COMPONENTS as _,
            A::GL_TYPE,
            if A::NORMALIZED { gl::TRUE } else { gl::FALSE },
            (A::STRIDE + B::STRIDE + C::STRIDE) as _,
            std::ptr::null(),
        );
        gl::VertexAttribPointer(
            1,
            B::NUM_COMPONENTS as _,
            B::GL_TYPE,
            if B::NORMALIZED { gl::TRUE } else { gl::FALSE },
            (A::STRIDE + B::STRIDE + C::STRIDE) as _,
            A::STRIDE as _,
        );
        gl::VertexAttribPointer(
            2,
            C::NUM_COMPONENTS as _,
            C::GL_TYPE,
            if C::NORMALIZED { gl::TRUE } else { gl::FALSE },
            (A::STRIDE + B::STRIDE + C::STRIDE) as _,
            (A::STRIDE + B::STRIDE) as _,
        );
    }
}

impl<A: GlType, B: GlType, C: GlType, D: GlType> VertexAttributes for (A, B, C, D) {
    const COUNT: usize = 4;

    unsafe fn vertex_attributes() {
        gl::VertexAttribPointer(
            0,
            A::NUM_COMPONENTS as _,
            A::GL_TYPE,
            if A::NORMALIZED { gl::TRUE } else { gl::FALSE },
            (A::STRIDE + B::STRIDE + C::STRIDE + D::STRIDE) as _,
            std::ptr::null(),
        );
        gl::VertexAttribPointer(
            1,
            B::NUM_COMPONENTS as _,
            B::GL_TYPE,
            if B::NORMALIZED { gl::TRUE } else { gl::FALSE },
            (A::STRIDE + B::STRIDE + C::STRIDE + D::STRIDE) as _,
            A::STRIDE as _,
        );
        gl::VertexAttribPointer(
            2,
            C::NUM_COMPONENTS as _,
            C::GL_TYPE,
            if C::NORMALIZED { gl::TRUE } else { gl::FALSE },
            (A::STRIDE + B::STRIDE + C::STRIDE + D::STRIDE) as _,
            (A::STRIDE + B::STRIDE) as _,
        );
        gl::VertexAttribPointer(
            3,
            D::NUM_COMPONENTS as _,
            D::GL_TYPE,
            if D::NORMALIZED { gl::TRUE } else { gl::FALSE },
            (A::STRIDE + B::STRIDE + C::STRIDE + D::STRIDE) as _,
            (A::STRIDE + B::STRIDE + C::STRIDE) as _,
        );
    }
}

pub trait AsVertexAttributes {
    type Attr: VertexAttributes;
}

impl<V: VertexAttributes> AsVertexAttributes for V {
    type Attr = V;
}
