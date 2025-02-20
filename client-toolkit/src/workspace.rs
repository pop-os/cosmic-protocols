use cosmic_protocols::workspace::v2::client::{
    zcosmic_workspace_handle_v2, zcosmic_workspace_manager_v2,
};
use sctk::registry::{GlobalProxy, RegistryState};
use std::collections::HashSet;
use wayland_client::{protocol::wl_output, Connection, Dispatch, QueueHandle, WEnum};
use wayland_protocols::ext::workspace::v1::client::{
    ext_workspace_group_handle_v1, ext_workspace_handle_v1, ext_workspace_manager_v1,
};

use crate::GlobalData;

#[derive(Clone, Debug)]
pub struct WorkspaceGroup {
    pub handle: ext_workspace_group_handle_v1::ExtWorkspaceGroupHandleV1,
    pub capabilities: WEnum<ext_workspace_group_handle_v1::GroupCapabilities>,
    pub outputs: Vec<wl_output::WlOutput>,
    pub workspaces: HashSet<ext_workspace_handle_v1::ExtWorkspaceHandleV1>,
}

#[derive(Clone, Debug)]
pub struct Workspace {
    pub handle: ext_workspace_handle_v1::ExtWorkspaceHandleV1,
    pub cosmic_handle: Option<zcosmic_workspace_handle_v2::ZcosmicWorkspaceHandleV2>,
    pub name: String,
    pub coordinates: Vec<u32>,
    pub state: WEnum<ext_workspace_handle_v1::State>,
    pub capabilities: WEnum<ext_workspace_handle_v1::WorkspaceCapabilities>,
    pub cosmic_capabilities: WEnum<zcosmic_workspace_handle_v2::WorkspaceCapabilities>,
    pub tiling: Option<WEnum<zcosmic_workspace_handle_v2::TilingState>>,
}

#[derive(Debug)]
pub struct WorkspaceState {
    workspace_groups: Vec<WorkspaceGroup>,
    workspaces: Vec<Workspace>,
    manager: GlobalProxy<ext_workspace_manager_v1::ExtWorkspaceManagerV1>,
    cosmic_manager: GlobalProxy<zcosmic_workspace_manager_v2::ZcosmicWorkspaceManagerV2>,
}

impl WorkspaceState {
    pub fn new<D>(registry: &RegistryState, qh: &QueueHandle<D>) -> Self
    where
        D: Dispatch<ext_workspace_manager_v1::ExtWorkspaceManagerV1, GlobalData>
            + Dispatch<zcosmic_workspace_manager_v2::ZcosmicWorkspaceManagerV2, GlobalData>
            + 'static,
    {
        Self {
            workspace_groups: Vec::new(),
            workspaces: Vec::new(),
            manager: GlobalProxy::from(registry.bind_one(qh, 1..=1, GlobalData)),
            cosmic_manager: GlobalProxy::from(registry.bind_one(qh, 1..=1, GlobalData)),
        }
    }

    pub fn workspace_manager(
        &self,
    ) -> &GlobalProxy<ext_workspace_manager_v1::ExtWorkspaceManagerV1> {
        &self.manager
    }

    pub fn workspace_groups(&self) -> &[WorkspaceGroup] {
        &self.workspace_groups
    }

    pub fn workspaces(&self) -> &[Workspace] {
        &self.workspaces
    }
}

pub trait WorkspaceHandler {
    fn workspace_state(&mut self) -> &mut WorkspaceState;

    // TODO: Added/remove/update methods? How to do that with groups and workspaces?
    fn done(&mut self);
}

impl<D> Dispatch<ext_workspace_manager_v1::ExtWorkspaceManagerV1, GlobalData, D> for WorkspaceState
where
    D: Dispatch<ext_workspace_manager_v1::ExtWorkspaceManagerV1, GlobalData>
        + Dispatch<ext_workspace_group_handle_v1::ExtWorkspaceGroupHandleV1, GlobalData>
        + Dispatch<ext_workspace_handle_v1::ExtWorkspaceHandleV1, GlobalData>
        + Dispatch<zcosmic_workspace_handle_v2::ZcosmicWorkspaceHandleV2, GlobalData>
        + WorkspaceHandler
        + 'static,
{
    fn event(
        state: &mut D,
        _: &ext_workspace_manager_v1::ExtWorkspaceManagerV1,
        event: ext_workspace_manager_v1::Event,
        _: &GlobalData,
        _: &Connection,
        qh: &QueueHandle<D>,
    ) {
        match event {
            ext_workspace_manager_v1::Event::WorkspaceGroup { workspace_group } => {
                state
                    .workspace_state()
                    .workspace_groups
                    .push(WorkspaceGroup {
                        handle: workspace_group,
                        capabilities: WEnum::Value(
                            ext_workspace_group_handle_v1::GroupCapabilities::empty(),
                        ),
                        outputs: Vec::new(),
                        workspaces: HashSet::new(),
                    });
            }
            ext_workspace_manager_v1::Event::Workspace { workspace } => {
                let cosmic_handle =
                    state
                        .workspace_state()
                        .cosmic_manager
                        .get()
                        .ok()
                        .map(|cosmic_manager| {
                            cosmic_manager.get_cosmic_workspace(&workspace, qh, GlobalData)
                        });
                state.workspace_state().workspaces.push(Workspace {
                    handle: workspace,
                    cosmic_handle,
                    name: String::new(),
                    coordinates: Vec::new(),
                    state: WEnum::Value(ext_workspace_handle_v1::State::empty()),
                    capabilities: WEnum::Value(
                        ext_workspace_handle_v1::WorkspaceCapabilities::empty(),
                    ),
                    cosmic_capabilities: WEnum::Value(
                        zcosmic_workspace_handle_v2::WorkspaceCapabilities::empty(),
                    ),
                    tiling: None,
                });
            }
            ext_workspace_manager_v1::Event::Done => {
                state.done();
            }
            ext_workspace_manager_v1::Event::Finished => {}
            _ => unreachable!(),
        }
    }

    wayland_client::event_created_child!(D, ext_workspace_manager_v1::ExtWorkspaceManagerV1, [
        ext_workspace_manager_v1::EVT_WORKSPACE_GROUP_OPCODE => (ext_workspace_group_handle_v1::ExtWorkspaceGroupHandleV1, GlobalData),
        ext_workspace_manager_v1::EVT_WORKSPACE_OPCODE => (ext_workspace_handle_v1::ExtWorkspaceHandleV1, GlobalData)
    ]);
}

impl<D> Dispatch<ext_workspace_group_handle_v1::ExtWorkspaceGroupHandleV1, GlobalData, D>
    for WorkspaceState
where
    D: Dispatch<ext_workspace_group_handle_v1::ExtWorkspaceGroupHandleV1, GlobalData>
        + Dispatch<ext_workspace_handle_v1::ExtWorkspaceHandleV1, GlobalData>
        + WorkspaceHandler
        + 'static,
{
    fn event(
        state: &mut D,
        handle: &ext_workspace_group_handle_v1::ExtWorkspaceGroupHandleV1,
        event: ext_workspace_group_handle_v1::Event,
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
            ext_workspace_group_handle_v1::Event::Capabilities { capabilities } => {
                group.capabilities = capabilities;
            }
            ext_workspace_group_handle_v1::Event::OutputEnter { output } => {
                group.outputs.push(output);
            }
            ext_workspace_group_handle_v1::Event::OutputLeave { output } => {
                if let Some(idx) = group.outputs.iter().position(|x| x == &output) {
                    group.outputs.remove(idx);
                }
            }
            ext_workspace_group_handle_v1::Event::WorkspaceEnter { workspace } => {
                group.workspaces.insert(workspace);
            }
            ext_workspace_group_handle_v1::Event::WorkspaceLeave { workspace } => {
                group.workspaces.remove(&workspace);
            }
            ext_workspace_group_handle_v1::Event::Removed => {
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
}

impl<D> Dispatch<ext_workspace_handle_v1::ExtWorkspaceHandleV1, GlobalData, D> for WorkspaceState
where
    D: Dispatch<ext_workspace_handle_v1::ExtWorkspaceHandleV1, GlobalData> + WorkspaceHandler,
{
    fn event(
        state: &mut D,
        handle: &ext_workspace_handle_v1::ExtWorkspaceHandleV1,
        event: ext_workspace_handle_v1::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        let workspace = state
            .workspace_state()
            .workspaces
            .iter_mut()
            .find(|w| &w.handle == handle)
            .unwrap();
        match event {
            ext_workspace_handle_v1::Event::Name { name } => {
                workspace.name = name;
            }
            ext_workspace_handle_v1::Event::Coordinates { coordinates } => {
                workspace.coordinates = coordinates
                    .chunks(4)
                    .map(|chunk| u32::from_ne_bytes(chunk.try_into().unwrap()))
                    .collect();
            }
            ext_workspace_handle_v1::Event::State { state } => {
                workspace.state = state;
            }
            ext_workspace_handle_v1::Event::Capabilities { capabilities } => {
                workspace.capabilities = capabilities;
            }
            ext_workspace_handle_v1::Event::Removed => {
                for group in state.workspace_state().workspace_groups.iter_mut() {
                    group.workspaces.remove(handle);
                }
                if let Some(idx) = state
                    .workspace_state()
                    .workspaces
                    .iter()
                    .position(|w| &w.handle == handle)
                {
                    state.workspace_state().workspaces.remove(idx);
                }
            }
            _ => unreachable!(),
        }
    }
}

impl<D> Dispatch<zcosmic_workspace_manager_v2::ZcosmicWorkspaceManagerV2, GlobalData, D>
    for WorkspaceState
where
    D: Dispatch<zcosmic_workspace_manager_v2::ZcosmicWorkspaceManagerV2, GlobalData>
        + WorkspaceHandler
        + 'static,
{
    fn event(
        _: &mut D,
        _: &zcosmic_workspace_manager_v2::ZcosmicWorkspaceManagerV2,
        _: zcosmic_workspace_manager_v2::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        unreachable!()
    }
}

impl<D> Dispatch<zcosmic_workspace_handle_v2::ZcosmicWorkspaceHandleV2, GlobalData, D>
    for WorkspaceState
where
    D: Dispatch<zcosmic_workspace_handle_v2::ZcosmicWorkspaceHandleV2, GlobalData>
        + WorkspaceHandler
        + 'static,
{
    fn event(
        state: &mut D,
        handle: &zcosmic_workspace_handle_v2::ZcosmicWorkspaceHandleV2,
        event: zcosmic_workspace_handle_v2::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        let workspace = state
            .workspace_state()
            .workspaces
            .iter_mut()
            .find(|w| w.cosmic_handle.as_ref() == Some(&handle))
            .unwrap();
        match event {
            zcosmic_workspace_handle_v2::Event::Capabilities { capabilities } => {
                workspace.cosmic_capabilities = capabilities;
            }
            zcosmic_workspace_handle_v2::Event::TilingState { state } => {
                workspace.tiling = Some(state);
            }
            _ => unreachable!(),
        }
    }
}

#[macro_export]
macro_rules! delegate_workspace {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::wayland_protocols::ext::workspace::v1::client::ext_workspace_manager_v1::ExtWorkspaceManagerV1: $crate::GlobalData
        ] => $crate::workspace::WorkspaceState);
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::wayland_protocols::ext::workspace::v1::client::ext_workspace_group_handle_v1::ExtWorkspaceGroupHandleV1: $crate::GlobalData
        ] => $crate::workspace::WorkspaceState);
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::wayland_protocols::ext::workspace::v1::client::ext_workspace_handle_v1::ExtWorkspaceHandleV1: $crate::GlobalData
        ] => $crate::workspace::WorkspaceState);

        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::cosmic_protocols::workspace::v2::client::zcosmic_workspace_manager_v2::ZcosmicWorkspaceManagerV2: $crate::GlobalData
        ] => $crate::workspace::WorkspaceState);
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::cosmic_protocols::workspace::v2::client::zcosmic_workspace_handle_v2::ZcosmicWorkspaceHandleV2: $crate::GlobalData
        ] => $crate::workspace::WorkspaceState);
    };
}
