use cosmic_client_toolkit::{
    screencopy::{BufferInfo, ScreencopyHandler, ScreencopyState},
    toplevel_info::{ToplevelInfoHandler, ToplevelInfoState},
};
use cosmic_protocols::{
    screencopy::v1::client::{zcosmic_screencopy_manager_v1, zcosmic_screencopy_session_v1},
    toplevel_info::v1::client::zcosmic_toplevel_handle_v1,
};
use sctk::{
    registry::{ProvidesRegistryState, RegistryState},
    shm::{raw::RawPool, ShmHandler, ShmState},
};
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_buffer, wl_shm},
    Connection, Dispatch, QueueHandle, WEnum,
};

struct AppData {
    registry_state: RegistryState,
    toplevel_info_state: ToplevelInfoState,
    screencopy_state: ScreencopyState,
    shm_state: ShmState,
    n_toplevels: usize,
    n_captured: usize,
    n_failed: usize,
}

impl ProvidesRegistryState for AppData {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    sctk::registry_handlers!();
}

impl ToplevelInfoHandler for AppData {
    fn toplevel_info_state(&mut self) -> &mut ToplevelInfoState {
        &mut self.toplevel_info_state
    }

    fn new_toplevel(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        toplevel: &zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1,
    ) {
        self.n_toplevels += 1;
        self.screencopy_state.screencopy_manager.capture_toplevel(
            &toplevel,
            zcosmic_screencopy_manager_v1::CursorMode::Hidden,
            &qh,
            Default::default(),
        );
    }

    fn update_toplevel(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _toplevel: &zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1,
    ) {
    }

    fn toplevel_closed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _toplevel: &zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1,
    ) {
    }
}

impl ScreencopyHandler for AppData {
    fn screencopy_state(&mut self) -> &mut ScreencopyState {
        &mut self.screencopy_state
    }

    fn init_done(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        session: &zcosmic_screencopy_session_v1::ZcosmicScreencopySessionV1,
        buffer_infos: &[BufferInfo],
    ) {
        let buffer_info = buffer_infos
            .iter()
            .find(|x| {
                x.type_ == WEnum::Value(zcosmic_screencopy_session_v1::BufferType::WlShm)
                    && x.format == wl_shm::Format::Abgr8888.into()
            })
            .unwrap();
        let buf_len = buffer_info.stride * buffer_info.height;

        let mut pool = RawPool::new(buf_len as usize, &self.shm_state).unwrap();
        let buffer = pool.create_buffer(
            0,
            buffer_info.width as i32,
            buffer_info.height as i32,
            buffer_info.stride as i32,
            wl_shm::Format::Abgr8888,
            (),
            qh,
        );

        session.attach_buffer(&buffer, None, 0); // XXX age?
        session.commit(zcosmic_screencopy_session_v1::Options::empty());
    }

    fn ready(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        session: &zcosmic_screencopy_session_v1::ZcosmicScreencopySessionV1,
    ) {
        self.n_captured += 1;
        println!("screencopy ready");
    }

    fn failed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        session: &zcosmic_screencopy_session_v1::ZcosmicScreencopySessionV1,
        reason: WEnum<zcosmic_screencopy_session_v1::FailureReason>,
    ) {
        self.n_failed += 1;
        eprintln!("Screencopy failed");
    }
}

impl ShmHandler for AppData {
    fn shm_state(&mut self) -> &mut ShmState {
        &mut self.shm_state
    }
}

impl Dispatch<wl_buffer::WlBuffer, ()> for AppData {
    fn event(
        _app_data: &mut Self,
        buffer: &wl_buffer::WlBuffer,
        event: wl_buffer::Event,
        _: &(),
        _: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

fn main() {
    let conn = Connection::connect_to_env().unwrap();
    let (globals, mut event_queue) = registry_queue_init(&conn).unwrap();
    let qh = event_queue.handle();

    let registry_state = RegistryState::new(&globals);
    let mut app_data = AppData {
        screencopy_state: ScreencopyState::new(&globals, &qh),
        toplevel_info_state: ToplevelInfoState::new(&registry_state, &qh),
        shm_state: ShmState::bind(&globals, &qh).unwrap(),
        registry_state,
        n_toplevels: 0,
        n_captured: 0,
        n_failed: 0,
    };

    event_queue.roundtrip(&mut app_data).unwrap();
    while app_data.n_toplevels > app_data.n_captured + app_data.n_failed {
        event_queue.blocking_dispatch(&mut app_data).unwrap();
    }

    println!("{} successful captures", app_data.n_captured);
    println!("{} failed captures", app_data.n_failed);
}

sctk::delegate_registry!(AppData);
sctk::delegate_shm!(AppData);
cosmic_client_toolkit::delegate_toplevel_info!(AppData);
cosmic_client_toolkit::delegate_screencopy!(AppData);
