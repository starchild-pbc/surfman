//! This module wraps EAGL rendering contexts.

use std::ptr;
use std::rc::Rc;
use std::thread;

use super::surface::Surface;
use super::{sm_ios_context_release, sm_ios_context_retain, SmIosContext};
use crate::context::ContextID;
use crate::{ContextAttributes, Error, Gl};

/// This value owns an EAGL rendering context.
pub struct Context {
    pub(crate) native: *mut SmIosContext,
    pub(crate) id: ContextID,
    pub(crate) surface: Option<Surface>,
    pub(crate) gl: Rc<Gl>,
    pub(crate) descriptor: ContextDescriptor,
}

/// This value retains a native EAGL rendering context.
pub struct NativeContext(pub(crate) *mut SmIosContext);

/// This value stores the requested OpenGL ES attributes.
#[derive(Clone)]
pub struct ContextDescriptor(pub(crate) ContextAttributes);

unsafe impl Send for ContextDescriptor {}

impl Drop for Context {
    fn drop(&mut self) {
        if !self.native.is_null() && !thread::panicking() {
            panic!("Contexts must be destroyed explicitly with destroy_context.");
        }
    }
}

impl Clone for NativeContext {
    fn clone(&self) -> Self {
        Self(unsafe { sm_ios_context_retain(self.0) })
    }
}

impl Drop for NativeContext {
    fn drop(&mut self) {
        unsafe { sm_ios_context_release(self.0) };
        self.0 = ptr::null_mut();
    }
}

impl NativeContext {
    /// This function reports that the current native context is not available.
    pub fn current() -> Result<Self, Error> {
        Err(Error::NoCurrentContext)
    }
}
