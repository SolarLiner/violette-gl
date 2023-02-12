use std::fmt::{Display};

/// Trait of types allowed to be bound. The binding is a separate type who has the responsibility of
/// unbinding the resource.
pub trait Resource<'a> {
    /// Type of the identifier on this resource
    type Id: Copy + Eq + Display;

    fn id(&self) -> Self::Id;
    /// Currently bound resource ID
    fn current() -> Option<Self::Id>;
    /// Bind the resource to bring it into focus within the OpenGL driver's state machine
    fn bind(&self);
    /// Unbind the resource, restoring back the OpenGL driver's state machine.
    fn unbind(&self);
}

/// Extension method on resources to help with managing bindings
pub trait ResourceExt<'a>: Resource<'a> {
    fn with_binding<T, F: FnOnce() -> T>(
        &'a self,
        cb: F,
    ) -> T {
        self.bind();
        
        #[cfg(not(feature = "no-unbind"))]
        self.unbind();
        cb()
    }
}

impl<'a, B: Resource<'a>> ResourceExt<'a> for B {}
