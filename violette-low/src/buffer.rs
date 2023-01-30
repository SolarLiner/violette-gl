use std::num::NonZeroUsize;
use std::ops::Range;
use std::{
    marker::PhantomData,
    num::NonZeroU32,
    ops::{Bound, RangeBounds},
};

use bitflags::bitflags;
use bytemuck::{cast_slice, Pod};
use gl::types::{GLbitfield, GLintptr, GLsizeiptr, GLuint};
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
    count: usize,
}

impl<T> Buffer<T> {
    pub(crate) unsafe fn from_id(id: BufferId) -> Self {
        let size = {
            let mut size = 0;
            gl::GetNamedBufferParameteriv(id.get(), gl::BUFFER_SIZE, &mut size);
            size
        };
        Self {
            __type: PhantomData,
            id,
            count: size as usize / std::mem::size_of::<T>(),
        }
    }
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
        assert!(std::mem::size_of::<T>() > 0, "Cannot allocate buffers for zero-sized types");
        let id = unsafe {
            let mut id = 0;
            gl::GenBuffers(1, &mut id);
            id
        };
        Self {
            __type: PhantomData,
            id: BufferId::new(id, kind).unwrap(),
            count: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn kind(&self) -> BufferKind {
        self.id.kind
    }
}

impl<T: Pod> Buffer<T> {
    pub fn with_data(kind: BufferKind, data: &[T]) -> anyhow::Result<Self> {
        assert!(std::mem::size_of::<T>() > 0, "Cannot allocate buffers for zero-sized types");
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
        const MAP_READ = gl::MAP_READ_BIT;
        const MAP_WRITE = gl::MAP_WRITE_BIT;
    }
}

#[derive(Debug)]
/// Bound OpenGL buffer.
pub struct BoundBuffer<'a, T> {
    buffer: &'a mut Buffer<T>,
}

impl<'a, T> BoundBuffer<'a, T> {
    fn byte_slice(&self, sizeof: usize, range: impl RangeBounds<usize>) -> Range<usize> {
        tracing::debug!(range.start = ?range.start_bound(), range.end = ?range.end_bound());
        let uniform_align = gl_alignment();
        let alignment = next_multiple(sizeof, uniform_align);
        let start = match range.start_bound() {
            Bound::Included(i) => *i,
            Bound::Excluded(i) => (i + 1),
            Bound::Unbounded => 0,
        } * alignment;
        let end = match range.end_bound() {
            Bound::Included(i) => (i + 1),
            Bound::Excluded(i) => *i,
            Bound::Unbounded => self.count * std::mem::size_of::<T>(),
        } * alignment;
        start..end
    }
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
        let bytes = if self.buffer.kind() == BufferKind::Uniform {
            let alignment = next_multiple(std::mem::size_of::<T>(), gl_alignment());
            data.iter()
                .flat_map(|x| {
                    let bytes = bytemuck::bytes_of(x);
                    let padding = alignment - bytes.len();
                    bytes
                        .iter()
                        .copied()
                        .chain(std::iter::repeat(0).take(padding))
                })
                .collect::<Vec<_>>()
        } else {
            bytemuck::cast_slice(data).to_owned()
        };
        self.buffer.count = data.len();
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

    pub fn slice(&self, range: impl RangeBounds<usize>) -> BufferSlice<'a, '_, T> {
        let range = self.byte_slice(std::mem::size_of::<T>(), range);
        let offset = range.start as _;
        let size = (range.end - range.start) as _;
        tracing::debug!(?range, %size, %offset);
        BufferSlice {
            bound_buffer: self,
            offset,
            size,
        }
    }

    pub fn slice_mut(&mut self, range: impl RangeBounds<usize>) -> BufferSliceMut<'a, '_, T> {
        let range = self.byte_slice(std::mem::size_of::<T>(), range);
        let offset = range.start as _;
        let size = (range.end - range.start) as _;
        BufferSliceMut {
            bound_buffer: self,
            offset,
            size,
        }
    }
}

pub struct BufferSlice<'a, 'b, T> {
    pub(crate) bound_buffer: &'b BoundBuffer<'a, T>,
    pub(crate) offset: GLintptr,
    pub(crate) size: GLsizeiptr,
}

impl<'a, 'b, T: bytemuck::Pod> BufferSlice<'a, 'b, T> {
    pub fn read(&self, access: BufferAccess) -> anyhow::Result<MappedBufferData<T>> {
        let bytes = gl_error_guard(|| unsafe {
            let access = access & !BufferAccess::MAP_WRITE;
            let ptr = gl::MapBufferRange(
                self.bound_buffer.buffer.kind() as _,
                self.offset,
                self.size,
                access.bits,
            );
            std::slice::from_raw_parts(ptr as *const u8, self.size as _)
        })?;
        Ok(MappedBufferData {
            __ty: PhantomData,
            id: self.bound_buffer.id,
            data: cast_slice(bytes),
        })
    }
}

pub struct BufferSliceMut<'a, 'b, T> {
    pub(crate) bound_buffer: &'b mut BoundBuffer<'a, T>,
    pub(crate) offset: GLintptr,
    pub(crate) size: GLsizeiptr,
}

impl<'a, 'b, T: bytemuck::Pod> BufferSliceMut<'a, 'b, T> {
    pub fn write(&mut self, data: &[T], access: BufferAccess) -> anyhow::Result<()> {
        anyhow::ensure!(
            data.len() * std::mem::size_of::<T>() == self.size as _,
            "Slice length need to equal mapped slice length"
        );
        let bytes = bytemuck::cast_slice(data);
        gl_error_guard(|| unsafe {
            let access = access | BufferAccess::MAP_READ | BufferAccess::MAP_WRITE;
            let ptr = gl::MapBufferRange(
                self.bound_buffer.kind() as _,
                self.offset,
                self.size,
                access.bits,
            );
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr as *mut u8, self.size as _);
            gl::UnmapBuffer(self.bound_buffer.id.get());
        })
    }
}

#[derive(Debug)]
/// Mapped buffer data from OpenGL.
pub struct MappedBufferData<'m, 'b, T> {
    __ty: PhantomData<&'b ()>,
    id: BufferId,
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
            gl::UnmapBuffer(self.id.kind as _);
        }
    }
}

#[cfg(not(feature = "fast"))]
#[tracing::instrument]
fn gl_alignment() -> NonZeroUsize {
    NonZeroUsize::new(
        gl_error_guard(|| unsafe {
            let mut val = 0;
            gl::GetIntegerv(gl::UNIFORM_BUFFER_OFFSET_ALIGNMENT, &mut val);
            tracing::trace!(
                "glGetIntegerv(GL_UNIFORM_BUFFER_OFFSET_ALIGNMENT, <inout val={}>)",
                val
            );
            val as usize
        })
        .unwrap(),
    )
    .unwrap()
}

#[cfg(feature = "fast")]
#[tracing::instrument]
fn gl_alignment() -> NonZeroUsize {
    unsafe {
        let mut val = 0;
        gl::GetIntegerv(gl::UNIFORM_BUFFER_OFFSET_ALIGNMENT, &mut val);
        tracing::trace!(
            "glGetIntegerv(GL_UNIFORM_BUFFER_OFFSET_ALIGNMENT, <inout val={}>)",
            val
        );
        NonZeroUsize::new_unchecked(val as _)
    }
}

#[inline(always)]
fn next_multiple(x: usize, of: NonZeroUsize) -> usize {
    let rem = x % of.get();
    let offset = of.get() - rem;
    x + offset
}
