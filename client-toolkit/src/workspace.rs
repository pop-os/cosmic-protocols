use cosmic_protocols::workspace::v1::client::{
    zcosmic_workspace_group_handle_v1, zcosmic_workspace_handle_v1, zcosmic_workspace_manager_v1,
};
use sctk::registry::{GlobalProxy, RegistryState};
use wayland_client::{protocol::wl_output, Connection, Dispatch, QueueHandle};

#[derive(Default, Clone, Debug)]
pub struct WorkspaceGroup {
    pub capabilities: Vec<u8>,
    pub output: Option<wl_output::WlOutput>,
    pub workspaces: Vec<(
        zcosmic_workspace_handle_v1::ZcosmicWorkspaceHandleV1,
        Workspace,
    )>,
}

#[derive(Default, Clone, Debug)]
pub struct Workspace {
    pub name: Option<String>,
    pub coordinates: Vec<u8>,
    pub state: Vec<u8>,
    pub capabilities: Vec<u8>,
}

#[derive(Debug)]
pub struct WorkspaceState {
    workspace_groups: Vec<(
        zcosmic_workspace_group_handle_v1::ZcosmicWorkspaceGroupHandleV1,
        WorkspaceGroup,
    )>,
    _manager: GlobalProxy<zcosmic_workspace_manager_v1::ZcosmicWorkspaceManagerV1>,
}

impl WorkspaceState {
    pub fn new<D>(registry: &RegistryState, qh: &QueueHandle<D>) -> Self
    where
        D: Dispatch<zcosmic_workspace_manager_v1::ZcosmicWorkspaceManagerV1, ()> + 'static,
    {
        Self {
            workspace_groups: Vec::new(),
            _manager: GlobalProxy::from(registry.bind_one(qh, 1..=1, ())),
        }
    }

    pub fn workspace_groups(
        &self,
    ) -> &[(
        zcosmic_workspace_group_handle_v1::ZcosmicWorkspaceGroupHandleV1,
        WorkspaceGroup,
    )] {
        &self.workspace_groups
    }
}

pub trait WorkspaceHandler {
    fn workspace_state(&mut self) -> &mut WorkspaceState;

    // TODO: Added/remove/update methods? How to do that with groups and workspaces?
    fn done(&mut self);
}

impl<D> Dispatch<zcosmic_workspace_manager_v1::ZcosmicWorkspaceManagerV1, (), D> for WorkspaceState
where
    D: Dispatch<zcosmic_workspace_manager_v1::ZcosmicWorkspaceManagerV1, ()>
        + Dispatch<zcosmic_workspace_group_handle_v1::ZcosmicWorkspaceGroupHandleV1, ()>
        + WorkspaceHandler
        + 'static,
{
    fn event(
        state: &mut D,
        _: &zcosmic_workspace_manager_v1::ZcosmicWorkspaceManagerV1,
        event: zcosmic_workspace_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        match event {
            zcosmic_workspace_manager_v1::Event::WorkspaceGroup { workspace_group } => {
                state
                    .workspace_state()
                    .workspace_groups
                    .push((workspace_group, WorkspaceGroup::default()));
            }
            zcosmic_workspace_manager_v1::Event::Done => {
                state.done();
            }
            zcosmic_workspace_manager_v1::Event::Finished => {}
            _ => unreachable!(),
        }
    }

    wayland_client::event_created_child!(D, zcosmic_workspace_manager_v1::ZcosmicWorkspaceManagerV1, [
        zcosmic_workspace_manager_v1::EVT_WORKSPACE_GROUP_OPCODE => (zcosmic_workspace_group_handle_v1::ZcosmicWorkspaceGroupHandleV1, ())
    ]);
}

impl<D> Dispatch<zcosmic_workspace_group_handle_v1::ZcosmicWorkspaceGroupHandleV1, (), D>
    for WorkspaceState
where
    D: Dispatch<zcosmic_workspace_group_handle_v1::ZcosmicWorkspaceGroupHandleV1, ()>
        + Dispatch<zcosmic_workspace_handle_v1::ZcosmicWorkspaceHandleV1, ()>
        + WorkspaceHandler
        + 'static,
{
    fn event(
        state: &mut D,
        workspace_group: &zcosmic_workspace_group_handle_v1::ZcosmicWorkspaceGroupHandleV1,
        event: zcosmic_workspace_group_handle_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        let mut group_info = &mut state
            .workspace_state()
            .workspace_groups
            .iter_mut()
            .find(|(group, _)| group == workspace_group)
            .unwrap()
            .1;
        match event {
            zcosmic_workspace_group_handle_v1::Event::Capabilities { capabilities } => {
                group_info.capabilities = capabilities;
            }
            zcosmic_workspace_group_handle_v1::Event::OutputEnter { output } => {
                group_info.output = Some(output);
            }
            zcosmic_workspace_group_handle_v1::Event::OutputLeave { output } => {
                group_info.output = None;
            }
            zcosmic_workspace_group_handle_v1::Event::Workspace { workspace } => {
                group_info
                    .workspaces
                    .push((workspace, Workspace::default()));
            }
            zcosmic_workspace_group_handle_v1::Event::Remove => {
                if let Some(idx) = state
                    .workspace_state()
                    .workspace_groups
                    .iter()
                    .position(|(group, _)| group == workspace_group)
                {
                    state.workspace_state().workspace_groups.remove(idx);
                }
            }
            _ => unreachable!(),
        }
    }

    wayland_client::event_created_child!(D, zcosmic_workspace_group_handle_v1::ZcosmicWorkspaceGroupHandleV1, [
        zcosmic_workspace_group_handle_v1::EVT_WORKSPACE_OPCODE => (zcosmic_workspace_handle_v1::ZcosmicWorkspaceHandleV1, ())
    ]);
}

impl<D> Dispatch<zcosmic_workspace_handle_v1::ZcosmicWorkspaceHandleV1, (), D> for WorkspaceState
where
    D: Dispatch<zcosmic_workspace_handle_v1::ZcosmicWorkspaceHandleV1, ()> + WorkspaceHandler,
{
    fn event(
        state: &mut D,
        workspace: &zcosmic_workspace_handle_v1::ZcosmicWorkspaceHandleV1,
        event: zcosmic_workspace_handle_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        let (_, workspace_info) = state
            .workspace_state()
            .workspace_groups
            .iter_mut()
            .find_map(|(_, group_info)| {
                group_info
                    .workspaces
                    .iter_mut()
                    .find(|(w, _)| w == workspace)
            })
            .unwrap();
        match event {
            zcosmic_workspace_handle_v1::Event::Name { name } => {
                workspace_info.name = Some(name);
            }
            zcosmic_workspace_handle_v1::Event::Coordinates { coordinates } => {
                workspace_info.coordinates = coordinates;
            }
            zcosmic_workspace_handle_v1::Event::State { state } => {
                workspace_info.state = state;
            }
            zcosmic_workspace_handle_v1::Event::Capabilities { capabilities } => {
                workspace_info.capabilities = capabilities;
            }
            zcosmic_workspace_handle_v1::Event::Remove => {
                for (_, group_info) in state.workspace_state().workspace_groups.iter_mut() {
                    if let Some(idx) = group_info
                        .workspaces
                        .iter()
                        .position(|(w, _)| w == workspace)
                    {
                        group_info.workspaces.remove(idx);
                    }
                }
            }
            _ => unreachable!(),
        }
    }
}

#[macro_export]
macro_rules! delegate_workspace {
    ($ty: ty) => {
        $crate::wayland_client::delegate_dispatch!($ty: [
            $crate::cosmic_protocols::workspace::v1::client::zcosmic_workspace_manager_v1::ZcosmicWorkspaceManagerV1: ()
        ] => $crate::workspace::WorkspaceState);
        $crate::wayland_client::delegate_dispatch!($ty: [
            $crate::cosmic_protocols::workspace::v1::client::zcosmic_workspace_group_handle_v1::ZcosmicWorkspaceGroupHandleV1: ()
        ] => $crate::workspace::WorkspaceState);
        $crate::wayland_client::delegate_dispatch!($ty: [
            $crate::cosmic_protocols::workspace::v1::client::zcosmic_workspace_handle_v1::ZcosmicWorkspaceHandleV1: ()
        ] => $crate::workspace::WorkspaceState);
    };
}
