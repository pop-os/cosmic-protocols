use cosmic_protocols::{
    image_source::v1::client::{
        zcosmic_ext_workspace_image_source_manager_v1, zcosmic_image_source_v1,
        zcosmic_output_image_source_manager_v1, zcosmic_toplevel_image_source_manager_v1,
    },
    screencopy::v2::client::{
        zcosmic_screencopy_frame_v2, zcosmic_screencopy_manager_v2, zcosmic_screencopy_session_v2,
    },
};
use std::time::Duration;
use wayland_client::{protocol::wl_shm, Connection, Dispatch, QueueHandle, WEnum};

use super::super::{
    CaptureFrame, CaptureSession, Rect, ScreencopyFrameDataExt, ScreencopyHandler,
    ScreencopySessionDataExt, ScreencopyState,
};
use crate::GlobalData;

impl<D> Dispatch<zcosmic_screencopy_manager_v2::ZcosmicScreencopyManagerV2, GlobalData, D>
    for ScreencopyState
where
    D: Dispatch<zcosmic_screencopy_manager_v2::ZcosmicScreencopyManagerV2, GlobalData>
        + ScreencopyHandler,
{
    fn event(
        _: &mut D,
        _: &zcosmic_screencopy_manager_v2::ZcosmicScreencopyManagerV2,
        _: zcosmic_screencopy_manager_v2::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        unreachable!()
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
                if let Ok(value) = wl_shm::Format::try_from(format) {
                    formats.lock().unwrap().shm_formats.push(value);
                }
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
                if let Some(session) = udata
                    .screencopy_session_data()
                    .session
                    .get()
                    .unwrap()
                    .upgrade()
                    .map(CaptureSession)
                {
                    app_data.init_done(conn, qh, &session, &formats.lock().unwrap());
                }
            }
            zcosmic_screencopy_session_v2::Event::Stopped => {
                if let Some(session) = udata
                    .screencopy_session_data()
                    .session
                    .get()
                    .unwrap()
                    .upgrade()
                    .map(CaptureSession)
                {
                    app_data.stopped(conn, qh, &session);
                }
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
                app_data.ready(
                    conn,
                    qh,
                    &CaptureFrame::Cosmic(screencopy_frame.clone()),
                    frame,
                );
                screencopy_frame.destroy();
            }
            zcosmic_screencopy_frame_v2::Event::Failed { reason } => {
                // Convert to ext `FailureReason`, which has same definition
                let reason = WEnum::from(match reason {
                    WEnum::Value(value) => value as u32,
                    WEnum::Unknown(value) => value,
                });
                app_data.failed(
                    conn,
                    qh,
                    &CaptureFrame::Cosmic(screencopy_frame.clone()),
                    reason,
                );
                screencopy_frame.destroy();
            }
            _ => unreachable!(),
        }
    }
}

impl<D> Dispatch<zcosmic_image_source_v1::ZcosmicImageSourceV1, GlobalData, D> for ScreencopyState
where
    D: Dispatch<zcosmic_image_source_v1::ZcosmicImageSourceV1, GlobalData> + ScreencopyHandler,
{
    fn event(
        _app_data: &mut D,
        _source: &zcosmic_image_source_v1::ZcosmicImageSourceV1,
        _event: zcosmic_image_source_v1::Event,
        _udata: &GlobalData,
        _conn: &Connection,
        _qh: &QueueHandle<D>,
    ) {
        unreachable!()
    }
}

impl<D>
    Dispatch<
        zcosmic_output_image_source_manager_v1::ZcosmicOutputImageSourceManagerV1,
        GlobalData,
        D,
    > for ScreencopyState
where
    D: Dispatch<
            zcosmic_output_image_source_manager_v1::ZcosmicOutputImageSourceManagerV1,
            GlobalData,
        > + ScreencopyHandler,
{
    fn event(
        _app_data: &mut D,
        _source: &zcosmic_output_image_source_manager_v1::ZcosmicOutputImageSourceManagerV1,
        _event: zcosmic_output_image_source_manager_v1::Event,
        _udata: &GlobalData,
        _conn: &Connection,
        _qh: &QueueHandle<D>,
    ) {
        unreachable!()
    }
}

impl<D>
    Dispatch<
        zcosmic_toplevel_image_source_manager_v1::ZcosmicToplevelImageSourceManagerV1,
        GlobalData,
        D,
    > for ScreencopyState
where
    D: Dispatch<
            zcosmic_toplevel_image_source_manager_v1::ZcosmicToplevelImageSourceManagerV1,
            GlobalData,
        > + ScreencopyHandler,
{
    fn event(
        _app_data: &mut D,
        _source: &zcosmic_toplevel_image_source_manager_v1::ZcosmicToplevelImageSourceManagerV1,
        _event: zcosmic_toplevel_image_source_manager_v1::Event,
        _udata: &GlobalData,
        _conn: &Connection,
        _qh: &QueueHandle<D>,
    ) {
        unreachable!()
    }
}

impl<D>
    Dispatch<
        zcosmic_ext_workspace_image_source_manager_v1::ZcosmicExtWorkspaceImageSourceManagerV1,
        GlobalData,
        D,
    > for ScreencopyState
where
    D: Dispatch<
            zcosmic_ext_workspace_image_source_manager_v1::ZcosmicExtWorkspaceImageSourceManagerV1,
            GlobalData,
        > + ScreencopyHandler,
{
    fn event(
        _app_data: &mut D,
        _source: &zcosmic_ext_workspace_image_source_manager_v1::ZcosmicExtWorkspaceImageSourceManagerV1,
        _event: zcosmic_ext_workspace_image_source_manager_v1::Event,
        _udata: &GlobalData,
        _conn: &Connection,
        _qh: &QueueHandle<D>,
    ) {
        unreachable!()
    }
}

#[macro_export]
macro_rules! delegate_cosmic_screencopy {
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
            $crate::cosmic_protocols::image_source::v1::client::zcosmic_ext_workspace_image_source_manager_v1::ZcosmicExtWorkspaceImageSourceManagerV1: $crate::GlobalData
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
