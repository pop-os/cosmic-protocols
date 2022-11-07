use cosmic_client_toolkit::workspace::{WorkspaceHandler, WorkspaceState};
use sctk::registry::{ProvidesRegistryState, RegistryState};
use wayland_client::Connection;

struct AppData {
    registry_state: RegistryState,
    workspace_state: WorkspaceState,
}

impl ProvidesRegistryState for AppData {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    sctk::registry_handlers!(WorkspaceState,);
}

impl WorkspaceHandler for AppData {
    fn workspace_state(&mut self) -> &mut WorkspaceState {
        &mut self.workspace_state
    }

    fn done(&mut self) {
        for (_, group) in self.workspace_state.workspace_groups() {
            println!(
                "Group: capabilities: {:?}, output: {:?}",
                &group.capabilities, &group.output
            );
            for (_, workspace) in &group.workspaces {
                println!("{:?}", &workspace);
            }
        }
    }
}

fn main() {
    let connection = Connection::connect_to_env().unwrap();
    let mut event_queue = connection.new_event_queue();
    let qh = event_queue.handle();

    let mut app_data = AppData {
        registry_state: RegistryState::new(&connection, &qh),
        workspace_state: WorkspaceState::new(),
    };

    loop {
        event_queue.blocking_dispatch(&mut app_data).unwrap();
    }
}

sctk::delegate_registry!(AppData);
cosmic_client_toolkit::delegate_workspace!(AppData);
