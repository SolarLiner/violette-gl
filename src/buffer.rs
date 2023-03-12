use std::{
    fmt::{self, Formatter},
    marker::PhantomData,
    num::{NonZeroU32, NonZeroUsize},
    ops::{Bound, Range, RangeBounds},
};

use bitflags::bitflags;
use bytemuck::Pod;
use eyre::Result;
use gl::types::{GLbitfield, GLintptr, GLsizeiptr, GLuint};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use once_cell::sync::Lazy;

use crate::{
    base::resource::{Resource, ResourceExt},
    utils::gl_error_guard,
};

pub type ArrayBuffer<T> = Buffer<T, { gl::ARRAY_BUFFER }>;
pub type ElementBuffer<T> = Buffer<T, { gl::ELEMENT_ARRAY_BUFFER }>;
pub type UniformBuffer<T> = Buffer<T, { gl::UNIFORM_BUFFER }>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferId<const K: u32> {
    id: NonZeroU32,
}

impl<const K: u32> fmt::Display for BufferId<K> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id.get())
    }
}

impl<const K: u32> std::ops::Deref for BufferId<K> {
    type Target = NonZeroU32;

    fn deref(&self) -> &Self::Target {
        &self.id
    }
}

impl<const K: u32> BufferId<K> {
    fn new(id: GLuint) -> Option<Self> {
        Some(BufferId {
            id: NonZeroU32::new(id)?,
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
pub struct Buffer<T, const K: u32> {
    __type: PhantomData<*mut T>,
    pub id: BufferId<K>,
    count: usize,
}

impl<T, const K: u32> Drop for Buffer<T, K> {
    fn drop(&mut self) {
        tracing::debug!("Delete buffer {}", self.id);
        unsafe { gl::DeleteBuffers(1, [self.id.get()].as_ptr()) }
    }
}

impl<'a, T: 'a, const K: u32> Resource<'a> for Buffer<T, K> {
    type Id = BufferId<K>;

    fn id(&self) -> Self::Id {
        self.id
    }

    fn current() -> Option<Self::Id> {
        let mut id = 0;
        unsafe {
            gl::GetIntegerv(BufferKind::from_u32(K).unwrap().binding_const(), &mut id);
        }
        BufferId::new(id as _)
    }

    fn bind(&self) {
        tracing::trace!(
            "glBindBuffer({:?}, {})",
            BufferKind::from_u32(K).unwrap(),
            self.id
        );
        unsafe { gl::BindBuffer(K, self.id.get() as _) };
    }

    fn unbind(&self) {
        tracing::trace!("glBindBuffer({:?}, 0)", BufferKind::from_u32(K).unwrap());
        unsafe { gl::BindBuffer(K, 0) };
    }
}

#[allow(clippy::new_without_default)]
impl<T, const K: u32> Buffer<T, K> {
    pub fn new() -> Self {
        assert!(
            std::mem::size_of::<T>() > 0,
            "Cannot allocate buffers for zero-sized types"
        );
        let id = unsafe {
            let mut id = 0;
            gl::GenBuffers(1, &mut id);
            id
        };
        tracing::debug!("Create buffer {}", id);
        Self {
            __type: PhantomData,
            id: BufferId::new(id).unwrap(),
            count: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn len(&self) -> usize {
        self.count
    }
}

impl<T: Pod, const K: u32> Buffer<T, K> {
    pub fn with_data(data: &[T]) -> Result<Self> {
        assert!(
            std::mem::size_of::<T>() > 0,
            "Cannot allocate buffers for zero-sized types"
        );
        let mut this = Self::new();
        this.set(data, BufferUsageHint::Static)?;
        Ok(this)
    }

    /// Sets GPU data.
    pub fn set(&mut self, data: &[T], usage_hint: BufferUsageHint) -> Result<()> {
        self.bind();
        let bytes = if K == BufferKind::Uniform as u32 {
            let sizeof = std::mem::size_of::<T>();
            let gl_alignment = *GL_ALIGNMENT;
            let alignment = next_multiple(sizeof, gl_alignment);
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
        self.count = data.len();
        tracing::trace!(
            "glBufferData({:?}, {}, <bytes ptr>, {:?})",
            BufferKind::from_u32(K).unwrap(),
            bytes.len(),
            usage_hint
        );
        gl_error_guard(|| unsafe {
            gl::BufferData(
                K,
                bytes.len() as _,
                bytes.as_ptr() as *const _,
                usage_hint as _,
            );
        })?;
        self.unbind();
        Ok(())
    }

    pub fn at(&self, ix: usize) -> BufferSlice<T, K> {
        self.slice(ix..=ix)
    }

    pub fn slice(&self, range: impl RangeBounds<usize>) -> BufferSlice<T, K> {
        let (alignment, range) = self.byte_slice(std::mem::size_of::<T>(), range);
        let offset = range.start as _;
        let size = (range.end - range.start) as _;
        BufferSlice {
            buffer: self,
            alignment,
            offset,
            size,
        }
    }

    fn byte_slice(&self, sizeof: usize, range: impl RangeBounds<usize>) -> (usize, Range<usize>) {
        tracing::trace!(range.start = ?range.start_bound(), range.end = ?range.end_bound());
        let alignment = next_multiple(sizeof, *GL_ALIGNMENT);
        let start = match range.start_bound() {
            Bound::Included(i) => *i,
            Bound::Excluded(i) => i + 1,
            Bound::Unbounded => 0,
        } * alignment;
        let end = match range.end_bound() {
            Bound::Included(i) => i + 1,
            Bound::Excluded(i) => *i,
            Bound::Unbounded => self.count * std::mem::size_of::<T>(),
        } * alignment;
        (alignment, start..end)
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

pub struct BufferSlice<'buf, T, const K: u32> {
    pub(crate) buffer: &'buf Buffer<T, K>,
    pub(crate) offset: GLintptr,
    pub(crate) size: GLsizeiptr,
    pub alignment: usize,
}

impl<'buf, T: bytemuck::Pod, const K: u32> BufferSlice<'buf, T, K> {
    pub fn get_all(&self, access: BufferAccess) -> Result<MappedBufferData<T, K>> {
        gl_error_guard(|| {
            self.buffer.with_binding(|| {
                let bytes = unsafe {
                    let access = access & !BufferAccess::MAP_WRITE;
                    let ptr = gl::MapBufferRange(K, self.offset, self.size, access.bits);
                    std::slice::from_raw_parts(ptr as *const u8, self.size as _)
                };
                tracing::debug!(
                    "Map buffer {} ({}..{})",
                    self.buffer.id,
                    self.offset,
                    self.offset + self.size
                );
                MappedBufferData {
                    __ty: PhantomData,
                    id: self.buffer.id,
                    data: bytemuck::cast_slice(bytes),
                }
            })
        })
    }

    pub fn set(&mut self, at: usize, value: &T) -> Result<()> {
        let offset = self.offset + (at * self.alignment) as GLintptr;
        let bytes = bytemuck::bytes_of(value);
        self.buffer.with_binding(|| gl_error_guard(|| unsafe {
            let access = BufferAccess::MAP_WRITE;
            let ptr = gl::MapBufferRange(K, offset, self.size, access.bits);
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr as *mut u8, self.alignment as _);
            gl::UnmapBuffer(K);
        }))
    }

    pub fn set_all(&mut self, data: &[T], access: BufferAccess) -> Result<()> {
        eyre::ensure!(
            data.len() * self.alignment == self.size as _,
            "Slice length need to equal mapped slice length"
        );
        let bytes = bytemuck::cast_slice(data);
        self.buffer.with_binding(|| {
            gl_error_guard(|| unsafe {
                let access = access | BufferAccess::MAP_READ | BufferAccess::MAP_WRITE;
                let ptr = gl::MapBufferRange(K, self.offset, self.size, access.bits);
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr as *mut u8, self.size as _);
                gl::UnmapBuffer(K);
            })
        })
    }
}

#[derive(Debug)]
/// Mapped buffer data from OpenGL.
pub struct MappedBufferData<'data, 'buf, T, const K: u32> {
    __ty: PhantomData<&'buf ()>,
    id: BufferId<K>,
    data: &'data [T],
}

impl<'m, 'b, T, const K: u32> std::ops::Deref for MappedBufferData<'m, 'b, T, K> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<'m, 'b, T, const K: u32> Drop for MappedBufferData<'m, 'b, T, K> {
    fn drop(&mut self) {
        tracing::debug!("Unmap buffer {}", self.id);
        unsafe {
            gl::UnmapBuffer(K);
            gl::BindBuffer(K, 0);
        }
    }
}

#[cfg(not(feature = "fast"))]
static GL_ALIGNMENT: Lazy<NonZeroUsize> = Lazy::new(|| {
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
});

#[cfg(feature = "fast")]
static GL_ALIGNMENT: Lazy<NonZeroUsize> = Lazy::new(|| unsafe {
    let mut val = 0;
    gl::GetIntegerv(gl::UNIFORM_BUFFER_OFFSET_ALIGNMENT, &mut val);
    tracing::trace!(
        "glGetIntegerv(GL_UNIFORM_BUFFER_OFFSET_ALIGNMENT, <inout val={}>)",
        val
    );
    NonZeroUsize::new_unchecked(val as _)
});

#[inline(always)]
fn next_multiple(x: usize, of: NonZeroUsize) -> usize {
    let rem = x % of.get();
    let offset = of.get() - rem;
    x + offset
}
