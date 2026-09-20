use euclid::default::Size2D;
use glow::{HasContext, PixelPackData};
use surfman::{
    Connection, Context, ContextAttributeFlags, ContextAttributes, Device, GLVersion,
    SurfaceAccess, SurfaceType,
};

const EXPECTED_PIXEL: [u8; 4] = [51, 102, 204, 255];

#[no_mangle]
pub extern "C" fn surfman_ios_device_test_run() -> i32 {
    run_test()
}

fn run_test() -> i32 {
    let connection = match Connection::new() {
        Ok(connection) => connection,
        Err(_) => return 1,
    };
    let adapter = match connection.create_hardware_adapter() {
        Ok(adapter) => adapter,
        Err(_) => return 2,
    };
    let device = match connection.create_device(&adapter) {
        Ok(device) => device,
        Err(_) => return 3,
    };
    let descriptor = match device.create_context_descriptor(&ContextAttributes {
        version: GLVersion::new(3, 0),
        flags: ContextAttributeFlags::empty(),
    }) {
        Ok(descriptor) => descriptor,
        Err(_) => return 4,
    };
    let mut context = match device.create_context(&descriptor, None) {
        Ok(context) => context,
        Err(_) => return 5,
    };
    let surface = match device.create_surface(
        &context,
        SurfaceAccess::GPUCPU,
        SurfaceType::Generic {
            size: Size2D::new(4, 4),
        },
    ) {
        Ok(surface) => surface,
        Err(_) => return finish_without_surface(&device, context, 6),
    };

    let native_surface = device.native_surface(&surface);
    if native_surface.as_ptr().is_null() {
        let mut surface = surface;
        let _ = device.destroy_surface(&mut context, &mut surface);
        return finish_without_surface(&device, context, 7);
    }
    drop(native_surface);

    if let Err((_, mut surface)) = device.bind_surface_to_context(&mut context, surface) {
        let _ = device.destroy_surface(&mut context, &mut surface);
        return finish_without_surface(&device, context, 8);
    }

    let gl = unsafe {
        glow::Context::from_loader_function(|name| device.get_proc_address(&context, name))
    };
    let mut actual_pixel = [0; 4];
    unsafe {
        gl.viewport(0, 0, 4, 4);
        gl.clear_color(
            EXPECTED_PIXEL[0] as f32 / 255.0,
            EXPECTED_PIXEL[1] as f32 / 255.0,
            EXPECTED_PIXEL[2] as f32 / 255.0,
            EXPECTED_PIXEL[3] as f32 / 255.0,
        );
        gl.clear(glow::COLOR_BUFFER_BIT);
        gl.finish();
        gl.read_pixels(
            0,
            0,
            1,
            1,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            PixelPackData::Slice(Some(&mut actual_pixel)),
        );
    }
    let gl_error = unsafe { gl.get_error() };

    let cleanup_ok = finish_with_surface(&device, context);
    if !cleanup_ok {
        return 9;
    }
    if gl_error != glow::NO_ERROR {
        return 10;
    }
    if actual_pixel != EXPECTED_PIXEL {
        return 11;
    }
    0
}

fn finish_with_surface(device: &Device, mut context: Context) -> bool {
    let mut success = true;
    match device.unbind_surface_from_context(&mut context) {
        Ok(Some(mut surface)) => {
            success &= device.destroy_surface(&mut context, &mut surface).is_ok();
        }
        Ok(None) => success = false,
        Err(_) => success = false,
    }
    success &= device.make_no_context_current().is_ok();
    finish_context(device, context) && success
}

fn finish_without_surface(device: &Device, context: Context, error_code: i32) -> i32 {
    let cleared_current_context = device.make_no_context_current().is_ok();
    if finish_context(device, context) && cleared_current_context {
        error_code
    } else {
        9
    }
}

fn finish_context(device: &Device, mut context: Context) -> bool {
    if device.destroy_context(&mut context).is_err() {
        std::mem::forget(context);
        return false;
    }
    true
}
