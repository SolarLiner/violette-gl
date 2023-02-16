use std::{
    fmt::{
        Formatter,
        self
    },
    num::NonZeroU32
};

use crate::{
    base::{
        resource::{Resource},
        GlType,
    },
    utils::gl_error_guard,
    base::resource::ResourceExt,
    buffer::ArrayBuffer
};

use eyre::Result;
use gl::types::{GLenum};

use crate::buffer::ElementBuffer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct VaoId(NonZeroU32);

impl fmt::Display for VaoId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.get())
    }
}

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
    Points = gl::POINTS,
    Triangles = gl::TRIANGLES,
    Lines = gl::LINES,
    LineLoop = gl::LINE_LOOP,
    LineStrip = gl::LINE_STRIP,
}

#[derive(Debug)]
pub struct VertexArray {
    id: VaoId,
    pub(crate) element: Option<GLenum>,
}

impl<'a> Resource<'a> for VertexArray {
    type Id = VaoId;

    fn id(&self) -> Self::Id {
        self.id
    }

    fn current() -> Option<Self::Id> {
        let mut id = 0;
        unsafe {
            gl::GetIntegerv(gl::VERTEX_ARRAY_BINDING, &mut id);
        }
        VaoId::new(id as _)
    }

    fn bind(&self) {
        unsafe {gl::BindVertexArray(self.id.get())}
    }

    fn unbind(&self) {
        unsafe {gl::BindVertexArray(0)}
    }
}

#[allow(clippy::new_without_default)]
impl VertexArray {
    pub fn new() -> Self {
        let mut id = 0;
        unsafe {
            gl::GenVertexArrays(1, &mut id);
        }
        Self {
            id: VaoId::new(id).unwrap(),
            element: None,
        }
    }
}

impl Drop for VertexArray {
    fn drop(&mut self) {
        unsafe {
            let vid = self.id.get();
            gl::DeleteVertexArrays(1, &vid);
        }
    }
}

impl VertexArray {
    pub fn set_vertex_attributes<V: VertexAttributes>(&mut self) -> Result<()> {
        gl_error_guard(|| self.with_binding(|| unsafe { V::vertex_attributes() }))
    }

    pub fn enable_vertex_attribute(&mut self, index: usize) {
        self.with_binding(|| unsafe {
            gl::EnableVertexAttribArray(index as _);
        })
    }

    pub fn disable_vertex_attribute(&mut self, index: usize) {
        self.with_binding(|| unsafe {
            gl::DisableVertexAttribArray(index as _);
        })
    }

    pub fn with_vertex_buffer<V: 'static + AsVertexAttributes>(
        &mut self,
        vertex_buffer: &ArrayBuffer<V>,
    ) -> Result<()> {
        gl_error_guard(|| {
            self.bind();
            vertex_buffer.bind();
            unsafe { V::Attr::vertex_attributes(); }
            for i in 0..V::Attr::COUNT {
                self.enable_vertex_attribute(i as _);
            }
            self.unbind();
            vertex_buffer.unbind();
        })
    }

    pub fn with_element_buffer<T: GlType>(&mut self, element_buffer: &ElementBuffer<T>) -> Result<()> {
        gl_error_guard(|| {
            self.bind();
            element_buffer.bind();
            self.unbind();
            element_buffer.unbind();
        })?;
        self.element.replace(T::GL_TYPE);
        Ok(())
    }
}

pub trait VertexAttributes {
    const COUNT: usize;

    /// Load vertex attributes.
    /// # Safety
    /// This function is unsafe because it is directly talking to OpenGL. Implementers *should*
    /// assume that a VAO is bound, and callees *must* check for errors here.
    /// This function is also unsafe because it has the responsibility of correctly telling OpenGL
    /// how to interpret the binary data sent to it for drawing. As such, implementers must make sure
    /// that the type is correctly described by the attributes described within this function call.
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
