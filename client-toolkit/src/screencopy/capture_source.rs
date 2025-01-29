use cosmic_protocols::{
    image_source::v1::client::zcosmic_image_source_v1,
    toplevel_info::v1::client::zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1,
    workspace::v1::client::zcosmic_workspace_handle_v1::ZcosmicWorkspaceHandleV1,
};
use std::{error::Error, fmt};
use wayland_client::{protocol::wl_output, Dispatch, QueueHandle};

use super::Capturer;
use crate::GlobalData;

#[derive(Debug)]
pub struct CaptureSourceError(CaptureSourceKind);

impl fmt::Display for CaptureSourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "capture kind '{:?}' unsupported by compositor", self.0)
    }
}

impl Error for CaptureSourceError {}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum CaptureSourceKind {
    Output,
    CosmicToplevel,
    CosmicWorkspace,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum CaptureSource {
    Output(wl_output::WlOutput),
    // TODO: when adding ext protocol
    // Toplevel(ExtForeignToplevelHandleV1),
    CosmicToplevel(ZcosmicToplevelHandleV1),
    CosmicWorkspace(ZcosmicWorkspaceHandleV1),
}

impl CaptureSource {
    pub fn kind(&self) -> CaptureSourceKind {
        match self {
            Self::Output(_) => CaptureSourceKind::Output,
            Self::CosmicToplevel(_) => CaptureSourceKind::CosmicToplevel,
            Self::CosmicWorkspace(_) => CaptureSourceKind::CosmicWorkspace,
        }
    }

    pub(crate) fn create_source<D>(
        &self,
        capturer: &Capturer,
        qh: &QueueHandle<D>,
    ) -> Result<WlCaptureSource, CaptureSourceError>
    where
        D: 'static,
        D: Dispatch<zcosmic_image_source_v1::ZcosmicImageSourceV1, GlobalData>,
    {
        match self {
            CaptureSource::Output(output) => {
                if let Some(manager) = &capturer.0.output_source_manager {
                    return Ok(WlCaptureSource(
                        manager.create_source(output, qh, GlobalData),
                    ));
                }
            }
            CaptureSource::CosmicToplevel(toplevel) => {
                if let Some(manager) = &capturer.0.toplevel_source_manager {
                    return Ok(WlCaptureSource(
                        manager.create_source(toplevel, qh, GlobalData),
                    ));
                }
            }
            CaptureSource::CosmicWorkspace(workspace) => {
                if let Some(manager) = &capturer.0.workspace_source_manager {
                    return Ok(WlCaptureSource(
                        manager.create_source(workspace, qh, GlobalData),
                    ));
                }
            }
        }
        Err(CaptureSourceError(self.kind()))
    }
}

// TODO name?
pub(crate) struct WlCaptureSource(pub(crate) zcosmic_image_source_v1::ZcosmicImageSourceV1);

impl Drop for WlCaptureSource {
    fn drop(&mut self) {
        self.0.destroy();
    }
}
