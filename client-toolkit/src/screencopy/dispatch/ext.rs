use std::time::Duration;
use wayland_client::{Connection, Dispatch, QueueHandle, WEnum};
use wayland_protocols::ext::{
    image_capture_source::v1::client::{
        ext_foreign_toplevel_image_capture_source_manager_v1, ext_image_capture_source_v1,
        ext_output_image_capture_source_manager_v1,
    },
    image_copy_capture::v1::client::{
        ext_image_copy_capture_frame_v1, ext_image_copy_capture_manager_v1,
        ext_image_copy_capture_session_v1,
    },
};

use super::super::{
    CaptureFrame, CaptureSession, Rect, ScreencopyFrameDataExt, ScreencopyHandler,
    ScreencopySessionDataExt, ScreencopyState,
};
use crate::GlobalData;

impl<D> Dispatch<ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1, GlobalData, D>
    for ScreencopyState
where
    D: Dispatch<ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1, GlobalData>
        + ScreencopyHandler,
{
    fn event(
        _: &mut D,
        _: &ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1,
        _: ext_image_copy_capture_manager_v1::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        unreachable!()
    }
}

impl<D, U> Dispatch<ext_image_copy_capture_session_v1::ExtImageCopyCaptureSessionV1, U, D>
    for ScreencopyState
where
    D: Dispatch<ext_image_copy_capture_session_v1::ExtImageCopyCaptureSessionV1, U>
        + ScreencopyHandler,
    U: ScreencopySessionDataExt,
{
    fn event(
        app_data: &mut D,
        session: &ext_image_copy_capture_session_v1::ExtImageCopyCaptureSessionV1,
        event: ext_image_copy_capture_session_v1::Event,
        udata: &U,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        let formats = &udata.screencopy_session_data().formats;
        match event {
            ext_image_copy_capture_session_v1::Event::BufferSize { width, height } => {
                formats.lock().unwrap().buffer_size = (width, height);
            }
            ext_image_copy_capture_session_v1::Event::ShmFormat { format } => {
                if let WEnum::Value(value) = format {
                    formats.lock().unwrap().shm_formats.push(value);
                }
            }
            ext_image_copy_capture_session_v1::Event::DmabufDevice { device } => {
                let device = libc::dev_t::from_ne_bytes(device.try_into().unwrap());
                formats.lock().unwrap().dmabuf_device = Some(device);
            }
            ext_image_copy_capture_session_v1::Event::DmabufFormat { format, modifiers } => {
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
            ext_image_copy_capture_session_v1::Event::Done => {
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
            ext_image_copy_capture_session_v1::Event::Stopped => {
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

impl<D, U> Dispatch<ext_image_copy_capture_frame_v1::ExtImageCopyCaptureFrameV1, U, D>
    for ScreencopyState
where
    D: Dispatch<ext_image_copy_capture_frame_v1::ExtImageCopyCaptureFrameV1, U> + ScreencopyHandler,
    U: ScreencopyFrameDataExt,
{
    fn event(
        app_data: &mut D,
        screencopy_frame: &ext_image_copy_capture_frame_v1::ExtImageCopyCaptureFrameV1,
        event: ext_image_copy_capture_frame_v1::Event,
        udata: &U,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        let frame = &udata.screencopy_frame_data().frame;
        match event {
            ext_image_copy_capture_frame_v1::Event::Transform { transform } => {
                frame.lock().unwrap().transform = transform;
            }
            ext_image_copy_capture_frame_v1::Event::Damage {
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
            ext_image_copy_capture_frame_v1::Event::PresentationTime {
                tv_sec_hi,
                tv_sec_lo,
                tv_nsec,
            } => {
                let secs = (u64::from(tv_sec_hi) << 32) + u64::from(tv_sec_lo);
                let duration = Duration::new(secs, tv_nsec);
                frame.lock().unwrap().present_time = Some(duration);
            }
            ext_image_copy_capture_frame_v1::Event::Ready => {
                let frame = frame.lock().unwrap().clone();
                app_data.ready(
                    conn,
                    qh,
                    &CaptureFrame::Ext(screencopy_frame.clone()),
                    frame,
                );
                screencopy_frame.destroy();
            }
            ext_image_copy_capture_frame_v1::Event::Failed { reason } => {
                app_data.failed(
                    conn,
                    qh,
                    &CaptureFrame::Ext(screencopy_frame.clone()),
                    reason,
                );
                screencopy_frame.destroy();
            }
            _ => unreachable!(),
        }
    }
}

impl<D> Dispatch<ext_image_capture_source_v1::ExtImageCaptureSourceV1, GlobalData, D>
    for ScreencopyState
where
    D: Dispatch<ext_image_capture_source_v1::ExtImageCaptureSourceV1, GlobalData>
        + ScreencopyHandler,
{
    fn event(
        _app_data: &mut D,
        _source: &ext_image_capture_source_v1::ExtImageCaptureSourceV1,
        _event: ext_image_capture_source_v1::Event,
        _udata: &GlobalData,
        _conn: &Connection,
        _qh: &QueueHandle<D>,
    ) {
        unreachable!()
    }
}

impl<D>
    Dispatch<
        ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1,
        GlobalData,
        D,
    > for ScreencopyState
where
    D: Dispatch<
            ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1,
            GlobalData,
        > + ScreencopyHandler,
{
    fn event(
        _app_data: &mut D,
        _source: &ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1,
        _event: ext_output_image_capture_source_manager_v1::Event,
        _udata: &GlobalData,
        _conn: &Connection,
        _qh: &QueueHandle<D>,
    ) {
        unreachable!()
    }
}

impl<D>
    Dispatch<
        ext_foreign_toplevel_image_capture_source_manager_v1::ExtForeignToplevelImageCaptureSourceManagerV1,
        GlobalData,
        D,
    > for ScreencopyState
where
    D: Dispatch<
            ext_foreign_toplevel_image_capture_source_manager_v1::ExtForeignToplevelImageCaptureSourceManagerV1,
            GlobalData,
        > + ScreencopyHandler,
{
    fn event(
        _app_data: &mut D,
        _source: &ext_foreign_toplevel_image_capture_source_manager_v1::ExtForeignToplevelImageCaptureSourceManagerV1,
        _event: ext_foreign_toplevel_image_capture_source_manager_v1::Event,
        _udata: &GlobalData,
        _conn: &Connection,
        _qh: &QueueHandle<D>,
    ) {
        unreachable!()
    }
}

#[macro_export]
macro_rules! delegate_ext_image_capture {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::delegate_screencopy($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty,
            session: $crate::screencopy::ScreencopySessionData, frame: $crate::screencopy::ScreencopyFrameData);
    };
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty, session: [$($session_data:ty),* $(,)?], frame: [$($frame_data:ty),* $(,)?]) => {
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::wayland_protocols::ext::image_capture_source::v1::client::ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1: $crate::GlobalData
        ] => $crate::screencopy::ScreencopyState);
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::wayland_protocols::ext::image_capture_source::v1::client::ext_foreign_toplevel_image_capture_source_manager_v1::ExtForeignToplevelImageCaptureSourceManagerV1: $crate::GlobalData
        ] => $crate::screencopy::ScreencopyState);
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::wayland_protocols::ext::image_capture_source::v1::client::ext_image_capture_source_v1::ExtImageCaptureSourceV1: $crate::GlobalData
        ] => $crate::screencopy::ScreencopyState);
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1: $crate::GlobalData
        ] => $crate::screencopy::ScreencopyState);
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $(
                $crate::wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_session_v1::ExtImageCopyCaptureSessionV1: $session_data
            ),*
        ] => $crate::screencopy::ScreencopyState);
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $(
                $crate::wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_frame_v1::ExtImageCopyCaptureFrameV1: $frame_data
            ),*
        ] => $crate::screencopy::ScreencopyState);
    };
}
