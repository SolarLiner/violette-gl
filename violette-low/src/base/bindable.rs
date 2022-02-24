use std::ops::{Deref, DerefMut};

use tracing::span::EnteredSpan;

/// Trait of types representing a bound value.
pub trait Binding<'a>: Deref<Target = Self::Parent> {
    type Parent: Resource<'a>;
    /// Unbind the resource
    fn unbind(&mut self, previous: Option<<<Self as Binding<'a>>::Parent as Resource<'a>>::Id>);
}

/// Trait of types allowed to be bound. The binding is a separate type who has the responsibility of
/// unbinding the resource.
pub trait Resource<'a> {
    /// Type of the identifier on this resource
    type Id: Copy + Eq;
    /// Resource kind. This is mainly for buffers, as they have multiple kinds
    type Kind;
    /// Bound resource type.
    type Bound: 'a + Binding<'a, Parent = Self>;

    /// Currently bound resource ID
    fn current(kind: Self::Kind) -> Option<Self::Id>;
    /// Resource kind
    fn kind(&self) -> Self::Kind;
    /// Bind the resource
    fn make_binding(&'a mut self) -> anyhow::Result<Self::Bound>;
}

/// A `Bindable` guard that unbinds the resource on `Drop`.
pub struct BindGuard<'a, B: Binding<'a>> {
    _span: EnteredSpan,
    previous: Option<<B::Parent as Resource<'a>>::Id>,
    binding: B,
}

impl<'a, B: Binding<'a>> Drop for BindGuard<'a, B> {
    fn drop(&mut self) {
        if !cfg!(feature = "no-unbind") {
            self.binding.unbind(self.previous);
        }
    }
}

impl<'a, B: Binding<'a>> Deref for BindGuard<'a, B> {
    type Target = B;

    fn deref(&self) -> &Self::Target {
        &self.binding
    }
}

impl<'a, B: Binding<'a>> DerefMut for BindGuard<'a, B> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.binding
    }
}

/// Extension method on resources to help with managing bindings
pub trait BindableExt<'a>: Resource<'a> {
    fn bind(&'a mut self) -> anyhow::Result<BindGuard<'a, Self::Bound>> {
        let _span = tracing::trace_span!("bind_guard", ty=%std::any::type_name::<Self>()).entered();
        Ok(BindGuard {
            _span,
            previous: Self::current(self.kind()),
            binding: self.make_binding()?,
        })
    }

    fn with_binding<T, F: FnOnce(&mut Self::Bound) -> anyhow::Result<T>>(
        &'a mut self,
        cb: F,
    ) -> anyhow::Result<T> {
        let mut guard = self.bind()?;
        cb(&mut guard.binding)
    }
}

impl<'a, B: Resource<'a>> BindableExt<'a> for B {}
