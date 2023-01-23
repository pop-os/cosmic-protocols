use cosmic_protocols::screencopy::v1::client::{
    zcosmic_screencopy_manager_v1, zcosmic_screencopy_session_v1,
};
use std::{sync::Mutex, time::Duration};
use wayland_client::{
    backend::ObjectId, globals::GlobalList, Connection, Dispatch, Proxy, QueueHandle, WEnum,
};

struct Frame {
    // transform
    // damage
    commit_time: Duration, // XXX monotonic? Is this used elsewhere in wayland?
}

struct CursorInfo {}

#[derive(Clone, Debug)]
pub struct BufferInfo {
    pub type_: WEnum<zcosmic_screencopy_session_v1::BufferType>,
    pub node: Option<String>,
    pub format: u32,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
}

#[derive(Debug)]
pub struct ScreencopyState {
    pub screencopy_manager: zcosmic_screencopy_manager_v1::ZcosmicScreencopyManagerV1, // XXX pub
    supported_cursor_modes: Vec<zcosmic_screencopy_manager_v1::CursorMode>,
}

impl ScreencopyState {
    pub fn new<D>(globals: &GlobalList, qh: &QueueHandle<D>) -> Self
    where
        D: Dispatch<zcosmic_screencopy_manager_v1::ZcosmicScreencopyManagerV1, ()> + 'static,
    {
        // TODO bind
        let screencopy_manager = globals.bind(qh, 1..=1, ()).unwrap(); // XXX
        Self {
            screencopy_manager,
            supported_cursor_modes: Vec::new(),
        }
    }
}

pub trait ScreencopyHandler: Sized {
    fn screencopy_state(&mut self) -> &mut ScreencopyState;

    fn init_done(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        session: &zcosmic_screencopy_session_v1::ZcosmicScreencopySessionV1,
        buffer_infos: &[BufferInfo],
    );

    // needs to take transform, damage, cursor_{enter, leave, info}
    // I assume commit_time is also before ready?
    fn ready(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        session: &zcosmic_screencopy_session_v1::ZcosmicScreencopySessionV1,
    );

    fn failed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        session: &zcosmic_screencopy_session_v1::ZcosmicScreencopySessionV1,
        reason: WEnum<zcosmic_screencopy_session_v1::FailureReason>,
    );
}

pub trait ScreencopySessionDataExt {
    fn screencopy_session_data(&self) -> &ScreencopySessionData;
}

#[derive(Default)]
pub struct ScreencopySessionData {
    buffer_infos: Mutex<Vec<BufferInfo>>,
    // damage, transform
}

impl ScreencopySessionDataExt for ScreencopySessionData {
    fn screencopy_session_data(&self) -> &ScreencopySessionData {
        self
    }
}

impl<D> Dispatch<zcosmic_screencopy_manager_v1::ZcosmicScreencopyManagerV1, (), D>
    for ScreencopyState
where
    D: Dispatch<zcosmic_screencopy_manager_v1::ZcosmicScreencopyManagerV1, ()> + ScreencopyHandler,
{
    fn event(
        state: &mut D,
        _: &zcosmic_screencopy_manager_v1::ZcosmicScreencopyManagerV1,
        event: zcosmic_screencopy_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        match event {
            zcosmic_screencopy_manager_v1::Event::SupportedCursorMode { mode } => {
                if let WEnum::Value(mode) = mode {
                    state.screencopy_state().supported_cursor_modes.push(mode);
                }
            }
            _ => unreachable!(),
        }
    }
}

impl<D, U> Dispatch<zcosmic_screencopy_session_v1::ZcosmicScreencopySessionV1, U, D>
    for ScreencopyState
where
    D: Dispatch<zcosmic_screencopy_session_v1::ZcosmicScreencopySessionV1, U> + ScreencopyHandler,
    U: ScreencopySessionDataExt,
{
    fn event(
        app_data: &mut D,
        session: &zcosmic_screencopy_session_v1::ZcosmicScreencopySessionV1,
        event: zcosmic_screencopy_session_v1::Event,
        udata: &U,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        let data = udata.screencopy_session_data();
        match event {
            zcosmic_screencopy_session_v1::Event::BufferInfo {
                _type,
                node,
                format,
                width,
                height,
                stride,
            } => {
                let info = BufferInfo {
                    type_: _type,
                    node,
                    format,
                    width,
                    height,
                    stride,
                };
                data.buffer_infos.lock().unwrap().push(info);
            }

            zcosmic_screencopy_session_v1::Event::InitDone => {
                let buffer_infos = data.buffer_infos.lock().unwrap();
                //println!("{:?}", &buffer_infos);
                app_data.init_done(conn, qh, session, &*buffer_infos);
            }

            zcosmic_screencopy_session_v1::Event::Transform { transform } => {}

            zcosmic_screencopy_session_v1::Event::Damage {
                x,
                y,
                width,
                height,
            } => {}

            zcosmic_screencopy_session_v1::Event::CursorEnter { seat, input_type } => {}

            zcosmic_screencopy_session_v1::Event::CursorLeave { seat, input_type } => {}

            zcosmic_screencopy_session_v1::Event::CursorInfo {
                seat,
                input_type,
                position_x,
                position_y,
                width,
                height,
                hotspot_x,
                hotspot_y,
            } => {}

            zcosmic_screencopy_session_v1::Event::Failed { reason } => {
                app_data.failed(conn, qh, session, reason);
            }

            zcosmic_screencopy_session_v1::Event::CommitTime {
                tv_sec_hi,
                tv_sec_lo,
                tv_nsec,
            } => {
                let secs = (u64::from(tv_sec_hi) << 32) + u64::from(tv_sec_lo);
                let duration = Duration::new(secs, tv_nsec);
                // TODO
            }

            zcosmic_screencopy_session_v1::Event::Ready => {
                app_data.ready(conn, qh, session); // pass other info?
            }

            _ => unreachable!(),
        }
    }
}

// Type representing screencopy session? How to handle events?

#[macro_export]
macro_rules! delegate_screencopy {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::delegate_screencopy($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty,
            session: $crate::screencopy::ScreencopySessionData);
    };
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty, session: [$($session_data:ty),* $(,)?]) => {
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::cosmic_protocols::screencopy::v1::client::zcosmic_screencopy_manager_v1::ZcosmicScreencopyManagerV1: ()
        ] => $crate::screencopy::ScreencopyState);
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $(
                $crate::cosmic_protocols::screencopy::v1::client::zcosmic_screencopy_session_v1::ZcosmicScreencopySessionV1: $session_data
            ),*
        ] => $crate::screencopy::ScreencopyState);
    };
}
