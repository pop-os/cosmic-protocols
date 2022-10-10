use std::{
    ffi::{c_void, CString},
    os::unix::io::AsRawFd,
    sync::Once,
};

use crate::export_dmabuf::DmabufFrame;

#[allow(non_camel_case_types)]
mod ffi {
    use std::ffi::{c_char, c_long, c_void};

    #[link(name = "EGL")]
    extern "C" {
        pub fn eglGetProcAddress(procname: *const c_char) -> *mut c_void;
    }

    pub type khronos_utime_nanoseconds_t = khronos_uint64_t;
    pub type khronos_uint64_t = u64;
    pub type khronos_ssize_t = c_long;
    pub type EGLint = i32;
    pub type EGLNativeDisplayType = NativeDisplayType;
    pub type EGLNativePixmapType = NativePixmapType;
    pub type EGLNativeWindowType = NativeWindowType;
    pub type NativeDisplayType = *const c_void;
    pub type NativePixmapType = *const c_void;
    pub type NativeWindowType = *const c_void;

    include!(concat!(env!("OUT_DIR"), "/egl_bindings.rs"));
}

static DMA_BUF_PLANE_ATTRIBS: &[(
    ffi::types::EGLenum,
    ffi::types::EGLenum,
    ffi::types::EGLenum,
    ffi::types::EGLenum,
    ffi::types::EGLenum,
)] = &[
    (
        ffi::DMA_BUF_PLANE0_FD_EXT,
        ffi::DMA_BUF_PLANE0_MODIFIER_HI_EXT,
        ffi::DMA_BUF_PLANE0_MODIFIER_LO_EXT,
        ffi::DMA_BUF_PLANE0_OFFSET_EXT,
        ffi::DMA_BUF_PLANE0_PITCH_EXT,
    ),
    (
        ffi::DMA_BUF_PLANE1_FD_EXT,
        ffi::DMA_BUF_PLANE1_MODIFIER_HI_EXT,
        ffi::DMA_BUF_PLANE1_MODIFIER_LO_EXT,
        ffi::DMA_BUF_PLANE1_OFFSET_EXT,
        ffi::DMA_BUF_PLANE1_PITCH_EXT,
    ),
    (
        ffi::DMA_BUF_PLANE2_FD_EXT,
        ffi::DMA_BUF_PLANE2_MODIFIER_HI_EXT,
        ffi::DMA_BUF_PLANE2_MODIFIER_LO_EXT,
        ffi::DMA_BUF_PLANE2_OFFSET_EXT,
        ffi::DMA_BUF_PLANE2_PITCH_EXT,
    ),
    (
        ffi::DMA_BUF_PLANE3_FD_EXT,
        ffi::DMA_BUF_PLANE3_MODIFIER_HI_EXT,
        ffi::DMA_BUF_PLANE3_MODIFIER_LO_EXT,
        ffi::DMA_BUF_PLANE3_OFFSET_EXT,
        ffi::DMA_BUF_PLANE3_PITCH_EXT,
    ),
];

pub(crate) fn get_proc_address(procname: &str) -> *const c_void {
    let procname = CString::new(procname.as_bytes()).unwrap();
    unsafe { ffi::eglGetProcAddress(procname.as_ptr()) }
}

fn load() {
    static LOADED: Once = Once::new();
    LOADED.call_once(|| {
        ffi::load_with(get_proc_address);
    });
}

pub struct EGLImage {
    display: ffi::types::EGLDisplay,
    ptr: ffi::types::EGLImage,
}

impl EGLImage {
    pub fn ptr(&self) -> ffi::types::EGLImage {
        self.ptr
    }

    pub unsafe fn import_dmabuf(
        display: ffi::types::EGLDisplay,
        frame: &DmabufFrame,
    ) -> Result<Self, ffi::types::EGLint> {
        load();

        // TODO Check for extensions?

        let mut attribs: Vec<ffi::types::EGLAttrib> = Vec::new();
        attribs.extend_from_slice(&[
            ffi::WIDTH as _,
            frame.width as _,
            ffi::HEIGHT as _,
            frame.height as _,
            ffi::LINUX_DRM_FOURCC_EXT as _,
            frame.format as _,
        ]);

        for ((fd, mod_hi, mod_lo, offset, pitch), plane) in DMA_BUF_PLANE_ATTRIBS
            .iter()
            .copied()
            .zip(frame.objects.iter())
        {
            attribs.extend_from_slice(&[
                fd as _,
                plane.fd.as_raw_fd() as _,
                mod_hi as _,
                (frame.modifier >> 32) as _,
                mod_lo as _,
                (frame.modifier & 0xFFFFFFFF) as _,
                offset as _,
                plane.offset as _,
                pitch as _,
                plane.stride as _,
            ]);
        }

        attribs.push(ffi::NONE as _);

        let ptr = ffi::CreateImage(
            display,
            ffi::NO_CONTEXT,
            ffi::LINUX_DMA_BUF_EXT,
            std::ptr::null(),
            attribs.as_ptr(),
        );
        if ptr.is_null() {
            Err(ffi::GetError())
        } else {
            Ok(EGLImage { display, ptr })
        }
    }
}

impl Drop for EGLImage {
    fn drop(&mut self) {
        unsafe {
            ffi::DestroyImage(self.display, self.ptr);
        }
    }
}

// TODO: Way to create context for GPU, to read out pixels?
// EGLDevice, eglCreateContext, eglGetCurrentContext, eglMakeCurrent
