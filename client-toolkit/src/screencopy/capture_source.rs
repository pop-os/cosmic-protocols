use cosmic_protocols::{
    image_source::v1::client::zcosmic_image_source_v1,
    workspace::v1::client::zcosmic_workspace_handle_v1::ZcosmicWorkspaceHandleV1,
};
use std::{error::Error, fmt};
use wayland_client::{protocol::wl_output, Dispatch, Proxy, QueueHandle};
use wayland_protocols::ext::{
    foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
    image_capture_source::v1::client::ext_image_capture_source_v1,
};

use super::Capturer;
use crate::{toplevel_info::ToplevelUserData, GlobalData};

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
    Toplevel,
    CosmicWorkspace,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum CaptureSource {
    Output(wl_output::WlOutput),
    Toplevel(ExtForeignToplevelHandleV1),
    CosmicWorkspace(ZcosmicWorkspaceHandleV1),
}

impl CaptureSource {
    pub fn kind(&self) -> CaptureSourceKind {
        match self {
            Self::Output(_) => CaptureSourceKind::Output,
            Self::Toplevel(_) => CaptureSourceKind::Toplevel,
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
        D: Dispatch<ext_image_capture_source_v1::ExtImageCaptureSourceV1, GlobalData>,
    {
        if let Some(image_copy_capture) = &capturer.0.image_copy_capture {
            match self {
                CaptureSource::Output(output) => {
                    if let Some(manager) = &image_copy_capture.output_source_manager {
                        return Ok(WlCaptureSource::Ext(
                            manager.create_source(output, qh, GlobalData),
                        ));
                    }
                }
                CaptureSource::Toplevel(toplevel) => {
                    if let Some(manager) = &image_copy_capture.foreign_toplevel_source_manager {
                        return Ok(WlCaptureSource::Ext(
                            manager.create_source(toplevel, qh, GlobalData),
                        ));
                    }
                }
                CaptureSource::CosmicWorkspace(_) => {}
            }
        }
        if let Some(cosmic_screencopy) = &capturer.0.cosmic_screencopy {
            match self {
                CaptureSource::Output(output) => {
                    if let Some(manager) = &cosmic_screencopy.output_source_manager {
                        return Ok(WlCaptureSource::Cosmic(
                            manager.create_source(output, qh, GlobalData),
                        ));
                    }
                }
                CaptureSource::Toplevel(toplevel) => {
                    if let Some(cosmic_toplevel) = toplevel
                        .data::<ToplevelUserData>()
                        .and_then(|data| data.cosmic_toplevel())
                    {
                        if let Some(manager) = &cosmic_screencopy.toplevel_source_manager {
                            return Ok(WlCaptureSource::Cosmic(manager.create_source(
                                &cosmic_toplevel,
                                qh,
                                GlobalData,
                            )));
                        }
                    }
                }
                CaptureSource::CosmicWorkspace(workspace) => {
                    if let Some(manager) = &cosmic_screencopy.workspace_source_manager {
                        return Ok(WlCaptureSource::Cosmic(
                            manager.create_source(workspace, qh, GlobalData),
                        ));
                    }
                }
            }
        }
        Err(CaptureSourceError(self.kind()))
    }
}

// TODO name?
pub(crate) enum WlCaptureSource {
    Cosmic(zcosmic_image_source_v1::ZcosmicImageSourceV1),
    Ext(ext_image_capture_source_v1::ExtImageCaptureSourceV1),
}

impl Drop for WlCaptureSource {
    fn drop(&mut self) {
        match self {
            Self::Cosmic(source) => source.destroy(),
            Self::Ext(source) => source.destroy(),
        }
    }
}
