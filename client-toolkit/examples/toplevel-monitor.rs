use cosmic_client_toolkit::toplevel_info::{ToplevelInfoHandler, ToplevelInfoState};
use cosmic_protocols::toplevel_info::v1::client::zcosmic_toplevel_handle_v1;
use sctk::registry::{ProvidesRegistryState, RegistryState};
use wayland_client::{globals::registry_queue_init, Connection, QueueHandle};

struct AppData {
    registry_state: RegistryState,
    toplevel_info_state: ToplevelInfoState,
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
        _qh: &QueueHandle<Self>,
        toplevel: &zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1,
    ) {
        println!(
            "New toplevel: {:?}",
            self.toplevel_info_state.info(toplevel).unwrap()
        );
    }

    fn update_toplevel(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        toplevel: &zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1,
    ) {
        println!(
            "Update toplevel: {:?}",
            self.toplevel_info_state.info(toplevel).unwrap()
        );
    }

    fn toplevel_closed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        toplevel: &zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1,
    ) {
        println!(
            "Closed toplevel: {:?}",
            self.toplevel_info_state.info(toplevel).unwrap()
        );
    }
}

fn main() {
    let conn = Connection::connect_to_env().unwrap();
    let (globals, mut event_queue) = registry_queue_init(&conn).unwrap();
    let qh = event_queue.handle();

    let registry_state = RegistryState::new(&globals);
    let mut app_data = AppData {
        toplevel_info_state: ToplevelInfoState::new(&registry_state, &qh),
        registry_state,
    };

    loop {
        event_queue.blocking_dispatch(&mut app_data).unwrap();
    }
}

sctk::delegate_registry!(AppData);
cosmic_client_toolkit::delegate_toplevel_info!(AppData);
