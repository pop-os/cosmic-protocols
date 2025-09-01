use cosmic_protocols::{
    toplevel_info::v1::client::{zcosmic_toplevel_handle_v1, zcosmic_toplevel_info_v1},
    workspace::v1::client::{
        zcosmic_workspace_group_handle_v1, zcosmic_workspace_handle_v1,
        zcosmic_workspace_manager_v1,
    },
};
use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle, event_created_child,
    protocol::{wl_output, wl_registry},
};

#[derive(Default)]
struct AppData {
    toplevel_info: Option<zcosmic_toplevel_info_v1::ZcosmicToplevelInfoV1>,
    workspace_manager: Option<zcosmic_workspace_manager_v1::ZcosmicWorkspaceManagerV1>,
    outputs: Vec<(wl_output::WlOutput, String)>,
    toplevels: Vec<Toplevel>,
    workspaces: Vec<(
        zcosmic_workspace_group_handle_v1::ZcosmicWorkspaceGroupHandleV1,
        Vec<(
            zcosmic_workspace_handle_v1::ZcosmicWorkspaceHandleV1,
            Option<String>,
        )>,
    )>,
}

struct Toplevel {
    handle: zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1,
    title: Option<String>,
    app_id: Option<String>,
    outputs: Vec<wl_output::WlOutput>,
    workspaces: Vec<zcosmic_workspace_handle_v1::ZcosmicWorkspaceHandleV1>,
    state: Vec<State>,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
enum State {
    Maximized = 0,
    Minimized = 1,
    Activated = 2,
    Fullscreen = 3,
}

impl TryFrom<u32> for State {
    type Error = ();
    fn try_from(val: u32) -> Result<State, ()> {
        match val {
            0 => Ok(State::Maximized),
            1 => Ok(State::Minimized),
            2 => Ok(State::Activated),
            3 => Ok(State::Fullscreen),
            _ => Err(()),
        }
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for AppData {
    fn event(
        app_data: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<AppData>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version: _,
        } = event
        {
            match &*interface {
                "zcosmic_toplevel_info_v1" => {
                    app_data.toplevel_info = Some(
                        registry.bind::<zcosmic_toplevel_info_v1::ZcosmicToplevelInfoV1, _, _>(
                            name,
                            1,
                            qh,
                            (),
                        ),
                    );
                }
                "zcosmic_workspace_manager_v1" => {
                    app_data.workspace_manager = Some(
                        registry
                            .bind::<zcosmic_workspace_manager_v1::ZcosmicWorkspaceManagerV1, _, _>(
                                name,
                                1,
                                qh,
                                (),
                            ),
                    );
                }
                "wl_output" => {
                    registry.bind::<wl_output::WlOutput, _, _>(name, 4, qh, ());
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<wl_output::WlOutput, ()> for AppData {
    fn event(
        app_data: &mut Self,
        output: &wl_output::WlOutput,
        event: wl_output::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<AppData>,
    ) {
        match event {
            wl_output::Event::Name { name } => {
                app_data.outputs.push((output.clone(), name));
            }
            _ => {}
        }
    }
}

impl Dispatch<zcosmic_toplevel_info_v1::ZcosmicToplevelInfoV1, ()> for AppData {
    fn event(
        app_data: &mut Self,
        _info: &zcosmic_toplevel_info_v1::ZcosmicToplevelInfoV1,
        event: zcosmic_toplevel_info_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<AppData>,
    ) {
        match event {
            zcosmic_toplevel_info_v1::Event::Toplevel { toplevel } => {
                app_data.toplevels.push(Toplevel {
                    handle: toplevel,
                    title: None,
                    app_id: None,
                    outputs: Vec::new(),
                    workspaces: Vec::new(),
                    state: Vec::new(),
                })
            }
            _ => {}
        }
    }

    event_created_child!(
        AppData,
        zcosmic_toplevel_info_v1::ZcosmicToplevelInfoV1,
        [
            zcosmic_toplevel_info_v1::EVT_TOPLEVEL_OPCODE => (zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1, ()),
        ]
    );
}

impl Dispatch<zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1, ()> for AppData {
    fn event(
        app_data: &mut Self,
        toplevel: &zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1,
        event: zcosmic_toplevel_handle_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<AppData>,
    ) {
        match event {
            zcosmic_toplevel_handle_v1::Event::Title { title } => {
                if let Some(info) = app_data
                    .toplevels
                    .iter_mut()
                    .find(|t| &t.handle == toplevel)
                {
                    info.title = Some(title);
                }
            }
            zcosmic_toplevel_handle_v1::Event::AppId { app_id } => {
                if let Some(info) = app_data
                    .toplevels
                    .iter_mut()
                    .find(|t| &t.handle == toplevel)
                {
                    info.app_id = Some(app_id);
                }
            }
            zcosmic_toplevel_handle_v1::Event::OutputEnter { output } => {
                if let Some(info) = app_data
                    .toplevels
                    .iter_mut()
                    .find(|t| &t.handle == toplevel)
                {
                    info.outputs.push(output);
                }
            }
            zcosmic_toplevel_handle_v1::Event::OutputLeave { output } => {
                if let Some(info) = app_data
                    .toplevels
                    .iter_mut()
                    .find(|t| &t.handle == toplevel)
                {
                    info.outputs.retain(|o| o != &output);
                }
            }
            zcosmic_toplevel_handle_v1::Event::WorkspaceEnter { workspace } => {
                if let Some(info) = app_data
                    .toplevels
                    .iter_mut()
                    .find(|t| &t.handle == toplevel)
                {
                    info.workspaces.push(workspace);
                }
            }
            zcosmic_toplevel_handle_v1::Event::WorkspaceLeave { workspace } => {
                if let Some(info) = app_data
                    .toplevels
                    .iter_mut()
                    .find(|t| &t.handle == toplevel)
                {
                    info.workspaces.retain(|w| w != &workspace);
                }
            }
            zcosmic_toplevel_handle_v1::Event::State { state } => {
                if let Some(info) = app_data
                    .toplevels
                    .iter_mut()
                    .find(|t| &t.handle == toplevel)
                {
                    info.state = state
                        .chunks_exact(4)
                        .map(|chunk| u32::from_ne_bytes(chunk.try_into().unwrap()))
                        .flat_map(|val| State::try_from(val).ok())
                        .collect::<Vec<_>>();
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<zcosmic_workspace_manager_v1::ZcosmicWorkspaceManagerV1, ()> for AppData {
    fn event(
        app_data: &mut Self,
        _: &zcosmic_workspace_manager_v1::ZcosmicWorkspaceManagerV1,
        event: zcosmic_workspace_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<AppData>,
    ) {
        match event {
            zcosmic_workspace_manager_v1::Event::WorkspaceGroup { workspace_group } => {
                app_data.workspaces.push((workspace_group, Vec::new()));
            }
            _ => {}
        }
    }

    event_created_child!(
        AppData,
        zcosmic_workspace_manager_v1::ZcosmicWorkspaceManagerV1,
        [
            zcosmic_workspace_manager_v1::EVT_WORKSPACE_GROUP_OPCODE => (zcosmic_workspace_group_handle_v1::ZcosmicWorkspaceGroupHandleV1, ()),
        ]
    );
}

impl Dispatch<zcosmic_workspace_group_handle_v1::ZcosmicWorkspaceGroupHandleV1, ()> for AppData {
    fn event(
        app_data: &mut AppData,
        group: &zcosmic_workspace_group_handle_v1::ZcosmicWorkspaceGroupHandleV1,
        event: <zcosmic_workspace_group_handle_v1::ZcosmicWorkspaceGroupHandleV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            zcosmic_workspace_group_handle_v1::Event::Workspace { workspace } => {
                if let Some((_, spaces)) = app_data.workspaces.iter_mut().find(|(g, _)| g == group)
                {
                    spaces.push((workspace, None));
                }
            }
            _ => {}
        }
    }

    event_created_child!(
        AppData,
        zcosmic_workspace_group_handle_v1::ZcosmicWorkspaceGroupHandleV1,
        [
            zcosmic_workspace_group_handle_v1::EVT_WORKSPACE_OPCODE => (zcosmic_workspace_handle_v1::ZcosmicWorkspaceHandleV1, ()),
        ]
    );
}

impl Dispatch<zcosmic_workspace_handle_v1::ZcosmicWorkspaceHandleV1, ()> for AppData {
    fn event(
        app_data: &mut AppData,
        workspace: &zcosmic_workspace_handle_v1::ZcosmicWorkspaceHandleV1,
        event: <zcosmic_workspace_handle_v1::ZcosmicWorkspaceHandleV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            zcosmic_workspace_handle_v1::Event::Name { name } => {
                if let Some((_, n)) = app_data
                    .workspaces
                    .iter_mut()
                    .flat_map(|(_, s)| s.iter_mut())
                    .find(|(w, _)| w == workspace)
                {
                    *n = Some(name);
                }
            }
            _ => {}
        }
    }
}

fn main() {
    let conn = Connection::connect_to_env().unwrap();
    let display = conn.display();
    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();
    let _registry = display.get_registry(&qh, ());

    let mut app_data = AppData::default();
    event_queue.roundtrip(&mut app_data).unwrap();
    event_queue.roundtrip(&mut app_data).unwrap();
    event_queue.roundtrip(&mut app_data).unwrap();

    app_data
        .toplevel_info
        .as_ref()
        .expect("Toplevel_info protocol not supported");

    for toplevel in app_data.toplevels.iter() {
        println!(
            "Toplevel {:?}\n\tTitle: {:?}\n\tApp ID: {:?}\n\tStates: {:?}\n\tOutputs: {:?}\n\tWorkspaces: {:?}",
            toplevel.handle.id(),
            toplevel.title,
            toplevel.app_id,
            toplevel.state,
            toplevel
                .outputs
                .iter()
                .flat_map(|obj| app_data
                    .outputs
                    .iter()
                    .find(|(o, _)| o == obj)
                    .map(|(_, name)| name))
                .collect::<Vec<_>>(),
            toplevel
                .workspaces
                .iter()
                .flat_map(|obj| app_data
                    .workspaces
                    .iter()
                    .flat_map(|(_, w)| w.iter())
                    .find(|(o, _)| o == obj)
                    .map(|(_, name)| name))
                .collect::<Vec<_>>(),
        );
    }
}
