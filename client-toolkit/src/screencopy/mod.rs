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

use crate::GlobalData;

mod capture_source;
use capture_source::WlCaptureSource;
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
struct CosmicScreencopy {
    screencopy_manager: zcosmic_screencopy_manager_v2::ZcosmicScreencopyManagerV2,
    output_source_manager:
        Option<zcosmic_output_image_source_manager_v1::ZcosmicOutputImageSourceManagerV1>,
    toplevel_source_manager:
        Option<zcosmic_toplevel_image_source_manager_v1::ZcosmicToplevelImageSourceManagerV1>,
    workspace_source_manager:
        Option<zcosmic_workspace_image_source_manager_v1::ZcosmicWorkspaceImageSourceManagerV1>,
}

impl CosmicScreencopy {
    fn new<D>(globals: &GlobalList, qh: &QueueHandle<D>) -> Option<Self>
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
}

#[derive(Debug)]
struct ImageCopyCapture {
    image_copy_capture_manager: ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1,
    output_source_manager: Option<ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1>,
    foreign_toplevel_source_manager: Option<ext_foreign_toplevel_image_capture_source_manager_v1::ExtForeignToplevelImageCaptureSourceManagerV1>,
}

impl ImageCopyCapture {
    fn new<D>(globals: &GlobalList, qh: &QueueHandle<D>) -> Option<Self>
    where
        D: 'static,
        D: Dispatch<ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1, GlobalData>,
        D: Dispatch<ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1, GlobalData>,
        D: Dispatch<ext_foreign_toplevel_image_capture_source_manager_v1::ExtForeignToplevelImageCaptureSourceManagerV1, GlobalData>,
    {
        let image_copy_capture_manager = globals.bind(qh, 1..=1, GlobalData).ok()?;
        let output_source_manager = globals.bind(qh, 1..=1, GlobalData).ok();
        let foreign_toplevel_source_manager = globals.bind(qh, 1..=1, GlobalData).ok();
        Some(Self {
            image_copy_capture_manager,
            output_source_manager,
            foreign_toplevel_source_manager,
        })
    }
}

#[derive(Debug)]
struct CapturerInner {
    cosmic_screencopy: Option<CosmicScreencopy>,
    image_copy_capture: Option<ImageCopyCapture>,
}

impl Drop for CapturerInner {
    fn drop(&mut self) {
        if let Some(cosmic_screencopy) = &self.cosmic_screencopy {
            cosmic_screencopy.screencopy_manager.destroy();
            if let Some(manager) = &cosmic_screencopy.output_source_manager {
                manager.destroy();
            }
            if let Some(manager) = &cosmic_screencopy.toplevel_source_manager {
                manager.destroy();
            }
            if let Some(manager) = &cosmic_screencopy.workspace_source_manager {
                manager.destroy();
            }
        }
        if let Some(image_copy_capture) = &self.image_copy_capture {
            image_copy_capture.image_copy_capture_manager.destroy();
            if let Some(manager) = &image_copy_capture.output_source_manager {
                manager.destroy();
            }
            if let Some(manager) = &image_copy_capture.foreign_toplevel_source_manager {
                manager.destroy();
            }
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
        options: ext_image_copy_capture_manager_v1::Options,
        qh: &QueueHandle<D>,
        udata: U,
    ) -> Result<CaptureSession, CaptureSourceError>
    where
        D: 'static,
        D: Dispatch<zcosmic_image_source_v1::ZcosmicImageSourceV1, GlobalData>,
        D: Dispatch<zcosmic_screencopy_session_v2::ZcosmicScreencopySessionV2, U>,
        D: Dispatch<ext_image_capture_source_v1::ExtImageCaptureSourceV1, GlobalData>,
        D: Dispatch<ext_image_copy_capture_session_v1::ExtImageCopyCaptureSessionV1, U>,
        U: ScreencopySessionDataExt + Send + Sync + 'static,
    {
        let source = source.create_source(self, qh)?;
        Ok(CaptureSession(Arc::new_cyclic(|weak_session| {
            udata
                .screencopy_session_data()
                .session
                .set(weak_session.clone())
                .unwrap();
            match &source {
                WlCaptureSource::Cosmic(source) => {
                    let options = if options
                        .contains(ext_image_copy_capture_manager_v1::Options::PaintCursors)
                    {
                        zcosmic_screencopy_manager_v2::Options::PaintCursors
                    } else {
                        zcosmic_screencopy_manager_v2::Options::empty()
                    };
                    CaptureSessionInner::Cosmic(
                        self.0
                            .cosmic_screencopy
                            .as_ref()
                            .expect("cosmic capture source with no cosmic screencopy manager")
                            .screencopy_manager
                            .create_session(source, options, qh, udata),
                    )
                }
                WlCaptureSource::Ext(source) => CaptureSessionInner::Ext(
                    self.0
                        .image_copy_capture
                        .as_ref()
                        .expect("ext capture source with no image capture copy manager")
                        .image_copy_capture_manager
                        .create_session(source, options, qh, udata),
                ),
            }
        })))
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum CaptureSessionInner {
    Cosmic(zcosmic_screencopy_session_v2::ZcosmicScreencopySessionV2),
    Ext(ext_image_copy_capture_session_v1::ExtImageCopyCaptureSessionV1),
}

impl Drop for CaptureSessionInner {
    fn drop(&mut self) {
        match self {
            Self::Cosmic(session) => session.destroy(),
            Self::Ext(session) => session.destroy(),
        }
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
    ) -> CaptureFrame
    where
        D: 'static,
        D: Dispatch<zcosmic_screencopy_frame_v2::ZcosmicScreencopyFrameV2, U>,
        D: Dispatch<ext_image_copy_capture_frame_v1::ExtImageCopyCaptureFrameV1, U>,
        U: ScreencopyFrameDataExt + Send + Sync + 'static,
    {
        udata
            .screencopy_frame_data()
            .session
            .set(Arc::downgrade(&self.0))
            .unwrap();
        match &*self.0 {
            CaptureSessionInner::Cosmic(session) => {
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
                CaptureFrame::Cosmic(frame)
            }
            CaptureSessionInner::Ext(session) => {
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
                CaptureFrame::Ext(frame)
            }
        }
    }

    pub fn data<U: Send + Sync + 'static>(&self) -> Option<&U> {
        match &*self.0 {
            CaptureSessionInner::Cosmic(session) => session.data(),
            CaptureSessionInner::Ext(session) => session.data(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum CaptureFrame {
    Cosmic(zcosmic_screencopy_frame_v2::ZcosmicScreencopyFrameV2),
    Ext(ext_image_copy_capture_frame_v1::ExtImageCopyCaptureFrameV1),
}

impl CaptureFrame {
    pub fn session<U: ScreencopyFrameDataExt + Send + Sync + 'static>(
        &self,
    ) -> Option<CaptureSession> {
        Some(CaptureSession(
            self.data::<U>()?
                .screencopy_frame_data()
                .session
                .get()
                .unwrap()
                .upgrade()?,
        ))
    }

    pub fn data<U: Send + Sync + 'static>(&self) -> Option<&U> {
        match self {
            Self::Cosmic(frame) => frame.data(),
            Self::Ext(frame) => frame.data(),
        }
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
        D: Dispatch<ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1, GlobalData>,
        D: Dispatch<ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1, GlobalData>,
        D: Dispatch<ext_foreign_toplevel_image_capture_source_manager_v1::ExtForeignToplevelImageCaptureSourceManagerV1, GlobalData>,
    {
        let capturer = Capturer(Arc::new(CapturerInner {
            cosmic_screencopy: CosmicScreencopy::new(globals, qh),
            image_copy_capture: ImageCopyCapture::new(globals, qh),
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
        screencopy_frame: &CaptureFrame,
        frame: Frame,
    );

    fn failed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        screencopy_frame: &CaptureFrame,
        reason: WEnum<ext_image_copy_capture_frame_v1::FailureReason>,
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
    session: OnceLock<Weak<CaptureSessionInner>>,
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
        $crate::delegate_screencopy!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty,
            session: $crate::screencopy::ScreencopySessionData, frame: $crate::screencopy::ScreencopyFrameData);
    };
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty, session: [$($session_data:ty),* $(,)?], frame: [$($frame_data:ty),* $(,)?]) => {
        $crate::delegate_cosmic_screencopy!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty,
            session: [$($session_data),*], frame: [$($frame_data),*]);
        $crate::delegate_ext_image_capture!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty,
            session: [$($session_data),*], frame: [$($frame_data),*]);
    };
}
