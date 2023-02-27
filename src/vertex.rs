use std::{
    fmt::{self, Formatter},
    num::NonZeroU32,
};
use std::marker::PhantomData;

use crate::{
    base::{
        resource::{Resource, ResourceExt},
        GlType,
    },
    buffer::ArrayBuffer,
    utils::gl_error_guard,
};

use eyre::Result;
use gl::types::GLenum;

use crate::buffer::ElementBuffer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VertexDesc {
    pub num_components: usize,
    pub raw_type: GLenum,
    pub normalized: bool,
    pub offset: usize,
}

impl VertexDesc {
    pub const fn from_gl_type<T: GlType>(offset: usize) -> Self {
        Self {
            num_components: T::NUM_COMPONENTS,
            raw_type: T::GL_TYPE,
            normalized: T::NORMALIZED,
            offset,
        }
    }

    pub const fn normalized(mut self) -> Self {
        self.normalized = true;
        self
    }
}

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
    __non_send: PhantomData<*mut ()>,
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
        unsafe { gl::BindVertexArray(self.id.get()) }
    }

    fn unbind(&self) {
        unsafe { gl::BindVertexArray(0) }
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
            __non_send: PhantomData,
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
    pub fn set_vertex_attributes<V>(&mut self) -> Result<usize>
    where
        V: VertexAttributes,
    {
        gl_error_guard(|| {
            self.with_binding(|| {
                let attr = V::attributes();
                for (i, el) in attr.iter().enumerate() {
                    unsafe {
                        gl::VertexAttribPointer(
                            i as _,
                            el.num_components as _,
                            el.raw_type,
                            if el.normalized { gl::TRUE } else { gl::FALSE },
                            std::mem::size_of::<V>() as _,
                            el.offset as *const _,
                        );
                    }
                }
                attr.len()
            })
        })
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

    pub fn with_vertex_buffer<V: 'static>(&mut self, vertex_buffer: &ArrayBuffer<V>) -> Result<()>
    where
        V: VertexAttributes,
    {
        gl_error_guard(|| {
            self.bind();
            vertex_buffer.bind();
            let attrib_count = self.set_vertex_attributes::<V>()?;
            for i in 0..attrib_count {
                self.enable_vertex_attribute(i as _);
            }
            self.unbind();
            vertex_buffer.unbind();
            Ok(())
        })
        .and_then(|r| r)
    }

    pub fn with_element_buffer<T: GlType>(
        &mut self,
        element_buffer: &ElementBuffer<T>,
    ) -> Result<()> {
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

pub trait VertexAttributes: Sized + bytemuck::Pod {
    fn attributes() -> &'static [VertexDesc];
}

impl<T: GlType + bytemuck::Pod> VertexAttributes for T {
    fn attributes() -> &'static [VertexDesc] {
        vec![VertexDesc::from_gl_type::<T>(0)].leak()
    }
}
