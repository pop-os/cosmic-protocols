// TODO add cursor session support

use cosmic_protocols::{
    image_source::v1::client::{
        zcosmic_image_source_v1, zcosmic_output_image_source_manager_v1,
        zcosmic_toplevel_image_source_manager_v1, zcosmic_workspace_image_source_manager_v2,
    },
    screencopy::v2::client::{
        zcosmic_screencopy_frame_v2, zcosmic_screencopy_manager_v2, zcosmic_screencopy_session_v2,
    },
};
use std::{sync::Mutex, time::Duration};
use wayland_client::{
    globals::GlobalList,
    protocol::{wl_buffer, wl_output::Transform},
    Connection, Dispatch, QueueHandle, WEnum,
};

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

struct CursorInfo {}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Formats {
    pub buffer_size: (u32, u32),
    pub shm_formats: Vec<u32>,
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
        Option<zcosmic_workspace_image_source_manager_v2::ZcosmicWorkspaceImageSourceManagerV2>,
}

impl ScreencopyState {
    pub fn new<D>(globals: &GlobalList, qh: &QueueHandle<D>) -> Self
    where
        D: 'static,
        D: Dispatch<zcosmic_screencopy_manager_v2::ZcosmicScreencopyManagerV2, ()>,
        D: Dispatch<zcosmic_output_image_source_manager_v1::ZcosmicOutputImageSourceManagerV1, ()>,
        D: Dispatch<
            zcosmic_toplevel_image_source_manager_v1::ZcosmicToplevelImageSourceManagerV1,
            (),
        >,
        D: Dispatch<
            zcosmic_workspace_image_source_manager_v2::ZcosmicWorkspaceImageSourceManagerV2,
            (),
        >,
    {
        // TODO bind
        let screencopy_manager = globals.bind(qh, 1..=1, ()).unwrap(); // XXX
        let output_source_manager = globals.bind(qh, 1..=1, ()).ok();
        let toplevel_source_manager = globals.bind(qh, 1..=1, ()).ok();
        let workspace_source_manager = globals.bind(qh, 1..=1, ()).ok();
        Self {
            screencopy_manager,
            output_source_manager,
            toplevel_source_manager,
            workspace_source_manager,
        }
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

impl<D> Dispatch<zcosmic_screencopy_manager_v2::ZcosmicScreencopyManagerV2, (), D>
    for ScreencopyState
where
    D: Dispatch<zcosmic_screencopy_manager_v2::ZcosmicScreencopyManagerV2, ()> + ScreencopyHandler,
{
    fn event(
        state: &mut D,
        _: &zcosmic_screencopy_manager_v2::ZcosmicScreencopyManagerV2,
        event: zcosmic_screencopy_manager_v2::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        match event {
            _ => unreachable!(),
        }
    }
}

impl<D, U> Dispatch<zcosmic_screencopy_session_v2::ZcosmicScreencopySessionV2, U, D>
    for ScreencopyState
where
    D: Dispatch<zcosmic_screencopy_session_v2::ZcosmicScreencopySessionV2, U> + ScreencopyHandler,
    U: ScreencopySessionDataExt,
{
    fn event(
        app_data: &mut D,
        session: &zcosmic_screencopy_session_v2::ZcosmicScreencopySessionV2,
        event: zcosmic_screencopy_session_v2::Event,
        udata: &U,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        let formats = &udata.screencopy_session_data().formats;
        match event {
            zcosmic_screencopy_session_v2::Event::BufferSize { width, height } => {
                formats.lock().unwrap().buffer_size = (width, height);
            }
            zcosmic_screencopy_session_v2::Event::ShmFormat { format } => {
                formats.lock().unwrap().shm_formats.push(format);
            }
            zcosmic_screencopy_session_v2::Event::DmabufDevice { device } => {
                let device = libc::dev_t::from_ne_bytes(device.try_into().unwrap());
                formats.lock().unwrap().dmabuf_device = Some(device);
            }
            zcosmic_screencopy_session_v2::Event::DmabufFormat { format, modifiers } => {
                let modifiers = modifiers
                    .chunks_exact(8)
                    .map(|x| u64::from_ne_bytes(x.try_into().unwrap()))
                    .collect();
                formats
                    .lock()
                    .unwrap()
                    .dmabuf_formats
                    .push((format, modifiers));
            }
            zcosmic_screencopy_session_v2::Event::Done => {
                app_data.init_done(conn, qh, session, &*formats.lock().unwrap());
            }
            zcosmic_screencopy_session_v2::Event::Stopped => {
                app_data.stopped(conn, qh, session);
                session.destroy();
            }
            _ => unreachable!(),
        }
    }
}

impl<D, U> Dispatch<zcosmic_screencopy_frame_v2::ZcosmicScreencopyFrameV2, U, D> for ScreencopyState
where
    D: Dispatch<zcosmic_screencopy_frame_v2::ZcosmicScreencopyFrameV2, U> + ScreencopyHandler,
    U: ScreencopyFrameDataExt,
{
    fn event(
        app_data: &mut D,
        screencopy_frame: &zcosmic_screencopy_frame_v2::ZcosmicScreencopyFrameV2,
        event: zcosmic_screencopy_frame_v2::Event,
        udata: &U,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        let frame = &udata.screencopy_frame_data().frame;
        match event {
            zcosmic_screencopy_frame_v2::Event::Transform { transform } => {
                frame.lock().unwrap().transform = transform;
            }
            zcosmic_screencopy_frame_v2::Event::Damage {
                x,
                y,
                width,
                height,
            } => {
                frame.lock().unwrap().damage.push(Rect {
                    x,
                    y,
                    width,
                    height,
                });
            }
            zcosmic_screencopy_frame_v2::Event::PresentationTime {
                tv_sec_hi,
                tv_sec_lo,
                tv_nsec,
            } => {
                let secs = (u64::from(tv_sec_hi) << 32) + u64::from(tv_sec_lo);
                let duration = Duration::new(secs, tv_nsec);
                frame.lock().unwrap().present_time = Some(duration);
            }
            zcosmic_screencopy_frame_v2::Event::Ready => {
                let frame = frame.lock().unwrap().clone();
                app_data.ready(conn, qh, screencopy_frame, frame);
                screencopy_frame.destroy();
            }
            zcosmic_screencopy_frame_v2::Event::Failed { reason } => {
                app_data.failed(conn, qh, screencopy_frame, reason);
                screencopy_frame.destroy();
            }
            _ => unreachable!(),
        }
    }
}

impl<D> Dispatch<zcosmic_image_source_v1::ZcosmicImageSourceV1, (), D> for ScreencopyState
where
    D: Dispatch<zcosmic_image_source_v1::ZcosmicImageSourceV1, ()> + ScreencopyHandler,
{
    fn event(
        app_data: &mut D,
        source: &zcosmic_image_source_v1::ZcosmicImageSourceV1,
        event: zcosmic_image_source_v1::Event,
        udata: &(),
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        unreachable!()
    }
}

impl<D> Dispatch<zcosmic_output_image_source_manager_v1::ZcosmicOutputImageSourceManagerV1, (), D>
    for ScreencopyState
where
    D: Dispatch<zcosmic_output_image_source_manager_v1::ZcosmicOutputImageSourceManagerV1, ()>
        + ScreencopyHandler,
{
    fn event(
        app_data: &mut D,
        source: &zcosmic_output_image_source_manager_v1::ZcosmicOutputImageSourceManagerV1,
        event: zcosmic_output_image_source_manager_v1::Event,
        udata: &(),
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        unreachable!()
    }
}

impl<D>
    Dispatch<zcosmic_toplevel_image_source_manager_v1::ZcosmicToplevelImageSourceManagerV1, (), D>
    for ScreencopyState
where
    D: Dispatch<zcosmic_toplevel_image_source_manager_v1::ZcosmicToplevelImageSourceManagerV1, ()>
        + ScreencopyHandler,
{
    fn event(
        app_data: &mut D,
        source: &zcosmic_toplevel_image_source_manager_v1::ZcosmicToplevelImageSourceManagerV1,
        event: zcosmic_toplevel_image_source_manager_v1::Event,
        udata: &(),
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        unreachable!()
    }
}

impl<D>
    Dispatch<zcosmic_workspace_image_source_manager_v2::ZcosmicWorkspaceImageSourceManagerV2, (), D>
    for ScreencopyState
where
    D: Dispatch<
            zcosmic_workspace_image_source_manager_v2::ZcosmicWorkspaceImageSourceManagerV2,
            (),
        > + ScreencopyHandler,
{
    fn event(
        app_data: &mut D,
        source: &zcosmic_workspace_image_source_manager_v2::ZcosmicWorkspaceImageSourceManagerV2,
        event: zcosmic_workspace_image_source_manager_v2::Event,
        udata: &(),
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        unreachable!()
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
            $crate::cosmic_protocols::image_source::v1::client::zcosmic_output_image_source_manager_v1::ZcosmicOutputImageSourceManagerV1: ()
        ] => $crate::screencopy::ScreencopyState);
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::cosmic_protocols::image_source::v1::client::zcosmic_toplevel_image_source_manager_v1::ZcosmicToplevelImageSourceManagerV1: ()
        ] => $crate::screencopy::ScreencopyState);
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::cosmic_protocols::image_source::v1::client::zcosmic_workspace_image_source_manager_v2::ZcosmicWorkspaceImageSourceManagerV1: ()
        ] => $crate::screencopy::ScreencopyState);
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::cosmic_protocols::image_source::v1::client::zcosmic_image_source_v1::ZcosmicImageSourceV1: ()
        ] => $crate::screencopy::ScreencopyState);
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::cosmic_protocols::screencopy::v2::client::zcosmic_screencopy_manager_v2::ZcosmicScreencopyManagerV2: ()
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
