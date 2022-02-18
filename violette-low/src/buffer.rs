use std::{
    borrow::BorrowMut,
    cell::RefCell,
    iter::Map,
    marker::PhantomData,
    num::NonZeroU32,
    ops::{Bound, RangeBounds},
};

use anyhow::Context;
use bitflags::bitflags;
use bytemuck::{cast_slice, Pod};
use gl::types::{GLbitfield, GLuint};
use once_cell::sync::OnceCell;

use crate::utils::gl_error_guard;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum BufferKind {
    Array = gl::ARRAY_BUFFER,
    ElementArray = gl::ELEMENT_ARRAY_BUFFER,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum BufferUsageHint {
    Stream = gl::STREAM_DRAW,
    Static = gl::STATIC_DRAW,
    Dynamic = gl::DYNAMIC_DRAW,
}

pub struct Buffer<T> {
    __type: PhantomData<T>,
    pub id: BufferId,
    size: usize,
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
            size: 0,
        }
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn kind(&self) -> BufferKind {
        self.id.kind
    }
}

bitflags! {
    pub struct BufferAccess: GLbitfield {
        const PERSISTENT = gl::MAP_PERSISTENT_BIT;
        const COHERENT = gl::MAP_COHERENT_BIT;
    }
}

pub struct BoundBuffer<'a, T> {
    buffer: &'a mut Buffer<T>,
    previous: Option<BufferId>,
}

impl<'a, T> std::ops::Deref for BoundBuffer<'a, T> {
    type Target = &'a mut Buffer<T>;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl<'a, T> From<&'a mut Buffer<T>> for BoundBuffer<'a, T> {
    fn from(buffer: &'a mut Buffer<T>) -> Self {
        Self {
            previous: ,
            buffer,
        }
    }
}

impl<'a, T> Drop for BoundBuffer<'a, T> {
    fn drop(&mut self) {
        buffer_stack_pop();
        if let Some(previous) = self.previous {
            unsafe { gl::BindBuffer(previous.kind as _, previous.get()) };
        }
    }
}

impl<'a, T: Pod> BoundBuffer<'a, T> {
    pub fn set(&self, data: &[T], usage_hint: BufferUsageHint) -> anyhow::Result<()> {
        let bytes = bytemuck::cast_slice::<_, u8>(data);
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
            Bound::Unbounded => self.size,
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

pub fn current_buffer(kind: BufferKind) -> Option<BufferId> {
    let id = unsafe {
        let mut res = 0;
        gl::GetIntegerv(kind as _, &mut res);
        res
    };
    BufferId::new(id as _, kind)
}
