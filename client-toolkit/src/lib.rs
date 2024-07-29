pub use cosmic_protocols;
pub use sctk;
pub use wayland_client;
pub use wayland_protocols;

pub mod keyboard_layout;
pub mod screencopy;
pub mod toplevel_info;
pub mod toplevel_management;
pub mod workspace;

#[doc(hidden)]
#[derive(Debug, Default)]
pub struct GlobalData;
