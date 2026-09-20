//! This module represents the iOS graphics connection.

use std::ffi::c_void;

use euclid::default::Size2D;

use super::device::{Adapter, Device, NativeDevice};
use super::surface::NativeWidget;
use crate::{Error, GLApi};

/// This value represents the process-wide iOS graphics connection.
#[derive(Clone)]
pub struct Connection;

/// This value represents the native iOS graphics connection.
#[derive(Clone)]
pub struct NativeConnection;

impl Connection {
    /// This function opens the process-wide iOS graphics connection.
    pub fn new() -> Result<Self, Error> {
        Ok(Self)
    }

    /// This function wraps the native iOS graphics connection.
    pub unsafe fn from_native_connection(_: NativeConnection) -> Result<Self, Error> {
        Ok(Self)
    }

    /// This function returns the native iOS graphics connection.
    pub fn native_connection(&self) -> NativeConnection {
        NativeConnection
    }

    /// This function returns the OpenGL ES API type.
    pub fn gl_api(&self) -> GLApi {
        GLApi::GLES
    }

    /// This function returns the default adapter.
    pub fn create_adapter(&self) -> Result<Adapter, Error> {
        self.create_hardware_adapter()
    }

    /// This function returns the hardware adapter.
    pub fn create_hardware_adapter(&self) -> Result<Adapter, Error> {
        Ok(Adapter)
    }

    /// This function returns the low-power adapter.
    pub fn create_low_power_adapter(&self) -> Result<Adapter, Error> {
        Ok(Adapter)
    }

    /// This function returns the available adapter for software requests.
    pub fn create_software_adapter(&self) -> Result<Adapter, Error> {
        Ok(Adapter)
    }

    /// This function opens a device for the adapter.
    pub fn create_device(&self, adapter: &Adapter) -> Result<Device, Error> {
        Device::new(self, adapter)
    }

    /// This function opens a device from a native device value.
    pub unsafe fn create_device_from_native_device(
        &self,
        _: NativeDevice,
    ) -> Result<Device, Error> {
        Device::new(self, &Adapter)
    }

    /// This function rejects raw display handles on iOS.
    #[cfg(feature = "sm-raw-window-handle-05")]
    pub fn from_raw_display_handle(_: rwh_05::RawDisplayHandle) -> Result<Self, Error> {
        Err(Error::IncompatibleRawDisplayHandle)
    }

    /// This function rejects display handles on iOS.
    #[cfg(feature = "sm-raw-window-handle-06")]
    pub fn from_display_handle(_: rwh_06::DisplayHandle) -> Result<Self, Error> {
        Err(Error::IncompatibleRawDisplayHandle)
    }

    /// This function creates the placeholder native widget value.
    pub unsafe fn create_native_widget_from_ptr(
        &self,
        _: *mut c_void,
        _: Size2D<i32>,
    ) -> NativeWidget {
        NativeWidget
    }

    /// This function rejects raw window handles on iOS.
    #[cfg(feature = "sm-raw-window-handle-05")]
    pub fn create_native_widget_from_raw_window_handle(
        &self,
        _: rwh_05::RawWindowHandle,
        _: Size2D<i32>,
    ) -> Result<NativeWidget, Error> {
        Err(Error::IncompatibleNativeWidget)
    }

    /// This function rejects window handles on iOS.
    #[cfg(feature = "sm-raw-window-handle-06")]
    pub fn create_native_widget_from_window_handle(
        &self,
        _: rwh_06::WindowHandle,
        _: Size2D<i32>,
    ) -> Result<NativeWidget, Error> {
        Err(Error::IncompatibleNativeWidget)
    }
}
