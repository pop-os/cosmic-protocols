// SPDX-License-Identifier: GPL-3.0-only

//! This crate provides bindings to the COSMIC wayland protocol extensions.
//!
//! These bindings are built on top of the crates wayland-client and wayland-server.
//!
//! Each protocol module contains a `client` and a `server` submodules, for each side of the
//! protocol. The creation of these modules (and the dependency on the associated crate) is
//! controlled by the two cargo features `client` and `server`.

#![warn(missing_docs)]
#![forbid(improper_ctypes, unsafe_op_in_unsafe_fn)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![cfg_attr(rustfmt, rustfmt_skip)]


#[macro_use]
mod protocol_macro;

pub mod a11y {
    //! Accessibility support.

    #[allow(missing_docs)]
    pub mod v1 {
        wayland_protocol!(
            "./unstable/cosmic-a11y-unstable-v1.xml",
            []
        );
    }
}

pub mod atspi {
    //! Atspi accessibility support.

    #[allow(missing_docs)]
    pub mod v1 {
        wayland_protocol!(
            "./unstable/cosmic-atspi-unstable-v1.xml",
            []
        );
    }
}

pub mod corner_radius {
    //! Hint toplevel corner radius values.

    #[allow(missing_docs)]
    pub mod v1 {
        wayland_protocol!(
            "./unstable/cosmic-corner-radius-unstable-v1.xml",
            [wayland_protocols::xdg::shell]
        );
    }
}

pub mod image_capture_source {
    //! Capture source interface extending `ext-image-capture-source-v1`.

    #[allow(missing_docs)]
    pub mod v1 {
        wayland_protocol!(
            "./unstable/cosmic-image-capture-source-unstable-v1.xml",
            [wayland_protocols::ext::image_capture_source::v1, wayland_protocols::ext::workspace::v1]
        );
    }
}

pub mod output_management {
    //! Output management interface.

    #[allow(missing_docs)]
    pub mod v1 {
        wayland_protocol!(
            "./unstable/cosmic-output-management-unstable-v1.xml",
            [wayland_protocols_wlr::output_management::v1]
        );
    }
}

pub mod toplevel_info {
    //! Receive information about toplevel surfaces.

    #[allow(missing_docs)]
    pub mod v1 {
        wayland_protocol!(
            "./unstable/cosmic-toplevel-info-unstable-v1.xml",
            [crate::workspace::v1, wayland_protocols::ext::foreign_toplevel_list::v1, wayland_protocols::ext::workspace::v1]
        );
    }
}

pub mod toplevel_management {
    //! Modify state toplevel surfaces.

    #[allow(missing_docs)]
    pub mod v1 {
        wayland_protocol!(
            "./unstable/cosmic-toplevel-management-unstable-v1.xml",
            [crate::toplevel_info::v1, crate::workspace::v1, wayland_protocols::ext::workspace::v1]
        );
    }
}

pub mod overlap_notify {
    //! Get overlap notifications for layer surfaces

    #[allow(missing_docs)]
    pub mod v1 {
        wayland_protocol!(
            "./unstable/cosmic-overlap-notify-unstable-v1.xml",
            [wayland_protocols_wlr::layer_shell::v1, wayland_protocols::ext::foreign_toplevel_list::v1]
        );
    }
}

pub mod workspace {
    //! Receive information about and control workspaces.

    #[allow(missing_docs)]
    pub mod v1 {
        wayland_protocol!(
            "./unstable/cosmic-workspace-unstable-v1.xml",
            []
        );
    }


    #[allow(missing_docs)]
    pub mod v2 {
        wayland_protocol!(
            "./unstable/cosmic-workspace-unstable-v2.xml",
            [wayland_protocols::ext::workspace::v1]
        );
    }
}
