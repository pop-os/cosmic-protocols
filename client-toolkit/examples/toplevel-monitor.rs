use cosmic_client_toolkit::toplevel_info::{ToplevelInfoHandler, ToplevelInfoState};
use cosmic_protocols::toplevel_info::v1::client::zcosmic_toplevel_handle_v1;
use sctk::registry::{ProvidesRegistryState, RegistryState};
use wayland_client::{Connection, QueueHandle};

struct AppData {
    registry_state: RegistryState,
    toplevel_info_state: ToplevelInfoState,
}

impl ProvidesRegistryState for AppData {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    sctk::registry_handlers!(ToplevelInfoState,);
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
    let connection = Connection::connect_to_env().unwrap();
    let mut event_queue = connection.new_event_queue();
    let qh = event_queue.handle();

    let mut app_data = AppData {
        registry_state: RegistryState::new(&connection, &qh),
        toplevel_info_state: ToplevelInfoState::new(),
    };

    loop {
        event_queue.blocking_dispatch(&mut app_data).unwrap();
    }
}

sctk::delegate_registry!(AppData);
cosmic_client_toolkit::delegate_toplevel_info!(AppData);
