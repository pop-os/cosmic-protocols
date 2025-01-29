use cosmic_protocols::workspace::v1::client::{
    zcosmic_workspace_group_handle_v1, zcosmic_workspace_handle_v1, zcosmic_workspace_manager_v1,
};
use sctk::registry::{GlobalProxy, RegistryState};
use wayland_client::{protocol::wl_output, Connection, Dispatch, QueueHandle, WEnum};

use crate::GlobalData;

#[derive(Clone, Debug)]
pub struct WorkspaceGroup {
    pub handle: zcosmic_workspace_group_handle_v1::ZcosmicWorkspaceGroupHandleV1,
    pub capabilities:
        Vec<WEnum<zcosmic_workspace_group_handle_v1::ZcosmicWorkspaceGroupCapabilitiesV1>>,
    pub outputs: Vec<wl_output::WlOutput>,
    pub workspaces: Vec<Workspace>,
}

#[derive(Clone, Debug)]
pub struct Workspace {
    pub handle: zcosmic_workspace_handle_v1::ZcosmicWorkspaceHandleV1,
    pub name: String,
    pub coordinates: Vec<u32>,
    pub state: Vec<WEnum<zcosmic_workspace_handle_v1::State>>,
    pub capabilities: Vec<WEnum<zcosmic_workspace_handle_v1::ZcosmicWorkspaceCapabilitiesV1>>,
    pub tiling: Option<WEnum<zcosmic_workspace_handle_v1::TilingState>>,
}

#[derive(Debug)]
pub struct WorkspaceState {
    workspace_groups: Vec<WorkspaceGroup>,
    manager: GlobalProxy<zcosmic_workspace_manager_v1::ZcosmicWorkspaceManagerV1>,
}

impl WorkspaceState {
    pub fn new<D>(registry: &RegistryState, qh: &QueueHandle<D>) -> Self
    where
        D: Dispatch<zcosmic_workspace_manager_v1::ZcosmicWorkspaceManagerV1, GlobalData> + 'static,
    {
        Self {
            workspace_groups: Vec::new(),
            manager: GlobalProxy::from(registry.bind_one(qh, 1..=2, GlobalData)),
        }
    }

    pub fn workspace_manager(
        &self,
    ) -> &GlobalProxy<zcosmic_workspace_manager_v1::ZcosmicWorkspaceManagerV1> {
        &self.manager
    }

    pub fn workspace_groups(&self) -> &[WorkspaceGroup] {
        &self.workspace_groups
    }
}

pub trait WorkspaceHandler {
    fn workspace_state(&mut self) -> &mut WorkspaceState;

    // TODO: Added/remove/update methods? How to do that with groups and workspaces?
    fn done(&mut self);
}

impl<D> Dispatch<zcosmic_workspace_manager_v1::ZcosmicWorkspaceManagerV1, GlobalData, D>
    for WorkspaceState
where
    D: Dispatch<zcosmic_workspace_manager_v1::ZcosmicWorkspaceManagerV1, GlobalData>
        + Dispatch<zcosmic_workspace_group_handle_v1::ZcosmicWorkspaceGroupHandleV1, GlobalData>
        + WorkspaceHandler
        + 'static,
{
    fn event(
        state: &mut D,
        _: &zcosmic_workspace_manager_v1::ZcosmicWorkspaceManagerV1,
        event: zcosmic_workspace_manager_v1::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        match event {
            zcosmic_workspace_manager_v1::Event::WorkspaceGroup { workspace_group } => {
                state
                    .workspace_state()
                    .workspace_groups
                    .push(WorkspaceGroup {
                        handle: workspace_group,
                        capabilities: Vec::new(),
                        outputs: Vec::new(),
                        workspaces: Vec::new(),
                    });
            }
            zcosmic_workspace_manager_v1::Event::Done => {
                state.done();
            }
            zcosmic_workspace_manager_v1::Event::Finished => {}
            _ => unreachable!(),
        }
    }

    wayland_client::event_created_child!(D, zcosmic_workspace_manager_v1::ZcosmicWorkspaceManagerV1, [
        zcosmic_workspace_manager_v1::EVT_WORKSPACE_GROUP_OPCODE => (zcosmic_workspace_group_handle_v1::ZcosmicWorkspaceGroupHandleV1, GlobalData)
    ]);
}

impl<D> Dispatch<zcosmic_workspace_group_handle_v1::ZcosmicWorkspaceGroupHandleV1, GlobalData, D>
    for WorkspaceState
where
    D: Dispatch<zcosmic_workspace_group_handle_v1::ZcosmicWorkspaceGroupHandleV1, GlobalData>
        + Dispatch<zcosmic_workspace_handle_v1::ZcosmicWorkspaceHandleV1, GlobalData>
        + WorkspaceHandler
        + 'static,
{
    fn event(
        state: &mut D,
        handle: &zcosmic_workspace_group_handle_v1::ZcosmicWorkspaceGroupHandleV1,
        event: zcosmic_workspace_group_handle_v1::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        let group = &mut state
            .workspace_state()
            .workspace_groups
            .iter_mut()
            .find(|group| &group.handle == handle)
            .unwrap();
        match event {
            zcosmic_workspace_group_handle_v1::Event::Capabilities { capabilities } => {
                group.capabilities = capabilities
                    .chunks(4)
                    .map(|chunk| WEnum::from(u32::from_ne_bytes(chunk.try_into().unwrap())))
                    .collect();
            }
            zcosmic_workspace_group_handle_v1::Event::OutputEnter { output } => {
                group.outputs.push(output);
            }
            zcosmic_workspace_group_handle_v1::Event::OutputLeave { output } => {
                if let Some(idx) = group.outputs.iter().position(|x| x == &output) {
                    group.outputs.remove(idx);
                }
            }
            zcosmic_workspace_group_handle_v1::Event::Workspace { workspace } => {
                group.workspaces.push(Workspace {
                    handle: workspace,
                    name: String::new(),
                    coordinates: Vec::new(),
                    state: Vec::new(),
                    capabilities: Vec::new(),
                    tiling: None,
                });
            }
            zcosmic_workspace_group_handle_v1::Event::Remove => {
                if let Some(idx) = state
                    .workspace_state()
                    .workspace_groups
                    .iter()
                    .position(|group| &group.handle == handle)
                {
                    state.workspace_state().workspace_groups.remove(idx);
                }
            }
            _ => unreachable!(),
        }
    }

    wayland_client::event_created_child!(D, zcosmic_workspace_group_handle_v1::ZcosmicWorkspaceGroupHandleV1, [
        zcosmic_workspace_group_handle_v1::EVT_WORKSPACE_OPCODE => (zcosmic_workspace_handle_v1::ZcosmicWorkspaceHandleV1, GlobalData)
    ]);
}

impl<D> Dispatch<zcosmic_workspace_handle_v1::ZcosmicWorkspaceHandleV1, GlobalData, D>
    for WorkspaceState
where
    D: Dispatch<zcosmic_workspace_handle_v1::ZcosmicWorkspaceHandleV1, GlobalData>
        + WorkspaceHandler,
{
    fn event(
        state: &mut D,
        handle: &zcosmic_workspace_handle_v1::ZcosmicWorkspaceHandleV1,
        event: zcosmic_workspace_handle_v1::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        let workspace = state
            .workspace_state()
            .workspace_groups
            .iter_mut()
            .find_map(|group| group.workspaces.iter_mut().find(|w| &w.handle == handle))
            .unwrap();
        match event {
            zcosmic_workspace_handle_v1::Event::Name { name } => {
                workspace.name = name;
            }
            zcosmic_workspace_handle_v1::Event::Coordinates { coordinates } => {
                workspace.coordinates = coordinates
                    .chunks(4)
                    .map(|chunk| u32::from_ne_bytes(chunk.try_into().unwrap()))
                    .collect();
            }
            zcosmic_workspace_handle_v1::Event::State { state } => {
                workspace.state = state
                    .chunks(4)
                    .map(|chunk| WEnum::from(u32::from_ne_bytes(chunk.try_into().unwrap())))
                    .collect();
            }
            zcosmic_workspace_handle_v1::Event::Capabilities { capabilities } => {
                workspace.capabilities = capabilities
                    .chunks(4)
                    .map(|chunk| WEnum::from(u32::from_ne_bytes(chunk.try_into().unwrap())))
                    .collect();
            }
            zcosmic_workspace_handle_v1::Event::TilingState { state } => {
                workspace.tiling = Some(state);
            }
            zcosmic_workspace_handle_v1::Event::Remove => {
                for group in state.workspace_state().workspace_groups.iter_mut() {
                    if let Some(idx) = group.workspaces.iter().position(|w| &w.handle == handle) {
                        group.workspaces.remove(idx);
                    }
                }
            }
            _ => unreachable!(),
        }
    }
}

#[macro_export]
macro_rules! delegate_workspace {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::cosmic_protocols::workspace::v1::client::zcosmic_workspace_manager_v1::ZcosmicWorkspaceManagerV1: $crate::GlobalData
        ] => $crate::workspace::WorkspaceState);
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::cosmic_protocols::workspace::v1::client::zcosmic_workspace_group_handle_v1::ZcosmicWorkspaceGroupHandleV1: $crate::GlobalData
        ] => $crate::workspace::WorkspaceState);
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::cosmic_protocols::workspace::v1::client::zcosmic_workspace_handle_v1::ZcosmicWorkspaceHandleV1: $crate::GlobalData
        ] => $crate::workspace::WorkspaceState);
    };
}
