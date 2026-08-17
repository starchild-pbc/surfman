//! This module implements the iOS EAGL graphics device.

use std::ffi::{c_void, CString};
use std::mem;
use std::num::NonZeroU32;
use std::ptr;

use euclid::default::Size2D;
use glow::{NativeFramebuffer, NativeTexture};

use super::connection::Connection;
use super::context::{Context, ContextDescriptor, NativeContext};
use super::surface::{NativeSurface, NativeWidget, Surface, SurfaceTexture};
use super::{
    sm_ios_context_clear_current, sm_ios_context_create, sm_ios_context_make_current,
    sm_ios_context_release, sm_ios_context_retain, sm_ios_gl_proc_address, sm_ios_surface_bind,
    sm_ios_surface_copy_io_surface, sm_ios_surface_create, sm_ios_surface_destroy,
    sm_ios_surface_finish, sm_ios_surface_framebuffer, sm_ios_surface_resize,
    sm_ios_surface_texture, sm_ios_surface_texture_create, sm_ios_surface_texture_destroy,
    sm_ios_surface_texture_name,
};
use crate::context::{ContextID, CREATE_CONTEXT_MUTEX};
use crate::{
    gl, ContextAttributeFlags, ContextAttributes, Error, GLApi, GLVersion, Gl, SurfaceAccess,
    SurfaceInfo, SurfaceType,
};

/// This value represents the available EAGL adapter.
#[derive(Clone, Debug)]
pub struct Adapter;

/// This value represents a thread-local EAGL device.
pub struct Device;

/// This value represents the native EAGL device.
#[derive(Clone)]
pub struct NativeDevice;

impl Device {
    pub(crate) fn new(_: &Connection, _: &Adapter) -> Result<Self, Error> {
        Ok(Self)
    }

    /// This function returns the native device value.
    pub fn native_device(&self) -> NativeDevice {
        NativeDevice
    }

    /// This function returns the iOS graphics connection.
    pub fn connection(&self) -> Connection {
        Connection
    }

    /// This function returns the EAGL adapter.
    pub fn adapter(&self) -> Adapter {
        Adapter
    }

    /// This function returns the OpenGL ES API type.
    pub fn gl_api(&self) -> GLApi {
        GLApi::GLES
    }

    /// This function creates an EAGL context descriptor.
    pub fn create_context_descriptor(
        &self,
        attributes: &ContextAttributes,
    ) -> Result<ContextDescriptor, Error> {
        if attributes.version.major > 3 || attributes.version.major < 2 {
            return Err(Error::UnsupportedGLVersion);
        }
        if attributes
            .flags
            .contains(ContextAttributeFlags::COMPATIBILITY_PROFILE)
        {
            return Err(Error::UnsupportedGLProfile);
        }
        Ok(ContextDescriptor(*attributes))
    }

    /// This function creates and selects an EAGL context.
    pub fn create_context(
        &self,
        descriptor: &ContextDescriptor,
        share_with: Option<&Context>,
    ) -> Result<Context, Error> {
        let mut next_id = CREATE_CONTEXT_MUTEX.lock().unwrap();
        next_id.0 += 1;
        let id = *next_id;
        let native = unsafe {
            sm_ios_context_create(
                share_with
                    .map(|context| context.native)
                    .unwrap_or(ptr::null_mut()),
            )
        };
        if native.is_null() {
            return Err(Error::ContextCreationFailed(
                crate::WindowingApiError::Failed,
            ));
        }
        let gl = unsafe { Gl::from_loader_function(get_proc_address) };
        Ok(Context {
            native,
            id,
            surface: None,
            gl: std::rc::Rc::new(gl),
            descriptor: descriptor.clone(),
        })
    }

    /// This function wraps a retained native EAGL context.
    pub unsafe fn create_context_from_native_context(
        &self,
        native_context: NativeContext,
    ) -> Result<Context, Error> {
        if native_context.0.is_null() {
            return Err(Error::IncompatibleNativeContext);
        }
        let mut next_id = CREATE_CONTEXT_MUTEX.lock().unwrap();
        next_id.0 += 1;
        let id = *next_id;
        let native = native_context.0;
        mem::forget(native_context);
        let descriptor = ContextDescriptor(ContextAttributes {
            flags: ContextAttributeFlags::ALPHA
                | ContextAttributeFlags::DEPTH
                | ContextAttributeFlags::STENCIL,
            version: GLVersion::new(3, 0),
        });
        let gl = Gl::from_loader_function(get_proc_address);
        Ok(Context {
            native,
            id,
            surface: None,
            gl: std::rc::Rc::new(gl),
            descriptor,
        })
    }

    /// This function destroys an EAGL context.
    pub fn destroy_context(&self, context: &mut Context) -> Result<(), Error> {
        if let Some(mut surface) = self.unbind_surface_from_context(context)? {
            self.destroy_surface(context, &mut surface)?;
        }
        unsafe { sm_ios_context_release(context.native) };
        context.native = ptr::null_mut();
        Ok(())
    }

    /// This function returns a retained native EAGL context.
    pub fn native_context(&self, context: &Context) -> NativeContext {
        NativeContext(unsafe { sm_ios_context_retain(context.native) })
    }

    /// This function returns the context descriptor.
    pub fn context_descriptor(&self, context: &Context) -> ContextDescriptor {
        context.descriptor.clone()
    }

    /// This function selects the EAGL context for the current thread.
    pub fn make_context_current(&self, context: &Context) -> Result<(), Error> {
        if unsafe { sm_ios_context_make_current(context.native) } {
            Ok(())
        } else {
            Err(Error::MakeCurrentFailed(crate::WindowingApiError::Failed))
        }
    }

    /// This function clears the EAGL context for the current thread.
    pub fn make_no_context_current(&self) -> Result<(), Error> {
        if unsafe { sm_ios_context_clear_current() } {
            Ok(())
        } else {
            Err(Error::MakeCurrentFailed(crate::WindowingApiError::Failed))
        }
    }

    /// This function returns the requested context attributes.
    pub fn context_descriptor_attributes(
        &self,
        descriptor: &ContextDescriptor,
    ) -> ContextAttributes {
        descriptor.0
    }

    /// This function returns the address of an OpenGL ES function.
    pub fn get_proc_address(&self, _: &Context, name: &str) -> *const c_void {
        get_proc_address(name)
    }

    /// This function attaches a surface to an EAGL context.
    pub fn bind_surface_to_context(
        &self,
        context: &mut Context,
        surface: Surface,
    ) -> Result<(), (Error, Surface)> {
        if context.surface.is_some() {
            return Err((Error::SurfaceAlreadyBound, surface));
        }
        if surface.context_id != context.id {
            return Err((Error::IncompatibleSurface, surface));
        }
        if let Err(error) = self.make_context_current(context) {
            return Err((error, surface));
        }
        unsafe { sm_ios_surface_bind(surface.native) };
        context.surface = Some(surface);
        Ok(())
    }

    /// This function detaches the current surface from an EAGL context.
    pub fn unbind_surface_from_context(
        &self,
        context: &mut Context,
    ) -> Result<Option<Surface>, Error> {
        if context.surface.is_none() {
            return Ok(None);
        }
        self.make_context_current(context)?;
        unsafe { sm_ios_surface_finish() };
        Ok(context.surface.take())
    }

    /// This function returns a unique context identifier.
    pub fn context_id(&self, context: &Context) -> ContextID {
        context.id
    }

    /// This function returns information about the attached surface.
    pub fn context_surface_info(&self, context: &Context) -> Result<Option<SurfaceInfo>, Error> {
        Ok(context
            .surface
            .as_ref()
            .map(|surface| self.surface_info(surface)))
    }

    /// This function creates an IOSurface-backed generic surface.
    pub fn create_surface(
        &self,
        context: &Context,
        _: SurfaceAccess,
        surface_type: SurfaceType<NativeWidget>,
    ) -> Result<Surface, Error> {
        let size = match surface_type {
            SurfaceType::Generic { size } => size,
            SurfaceType::Widget { .. } => return Err(Error::UnsupportedOnThisPlatform),
        };
        self.make_context_current(context)?;
        let native = unsafe { sm_ios_surface_create(context.native, size.width, size.height) };
        if native.is_null() {
            return Err(Error::SurfaceCreationFailed(
                crate::WindowingApiError::Failed,
            ));
        }
        let framebuffer_name = unsafe { sm_ios_surface_framebuffer(native) };
        let texture_name = unsafe { sm_ios_surface_texture(native) };
        let Some(framebuffer_name) = NonZeroU32::new(framebuffer_name) else {
            unsafe { sm_ios_surface_destroy(context.native, native) };
            return Err(Error::SurfaceCreationFailed(
                crate::WindowingApiError::Failed,
            ));
        };
        let Some(texture_name) = NonZeroU32::new(texture_name) else {
            unsafe { sm_ios_surface_destroy(context.native, native) };
            return Err(Error::SurfaceCreationFailed(
                crate::WindowingApiError::Failed,
            ));
        };
        Ok(Surface {
            native,
            context_id: context.id,
            size,
            framebuffer: NativeFramebuffer(framebuffer_name),
            texture: NativeTexture(texture_name),
        })
    }

    /// This function creates a texture view for a generic surface.
    pub fn create_surface_texture(
        &self,
        context: &mut Context,
        surface: Surface,
    ) -> Result<SurfaceTexture, (Error, Surface)> {
        if let Err(error) = self.make_context_current(context) {
            return Err((error, surface));
        }
        let native = unsafe { sm_ios_surface_texture_create(context.native, surface.native) };
        if native.is_null() {
            return Err((
                Error::SurfaceTextureCreationFailed(crate::WindowingApiError::Failed),
                surface,
            ));
        }
        let texture_name = unsafe { sm_ios_surface_texture_name(native) };
        let Some(texture_name) = NonZeroU32::new(texture_name) else {
            unsafe { sm_ios_surface_texture_destroy(native) };
            return Err((
                Error::SurfaceTextureCreationFailed(crate::WindowingApiError::Failed),
                surface,
            ));
        };
        Ok(SurfaceTexture {
            native,
            surface,
            texture: NativeTexture(texture_name),
        })
    }

    /// This function destroys a generic surface.
    pub fn destroy_surface(
        &self,
        context: &mut Context,
        surface: &mut Surface,
    ) -> Result<(), Error> {
        if surface.context_id != context.id {
            return Err(Error::IncompatibleSurface);
        }
        if surface.native.is_null() {
            return Ok(());
        }
        unsafe { sm_ios_surface_destroy(context.native, surface.native) };
        surface.native = ptr::null_mut();
        Ok(())
    }

    /// This function destroys a texture view and returns its source surface.
    pub fn destroy_surface_texture(
        &self,
        context: &mut Context,
        mut surface_texture: SurfaceTexture,
    ) -> Result<Surface, (Error, SurfaceTexture)> {
        if let Err(error) = self.make_context_current(context) {
            return Err((error, surface_texture));
        }
        unsafe { sm_ios_surface_texture_destroy(surface_texture.native) };
        surface_texture.native = ptr::null_mut();
        let surface = unsafe { ptr::read(&surface_texture.surface) };
        mem::forget(surface_texture);
        Ok(surface)
    }

    /// This function returns the OpenGL ES texture target.
    pub fn surface_gl_texture_target(&self) -> u32 {
        gl::TEXTURE_2D
    }

    /// This function completes work on the attached surface.
    pub fn present_bound_surface(&self, context: &mut Context) -> Result<(), Error> {
        self.make_context_current(context)?;
        unsafe { sm_ios_surface_finish() };
        Ok(())
    }

    /// This function completes work on a surface.
    pub fn present_surface(&self, context: &Context, surface: &mut Surface) -> Result<(), Error> {
        if surface.context_id != context.id {
            return Err(Error::IncompatibleSurface);
        }
        self.make_context_current(context)?;
        unsafe { sm_ios_surface_finish() };
        Ok(())
    }

    /// This function resizes the attached generic surface.
    pub fn resize_bound_surface(
        &self,
        context: &mut Context,
        size: Size2D<i32>,
    ) -> Result<(), Error> {
        let Some(mut surface) = context.surface.take() else {
            return Ok(());
        };
        let result = self.resize_surface(context, &mut surface, size);
        if result.is_ok() {
            unsafe { sm_ios_surface_bind(surface.native) };
        }
        context.surface = Some(surface);
        result
    }

    /// This function resizes a generic surface.
    pub fn resize_surface(
        &self,
        context: &Context,
        surface: &mut Surface,
        size: Size2D<i32>,
    ) -> Result<(), Error> {
        if surface.context_id != context.id {
            return Err(Error::IncompatibleSurface);
        }
        if !unsafe {
            sm_ios_surface_resize(context.native, surface.native, size.width, size.height)
        } {
            return Err(Error::SurfaceCreationFailed(
                crate::WindowingApiError::Failed,
            ));
        }
        let framebuffer = unsafe { sm_ios_surface_framebuffer(surface.native) };
        let texture = unsafe { sm_ios_surface_texture(surface.native) };
        surface.framebuffer = NativeFramebuffer(NonZeroU32::new(framebuffer).ok_or(
            Error::SurfaceCreationFailed(crate::WindowingApiError::Failed),
        )?);
        surface.texture = NativeTexture(NonZeroU32::new(texture).ok_or(
            Error::SurfaceCreationFailed(crate::WindowingApiError::Failed),
        )?);
        surface.size = size;
        Ok(())
    }

    /// This function returns information about a surface.
    pub fn surface_info(&self, surface: &Surface) -> SurfaceInfo {
        SurfaceInfo {
            size: surface.size,
            id: surface.id(),
            context_id: surface.context_id,
            framebuffer_object: Some(surface.framebuffer),
        }
    }

    /// This function returns the texture object for a surface texture.
    pub fn surface_texture_object(
        &self,
        surface_texture: &SurfaceTexture,
    ) -> Option<NativeTexture> {
        Some(surface_texture.texture)
    }

    /// This function returns a retained IOSurface for the surface.
    pub fn native_surface(&self, surface: &Surface) -> NativeSurface {
        NativeSurface::from_retained(unsafe { sm_ios_surface_copy_io_surface(surface.native) })
    }
}

fn get_proc_address(name: &str) -> *const c_void {
    CString::new(name)
        .ok()
        .map(|name| unsafe { sm_ios_gl_proc_address(name.as_ptr()) })
        .unwrap_or(ptr::null())
}
