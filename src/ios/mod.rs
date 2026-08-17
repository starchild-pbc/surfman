//! This module provides the iOS EAGL backend.

use std::ffi::{c_char, c_void};

pub mod connection;
pub mod context;
pub mod device;
pub mod surface;

#[repr(C)]
pub(crate) struct SmIosContext {
    _private: [u8; 0],
}

#[repr(C)]
pub(crate) struct SmIosSurface {
    _private: [u8; 0],
}

#[repr(C)]
pub(crate) struct SmIosSurfaceTexture {
    _private: [u8; 0],
}

unsafe extern "C" {
    pub(crate) fn sm_ios_context_create(shared: *mut SmIosContext) -> *mut SmIosContext;
    pub(crate) fn sm_ios_context_retain(context: *mut SmIosContext) -> *mut SmIosContext;
    pub(crate) fn sm_ios_context_release(context: *mut SmIosContext);
    pub(crate) fn sm_ios_context_make_current(context: *mut SmIosContext) -> bool;
    pub(crate) fn sm_ios_context_clear_current() -> bool;
    pub(crate) fn sm_ios_gl_proc_address(name: *const c_char) -> *const c_void;

    pub(crate) fn sm_ios_surface_create(
        context: *mut SmIosContext,
        width: i32,
        height: i32,
    ) -> *mut SmIosSurface;
    pub(crate) fn sm_ios_surface_destroy(context: *mut SmIosContext, surface: *mut SmIosSurface);
    pub(crate) fn sm_ios_surface_resize(
        context: *mut SmIosContext,
        surface: *mut SmIosSurface,
        width: i32,
        height: i32,
    ) -> bool;
    pub(crate) fn sm_ios_surface_bind(surface: *mut SmIosSurface);
    pub(crate) fn sm_ios_surface_finish();
    pub(crate) fn sm_ios_surface_framebuffer(surface: *mut SmIosSurface) -> u32;
    pub(crate) fn sm_ios_surface_texture(surface: *mut SmIosSurface) -> u32;
    pub(crate) fn sm_ios_surface_copy_io_surface(surface: *mut SmIosSurface) -> *mut c_void;
    pub(crate) fn sm_ios_io_surface_retain(surface: *mut c_void);
    pub(crate) fn sm_ios_io_surface_release(surface: *mut c_void);

    pub(crate) fn sm_ios_surface_texture_create(
        context: *mut SmIosContext,
        surface: *mut SmIosSurface,
    ) -> *mut SmIosSurfaceTexture;
    pub(crate) fn sm_ios_surface_texture_destroy(texture: *mut SmIosSurfaceTexture);
    pub(crate) fn sm_ios_surface_texture_name(texture: *mut SmIosSurfaceTexture) -> u32;
}

crate::implement_interfaces!();
