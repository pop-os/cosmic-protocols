// TODO add cursor session support

use cosmic_protocols::{
    image_source::v1::client::{
        zcosmic_output_image_source_manager_v1, zcosmic_toplevel_image_source_manager_v1,
        zcosmic_workspace_image_source_manager_v1,
    },
    screencopy::v2::client::{
        zcosmic_screencopy_frame_v2, zcosmic_screencopy_manager_v2, zcosmic_screencopy_session_v2,
    },
};
use std::{sync::Mutex, time::Duration};
use wayland_client::{
    globals::GlobalList,
    protocol::{wl_buffer, wl_output::Transform, wl_shm},
    Connection, Dispatch, QueueHandle, WEnum,
};

use crate::GlobalData;

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

// TODO Better API than standalone function?
pub fn capture<D, U: ScreencopyFrameDataExt + Send + Sync + 'static>(
    session: &zcosmic_screencopy_session_v2::ZcosmicScreencopySessionV2,
    buffer: &wl_buffer::WlBuffer,
    buffer_damage: &[Rect],
    qh: &QueueHandle<D>,
    udata: U,
) where
    D: Dispatch<zcosmic_screencopy_frame_v2::ZcosmicScreencopyFrameV2, U> + 'static,
{
    let frame = session.create_frame(qh, udata);
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
pub struct ScreencopyState {
    pub screencopy_manager: zcosmic_screencopy_manager_v2::ZcosmicScreencopyManagerV2, // XXX pub
    pub output_source_manager:
        Option<zcosmic_output_image_source_manager_v1::ZcosmicOutputImageSourceManagerV1>,
    pub toplevel_source_manager:
        Option<zcosmic_toplevel_image_source_manager_v1::ZcosmicToplevelImageSourceManagerV1>,
    pub workspace_source_manager:
        Option<zcosmic_workspace_image_source_manager_v1::ZcosmicWorkspaceImageSourceManagerV1>,
}

impl ScreencopyState {
    pub fn try_new<D>(globals: &GlobalList, qh: &QueueHandle<D>) -> Option<Self>
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
        // TODO bind
        let screencopy_manager = globals.bind(qh, 1..=1, GlobalData).ok()?;
        let output_source_manager = globals.bind(qh, 1..=1, GlobalData).ok();
        let toplevel_source_manager = globals.bind(qh, 1..=1, GlobalData).ok();
        let workspace_source_manager = globals.bind(qh, 1..=1, GlobalData).ok();

        Some(Self {
            screencopy_manager,
            output_source_manager,
            toplevel_source_manager,
            workspace_source_manager,
        })
    }

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
        Self::try_new(globals, qh).unwrap()
    }
}

pub trait ScreencopyHandler: Sized {
    fn screencopy_state(&mut self) -> &mut ScreencopyState;

    fn init_done(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        session: &zcosmic_screencopy_session_v2::ZcosmicScreencopySessionV2,
        formats: &Formats,
    );

    fn stopped(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        session: &zcosmic_screencopy_session_v2::ZcosmicScreencopySessionV2,
    );

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
