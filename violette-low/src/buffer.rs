use std::{
    marker::PhantomData,
    num::NonZeroU32,
    ops::{Bound, RangeBounds},
};

use bitflags::bitflags;
use bytemuck::{cast_slice, Pod};
use gl::types::{GLbitfield, GLuint};
use num_derive::FromPrimitive;

use crate::{
    base::bindable::{BindableExt, Binding, Resource},
    utils::gl_error_guard,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferId {
    id: NonZeroU32,
    kind: BufferKind,
}

impl std::ops::Deref for BufferId {
    type Target = NonZeroU32;

    fn deref(&self) -> &Self::Target {
        &self.id
    }
}

impl BufferId {
    fn new(id: GLuint, kind: BufferKind) -> Option<Self> {
        Some(BufferId {
            id: NonZeroU32::new(id)?,
            kind,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive)]
#[repr(u32)]
pub enum BufferKind {
    Array = gl::ARRAY_BUFFER,
    AtomicCounter = gl::ATOMIC_COUNTER_BUFFER,
    CopyRead = gl::COPY_READ_BUFFER,
    CopyWrite = gl::COPY_WRITE_BUFFER,
    DispatchIndirect = gl::DISPATCH_INDIRECT_BUFFER,
    DrawIndirect = gl::DRAW_INDIRECT_BUFFER,
    ElementArray = gl::ELEMENT_ARRAY_BUFFER,
    PixelPack = gl::PIXEL_PACK_BUFFER,
    PixelUnpack = gl::PIXEL_UNPACK_BUFFER,
    Query = gl::QUERY_BUFFER,
    ShaderStorage = gl::SHADER_STORAGE_BUFFER,
    TextureBuffer = gl::TEXTURE_BUFFER,
    TransformFeedback = gl::TRANSFORM_FEEDBACK_BUFFER,
    Uniform = gl::UNIFORM_BUFFER,
}

impl BufferKind {
    const fn binding_const(&self) -> u32 {
        match self {
            Self::Array => gl::ARRAY_BUFFER_BINDING,
            Self::AtomicCounter => gl::ATOMIC_COUNTER_BUFFER_BINDING,
            Self::CopyRead => gl::COPY_READ_BUFFER_BINDING,
            Self::CopyWrite => gl::COPY_WRITE_BUFFER_BINDING,
            Self::DispatchIndirect => gl::DISPATCH_INDIRECT_BUFFER_BINDING,
            Self::DrawIndirect => gl::DRAW_INDIRECT_BUFFER_BINDING,
            Self::ElementArray => gl::ELEMENT_ARRAY_BUFFER_BINDING,
            Self::PixelPack => gl::PIXEL_PACK_BUFFER_BINDING,
            Self::PixelUnpack => gl::PIXEL_UNPACK_BUFFER_BINDING,
            Self::Query => gl::QUERY_BUFFER_BINDING,
            Self::ShaderStorage => gl::SHADER_STORAGE_BUFFER_BINDING,
            Self::TextureBuffer => gl::TEXTURE_BUFFER_BINDING,
            Self::TransformFeedback => gl::TRANSFORM_FEEDBACK_BUFFER_BINDING,
            Self::Uniform => gl::UNIFORM_BUFFER_BINDING,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
/// Buffer usage hint.
pub enum BufferUsageHint {
    Stream = gl::STREAM_DRAW,
    Static = gl::STATIC_DRAW,
    Dynamic = gl::DYNAMIC_DRAW,
}

#[derive(Debug)]
/// An OpenGL buffer on the GPU.
pub struct Buffer<T> {
    __type: PhantomData<T>,
    pub id: BufferId,
    length: usize,
}

impl<T> Drop for Buffer<T> {
    fn drop(&mut self) {
        unsafe { gl::DeleteBuffers(1, [self.id.get()].as_ptr()) }
    }
}

impl<'a, T: 'a> Resource<'a> for Buffer<T> {
    type Id = BufferId;
    type Kind = BufferKind;
    type Bound = BoundBuffer<'a, T>;

    fn current(kind: BufferKind) -> Option<Self::Id> {
        let mut id = 0;
        unsafe {
            gl::GetIntegerv(kind.binding_const(), &mut id);
        }
        BufferId::new(id as _, kind)
    }

    fn kind(&self) -> Self::Kind {
        self.id.kind
    }

    fn make_binding(&'a mut self) -> anyhow::Result<Self::Bound> {
        tracing::trace!("glBindBuffer({:?}, {})", self.kind(), self.id.get());
        unsafe {
            gl::BindBuffer(self.kind() as _, self.id.get() as _);
        }
        Ok(Self::Bound { buffer: self })
    }
}

impl<T> Buffer<T> {
    pub fn new(kind: BufferKind) -> Self {
        let id = unsafe {
            let mut id = 0;
            gl::GenBuffers(1, &mut id);
            id
        };
        Self {
            __type: PhantomData,
            id: BufferId::new(id, kind).unwrap(),
            length: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.length
    }

    pub fn kind(&self) -> BufferKind {
        self.id.kind
    }
}

impl<T: Pod> Buffer<T> {
    pub fn with_data(kind: BufferKind, data: &[T]) -> anyhow::Result<Self> {
        let mut this = Self::new(kind);
        this.with_binding(|binding| {
            binding.set(data, BufferUsageHint::Static)?;
            Ok(())
        })?;
        Ok(this)
    }
}

bitflags! {
    pub struct BufferAccess: GLbitfield {
        const PERSISTENT = gl::MAP_PERSISTENT_BIT;
        const COHERENT = gl::MAP_COHERENT_BIT;
    }
}

#[derive(Debug)]
/// Bound OpenGL buffer.
pub struct BoundBuffer<'a, T> {
    buffer: &'a mut Buffer<T>,
}

impl<'a, T> std::ops::Deref for BoundBuffer<'a, T> {
    type Target = Buffer<T>;

    fn deref(&self) -> &Self::Target {
        self.buffer
    }
}

impl<'a, T> std::ops::DerefMut for BoundBuffer<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.buffer
    }
}

impl<'a, T: 'a> Binding<'a> for BoundBuffer<'a, T> {
    type Parent = Buffer<T>;

    #[tracing::instrument(skip(self))]
    fn unbind(&mut self, previous: Option<<Buffer<T> as Resource<'a>>::Id>) {
        tracing::trace!("glBindBuffer({:?}, <previous>)", self.buffer.kind());
        unsafe {
            gl::BindBuffer(
                self.buffer.kind() as _,
                previous.map(|id| id.get()).unwrap_or(0),
            );
        }
    }
}

impl<'a, T: Pod> BoundBuffer<'a, T> {
    /// Sets GPU data.
    pub fn set(&mut self, data: &[T], usage_hint: BufferUsageHint) -> anyhow::Result<()> {
        let bytes = bytemuck::cast_slice::<_, u8>(data);
        self.buffer.length = data.len();
        tracing::trace!(
            "glBufferData({:?}, {}, <bytes ptr>, {:?})",
            self.buffer.kind(),
            bytes.len(),
            usage_hint
        );
        unsafe {
            gl::BufferData(
                self.buffer.kind() as _,
                bytes.len() as _,
                bytes.as_ptr() as *const _,
                usage_hint as _,
            );
        }
        Ok(())
    }

    /// Reads GPU data into a temporary CPU buffer, managed by OpenGL
    pub fn read(
        &self,
        range: impl RangeBounds<usize>,
        access: BufferAccess,
    ) -> anyhow::Result<MappedBufferData<'_, 'a, T>> {
        let start = match range.start_bound() {
            Bound::Included(i) => *i,
            Bound::Excluded(i) => (i + 1),
            Bound::Unbounded => 0,
        } * std::mem::size_of::<T>();
        let end = match range.end_bound() {
            Bound::Included(i) => *i,
            Bound::Excluded(i) => (i - 1),
            Bound::Unbounded => self.length,
        } * std::mem::size_of::<T>();
        let length = end - start;
        let bytes = gl_error_guard(|| unsafe {
            let ptr = gl::MapBufferRange(self.kind() as _, start as _, length as _, access.bits);
            let bytes = std::slice::from_raw_parts(ptr as *const u8, length);
            gl::UnmapBuffer(self.buffer.id.get());
            bytes
        })?;
        Ok(MappedBufferData {
            bound_buffer: self,
            data: cast_slice(bytes),
        })
    }
}

#[derive(Debug)]
/// Mapped buffer data from OpenGL.
pub struct MappedBufferData<'m, 'b, T> {
    bound_buffer: &'m BoundBuffer<'b, T>,
    data: &'m [T],
}

impl<'m, 'b, T> std::ops::Deref for MappedBufferData<'m, 'b, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<'m, 'b, T> Drop for MappedBufferData<'m, 'b, T> {
    fn drop(&mut self) {
        unsafe {
            gl::UnmapBuffer(self.bound_buffer.buffer.kind() as _);
        }
    }
}
