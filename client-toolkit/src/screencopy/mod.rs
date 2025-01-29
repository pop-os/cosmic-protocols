// TODO add cursor session support

use cosmic_protocols::{
    image_source::v1::client::{
        zcosmic_image_source_v1, zcosmic_output_image_source_manager_v1,
        zcosmic_toplevel_image_source_manager_v1, zcosmic_workspace_image_source_manager_v1,
    },
    screencopy::v2::client::{
        zcosmic_screencopy_frame_v2, zcosmic_screencopy_manager_v2, zcosmic_screencopy_session_v2,
    },
};
use std::{
    sync::{Arc, Mutex, OnceLock, Weak},
    time::Duration,
};
use wayland_client::{
    globals::GlobalList,
    protocol::{wl_buffer, wl_output::Transform, wl_shm},
    Connection, Dispatch, Proxy, QueueHandle, WEnum,
};

use crate::GlobalData;

mod capture_source;
pub use capture_source::{CaptureSource, CaptureSourceError, CaptureSourceKind};
mod dispatch;

#[derive(Clone, Debug)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Clone, Debug)]
pub struct Frame {
    pub transform: WEnum<Transform>,
    pub damage: Vec<Rect>,
    // XXX monotonic? Is this used elsewhere in wayland?
    pub present_time: Option<Duration>,
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            transform: WEnum::Value(Transform::Normal),
            damage: Vec::new(),
            present_time: None,
        }
    }
}

// TODO
// struct CursorInfo {}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Formats {
    pub buffer_size: (u32, u32),
    pub shm_formats: Vec<wl_shm::Format>,
    pub dmabuf_device: Option<libc::dev_t>,
    pub dmabuf_formats: Vec<(u32, Vec<u64>)>,
}

#[derive(Debug)]
struct CapturerInner {
    screencopy_manager: Option<zcosmic_screencopy_manager_v2::ZcosmicScreencopyManagerV2>,
    output_source_manager:
        Option<zcosmic_output_image_source_manager_v1::ZcosmicOutputImageSourceManagerV1>,
    toplevel_source_manager:
        Option<zcosmic_toplevel_image_source_manager_v1::ZcosmicToplevelImageSourceManagerV1>,
    workspace_source_manager:
        Option<zcosmic_workspace_image_source_manager_v1::ZcosmicWorkspaceImageSourceManagerV1>,
}

impl Drop for CapturerInner {
    fn drop(&mut self) {
        if let Some(manager) = &self.screencopy_manager {
            manager.destroy();
        }
        if let Some(manager) = &self.output_source_manager {
            manager.destroy();
        }
        if let Some(manager) = &self.toplevel_source_manager {
            manager.destroy();
        }
        if let Some(manager) = &self.workspace_source_manager {
            manager.destroy();
        }
    }
}

#[derive(Clone, Debug)]
pub struct Capturer(Arc<CapturerInner>);

impl Capturer {
    // TODO check supported capture types

    pub fn create_session<D, U>(
        &self,
        source: &CaptureSource,
        options: zcosmic_screencopy_manager_v2::Options,
        qh: &QueueHandle<D>,
        udata: U,
    ) -> Result<CaptureSession, CaptureSourceError>
    where
        D: 'static,
        D: Dispatch<zcosmic_image_source_v1::ZcosmicImageSourceV1, GlobalData>,
        D: Dispatch<zcosmic_screencopy_session_v2::ZcosmicScreencopySessionV2, U>,
        U: ScreencopySessionDataExt + Send + Sync + 'static,
    {
        let source = source.create_source(self, qh)?;
        Ok(CaptureSession(Arc::new_cyclic(|weak_session| {
            udata
                .screencopy_session_data()
                .session
                .set(weak_session.clone())
                .unwrap();
            CaptureSessionInner(
                self.0
                    .screencopy_manager
                    .as_ref()
                    .unwrap()
                    .create_session(&source.0, options, qh, udata),
            )
        })))
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct CaptureSessionInner(zcosmic_screencopy_session_v2::ZcosmicScreencopySessionV2);

impl Drop for CaptureSessionInner {
    fn drop(&mut self) {
        self.0.destroy();
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CaptureSession(Arc<CaptureSessionInner>);

impl CaptureSession {
    pub fn capture<D, U>(
        &self,
        buffer: &wl_buffer::WlBuffer,
        buffer_damage: &[Rect],
        qh: &QueueHandle<D>,
        udata: U,
    ) -> zcosmic_screencopy_frame_v2::ZcosmicScreencopyFrameV2
    where
        D: Dispatch<zcosmic_screencopy_frame_v2::ZcosmicScreencopyFrameV2, U> + 'static,
        U: ScreencopyFrameDataExt + Send + Sync + 'static,
    {
        let frame = self.0 .0.create_frame(qh, udata);
        frame.attach_buffer(buffer);
        for Rect {
            x,
            y,
            width,
            height,
        } in buffer_damage
        {
            frame.damage_buffer(*x, *y, *width, *height);
        }
        frame.capture();
        frame
    }

    pub fn data<U: Send + Sync + 'static>(&self) -> Option<&U> {
        self.0 .0.data()
    }
}

#[derive(Debug)]
pub struct ScreencopyState {
    capturer: Capturer,
}

impl ScreencopyState {
    pub fn new<D>(globals: &GlobalList, qh: &QueueHandle<D>) -> Self
    where
        D: 'static,
        D: Dispatch<zcosmic_screencopy_manager_v2::ZcosmicScreencopyManagerV2, GlobalData>,
        D: Dispatch<
            zcosmic_output_image_source_manager_v1::ZcosmicOutputImageSourceManagerV1,
            GlobalData,
        >,
        D: Dispatch<
            zcosmic_toplevel_image_source_manager_v1::ZcosmicToplevelImageSourceManagerV1,
            GlobalData,
        >,
        D: Dispatch<
            zcosmic_workspace_image_source_manager_v1::ZcosmicWorkspaceImageSourceManagerV1,
            GlobalData,
        >,
    {
        let screencopy_manager = globals.bind(qh, 1..=1, GlobalData).ok();
        let output_source_manager = globals.bind(qh, 1..=1, GlobalData).ok();
        let toplevel_source_manager = globals.bind(qh, 1..=1, GlobalData).ok();
        let workspace_source_manager = globals.bind(qh, 1..=1, GlobalData).ok();

        let capturer = Capturer(Arc::new(CapturerInner {
            screencopy_manager,
            output_source_manager,
            toplevel_source_manager,
            workspace_source_manager,
        }));

        Self { capturer }
    }

    pub fn capturer(&self) -> &Capturer {
        &self.capturer
    }
}

pub trait ScreencopyHandler: Sized {
    fn screencopy_state(&mut self) -> &mut ScreencopyState;

    fn init_done(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        session: &CaptureSession,
        formats: &Formats,
    );

    fn stopped(&mut self, conn: &Connection, qh: &QueueHandle<Self>, session: &CaptureSession);

    fn ready(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        screencopy_frame: &zcosmic_screencopy_frame_v2::ZcosmicScreencopyFrameV2,
        frame: Frame,
    );

    fn failed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        screencopy_frame: &zcosmic_screencopy_frame_v2::ZcosmicScreencopyFrameV2,
        reason: WEnum<zcosmic_screencopy_frame_v2::FailureReason>,
    );
}

pub trait ScreencopySessionDataExt {
    fn screencopy_session_data(&self) -> &ScreencopySessionData;
}

#[derive(Default)]
pub struct ScreencopySessionData {
    formats: Mutex<Formats>,
    session: OnceLock<Weak<CaptureSessionInner>>,
}

impl ScreencopySessionDataExt for ScreencopySessionData {
    fn screencopy_session_data(&self) -> &ScreencopySessionData {
        self
    }
}

#[derive(Default)]
pub struct ScreencopyFrameData {
    frame: Mutex<Frame>,
}

pub trait ScreencopyFrameDataExt {
    fn screencopy_frame_data(&self) -> &ScreencopyFrameData;
}

impl ScreencopyFrameDataExt for ScreencopyFrameData {
    fn screencopy_frame_data(&self) -> &ScreencopyFrameData {
        self
    }
}

#[macro_export]
macro_rules! delegate_screencopy {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::delegate_screencopy($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty,
            session: $crate::screencopy::ScreencopySessionData, frame: $crate::screencopy::ScreencopyFrameData);
    };
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty, session: [$($session_data:ty),* $(,)?], frame: [$($frame_data:ty),* $(,)?]) => {
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::cosmic_protocols::image_source::v1::client::zcosmic_output_image_source_manager_v1::ZcosmicOutputImageSourceManagerV1: $crate::GlobalData
        ] => $crate::screencopy::ScreencopyState);
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::cosmic_protocols::image_source::v1::client::zcosmic_toplevel_image_source_manager_v1::ZcosmicToplevelImageSourceManagerV1: $crate::GlobalData
        ] => $crate::screencopy::ScreencopyState);
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::cosmic_protocols::image_source::v1::client::zcosmic_workspace_image_source_manager_v1::ZcosmicWorkspaceImageSourceManagerV1: $crate::GlobalData
        ] => $crate::screencopy::ScreencopyState);
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::cosmic_protocols::image_source::v1::client::zcosmic_image_source_v1::ZcosmicImageSourceV1: $crate::GlobalData
        ] => $crate::screencopy::ScreencopyState);
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::cosmic_protocols::screencopy::v2::client::zcosmic_screencopy_manager_v2::ZcosmicScreencopyManagerV2: $crate::GlobalData
        ] => $crate::screencopy::ScreencopyState);
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $(
                $crate::cosmic_protocols::screencopy::v2::client::zcosmic_screencopy_session_v2::ZcosmicScreencopySessionV2: $session_data
            ),*
        ] => $crate::screencopy::ScreencopyState);
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $(
                $crate::cosmic_protocols::screencopy::v2::client::zcosmic_screencopy_frame_v2::ZcosmicScreencopyFrameV2: $frame_data
            ),*
        ] => $crate::screencopy::ScreencopyState);
    };
}
