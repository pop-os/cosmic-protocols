use std::sync::Once;

use crate::egl::{get_proc_address, EGLImage};

mod ffi {
    include!(concat!(env!("OUT_DIR"), "/gl_bindings.rs"));
}

fn load() {
    static LOADED: Once = Once::new();
    LOADED.call_once(|| {
        ffi::load_with(get_proc_address);
    });
}

unsafe fn clear_error_flags() {
    while ffi::GetError() != ffi::NO_ERROR {}
}

pub unsafe fn bind_eglimage_to_texture(
    image: &EGLImage,
) -> Result<ffi::types::GLuint, ffi::types::GLenum> {
    load();
    clear_error_flags();

    let mut texture = 0;
    ffi::GenTextures(1, &mut texture);
    ffi::BindTexture(ffi::TEXTURE_2D, texture);
    ffi::EGLImageTargetTexture2DOES(ffi::TEXTURE_2D, image.ptr());
    ffi::BindTexture(ffi::TEXTURE_2D, 0);
    match ffi::GetError() {
        ffi::NO_ERROR => Ok(texture),
        err => Err(err),
    }
}

pub unsafe fn delete_texture(texture: ffi::types::GLuint) {
    ffi::DeleteTextures(1, &texture);
}

pub unsafe fn texture_read_pixels(
    texture: ffi::types::GLuint,
    width: i32,
    height: i32,
) -> Result<Vec<u8>, ffi::types::GLenum> {
    load();
    clear_error_flags();

    let mut fbo = 0;
    let mut buf = vec![0u8; width as usize * height as usize * 4];
    ffi::GenFramebuffers(1, &mut fbo);
    ffi::BindFramebuffer(ffi::FRAMEBUFFER, fbo);
    ffi::FramebufferTexture2D(
        ffi::FRAMEBUFFER,
        ffi::COLOR_ATTACHMENT0,
        ffi::TEXTURE_2D,
        texture,
        0,
    );
    ffi::ReadPixels(
        0,
        0,
        width,
        height,
        ffi::RGBA,
        ffi::UNSIGNED_BYTE,
        buf.as_mut_ptr() as *mut _,
    );
    ffi::BindFramebuffer(ffi::FRAMEBUFFER, 0);
    ffi::DeleteFramebuffers(1, &fbo);
    match ffi::GetError() {
        ffi::NO_ERROR => Ok(buf),
        err => Err(err),
    }
}
