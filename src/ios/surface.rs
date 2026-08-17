//! This module wraps iOS IOSurface-backed graphics surfaces.

use std::ffi::c_void;
use std::fmt::{self, Debug, Formatter};
use std::ptr;

use euclid::default::Size2D;
use glow::{NativeFramebuffer, NativeTexture};

use super::{
    sm_ios_io_surface_release, sm_ios_io_surface_retain, SmIosSurface, SmIosSurfaceTexture,
};
use crate::context::ContextID;
use crate::SurfaceID;

/// This value owns an iOS graphics surface.
pub struct Surface {
    pub(crate) native: *mut SmIosSurface,
    pub(crate) context_id: ContextID,
    pub(crate) size: Size2D<i32>,
    pub(crate) framebuffer: NativeFramebuffer,
    pub(crate) texture: NativeTexture,
}

/// This value owns a texture view and its source surface.
pub struct SurfaceTexture {
    pub(crate) native: *mut SmIosSurfaceTexture,
    pub(crate) surface: Surface,
    pub(crate) texture: NativeTexture,
}

/// This value retains an IOSurface reference.
pub struct NativeSurface(*mut c_void);

/// This value represents an unsupported iOS widget surface.
#[derive(Clone)]
pub struct NativeWidget;

unsafe impl Send for Surface {}
unsafe impl Send for NativeSurface {}
unsafe impl Sync for NativeSurface {}

impl Surface {
    pub(crate) fn id(&self) -> SurfaceID {
        SurfaceID(self.native as usize)
    }
}

impl NativeSurface {
    /// This function returns the retained IOSurface pointer.
    pub fn as_ptr(&self) -> *mut c_void {
        self.0
    }

    pub(crate) fn from_retained(pointer: *mut c_void) -> Self {
        Self(pointer)
    }
}

impl Clone for NativeSurface {
    fn clone(&self) -> Self {
        unsafe { sm_ios_io_surface_retain(self.0) };
        Self(self.0)
    }
}

impl Drop for NativeSurface {
    fn drop(&mut self) {
        unsafe { sm_ios_io_surface_release(self.0) };
        self.0 = ptr::null_mut();
    }
}

impl Debug for Surface {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "Surface({:x})", self.id().0)
    }
}

impl Debug for SurfaceTexture {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "SurfaceTexture({:?})", self.surface)
    }
}
