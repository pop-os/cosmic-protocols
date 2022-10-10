#[cfg(feature = "gl")]
fn main() {
    use gl_generator::{Api, Fallbacks, GlobalGenerator, Profile, Registry};
    use std::{env, fs::File, path::Path};

    let dest = env::var("OUT_DIR").unwrap();
    let dest = Path::new(&dest);
    let mut file = File::create(&dest.join("egl_bindings.rs")).unwrap();

    Registry::new(
        Api::Egl,
        (1, 5),
        Profile::Core,
        Fallbacks::All,
        [
            "EGL_EXT_image_dma_buf_import",
            "EGL_EXT_image_dma_buf_import_modifiers",
        ],
    )
    .write_bindings(GlobalGenerator, &mut file)
    .unwrap();

    let mut file = File::create(&dest.join("gl_bindings.rs")).unwrap();
    Registry::new(
        Api::Gles2,
        (2, 0),
        Profile::Core,
        Fallbacks::None,
        ["GL_OES_EGL_image"],
    )
    .write_bindings(GlobalGenerator, &mut file)
    .unwrap();
}

#[cfg(not(feature = "gl"))]
fn main() {}
