use cosmic_protocols::{
    toplevel_info::v1::client::{zcosmic_toplevel_handle_v1, zcosmic_toplevel_info_v1},
    workspace::v1::client::zcosmic_workspace_handle_v1,
};
use sctk::registry::{ProvidesRegistryState, RegistryState};
use wayland_client::{protocol::wl_output, Connection, Dispatch, QueueHandle};

#[derive(Clone, Debug, Default)]
pub struct ToplevelInfo {
    pub title: String,
    pub app_id: String,
    pub state: Vec<u8>,
    pub output: Option<wl_output::WlOutput>,
    pub workspace: Option<zcosmic_workspace_handle_v1::ZcosmicWorkspaceHandleV1>,
}

#[derive(Debug, Default)]
struct ToplevelData {
    current_info: Option<ToplevelInfo>,
    pending_info: ToplevelInfo,
}

#[derive(Debug)]
pub struct ToplevelInfoState {
    toplevels: Vec<(
        zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1,
        ToplevelData,
    )>,
}

impl ToplevelInfoState {
    pub fn new<D>(registry: &RegistryState, qh: &QueueHandle<D>) -> Self
    where
        D: Dispatch<zcosmic_toplevel_info_v1::ZcosmicToplevelInfoV1, ()> + 'static,
    {
        registry
            .bind_one::<zcosmic_toplevel_info_v1::ZcosmicToplevelInfoV1, _, _>(qh, 1..=1, ())
            .unwrap();

        Self {
            toplevels: Vec::new(),
        }
    }

    pub fn info(
        &self,
        toplevel: &zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1,
    ) -> Option<&ToplevelInfo> {
        self.toplevels
            .iter()
            .find(|(x, _)| x == toplevel)?
            .1
            .current_info
            .as_ref()
    }

    pub fn toplevels(
        &self,
    ) -> impl Iterator<
        Item = (
            &zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1,
            Option<&ToplevelInfo>,
        ),
    > {
        self.toplevels
            .iter()
            .map(|(toplevel, data)| (toplevel, data.current_info.as_ref()))
    }
}

pub trait ToplevelInfoHandler: Sized {
    fn toplevel_info_state(&mut self) -> &mut ToplevelInfoState;

    fn new_toplevel(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        toplevel: &zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1,
    );

    fn update_toplevel(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        toplevel: &zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1,
    );

    fn toplevel_closed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        toplevel: &zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1,
    );
}

impl<D> Dispatch<zcosmic_toplevel_info_v1::ZcosmicToplevelInfoV1, (), D> for ToplevelInfoState
where
    D: Dispatch<zcosmic_toplevel_info_v1::ZcosmicToplevelInfoV1, ()>
        + Dispatch<zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1, ()>
        + ToplevelInfoHandler
        + 'static,
{
    fn event(
        state: &mut D,
        _: &zcosmic_toplevel_info_v1::ZcosmicToplevelInfoV1,
        event: zcosmic_toplevel_info_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        match event {
            zcosmic_toplevel_info_v1::Event::Toplevel { toplevel } => {
                state
                    .toplevel_info_state()
                    .toplevels
                    .push((toplevel, ToplevelData::default()));
            }
            zcosmic_toplevel_info_v1::Event::Finished => {}
            _ => unreachable!(),
        }
    }

    wayland_client::event_created_child!(D, zcosmic_toplevel_info_v1::ZcosmicToplevelInfoV1, [
        zcosmic_toplevel_info_v1::EVT_TOPLEVEL_OPCODE => (zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1, ())
    ]);
}

impl<D> Dispatch<zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1, (), D> for ToplevelInfoState
where
    D: Dispatch<zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1, ()>
        + ToplevelInfoHandler
        + 'static,
{
    fn event(
        state: &mut D,
        toplevel: &zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1,
        event: zcosmic_toplevel_handle_v1::Event,
        _: &(),
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        use wayland_client::Proxy;
        let data = &mut state
            .toplevel_info_state()
            .toplevels
            .iter_mut()
            .find(|(x, _)| x == toplevel)
            .expect("Received event for dead toplevel")
            .1;
        match event {
            zcosmic_toplevel_handle_v1::Event::AppId { app_id } => {
                data.pending_info.app_id = app_id;
            }
            zcosmic_toplevel_handle_v1::Event::Title { title } => {
                data.pending_info.title = title;
            }
            zcosmic_toplevel_handle_v1::Event::OutputEnter { output } => {
                data.pending_info.output = Some(output);
            }
            zcosmic_toplevel_handle_v1::Event::OutputLeave { output: _ } => {
                data.pending_info.output = None;
            }
            zcosmic_toplevel_handle_v1::Event::WorkspaceEnter { workspace } => {
                data.pending_info.workspace = Some(workspace);
            }
            zcosmic_toplevel_handle_v1::Event::WorkspaceLeave { workspace: _ } => {
                data.pending_info.workspace = None;
            }
            zcosmic_toplevel_handle_v1::Event::State { state } => {
                data.pending_info.state = state;
            }
            zcosmic_toplevel_handle_v1::Event::Done => {
                let is_new = data.current_info.is_none();
                data.current_info = Some(data.pending_info.clone());
                if is_new {
                    state.new_toplevel(conn, qh, toplevel);
                } else {
                    state.update_toplevel(conn, qh, toplevel);
                }
            }
            zcosmic_toplevel_handle_v1::Event::Closed => {
                state.toplevel_closed(conn, qh, toplevel);

                let toplevels = &mut state.toplevel_info_state().toplevels;
                if let Some(idx) = toplevels.iter().position(|(handle, _)| handle == toplevel) {
                    toplevels.remove(idx);
                }
            }
            _ => unreachable!(),
        }
    }
}

#[macro_export]
macro_rules! delegate_toplevel_info {
    ($ty: ty) => {
        $crate::wayland_client::delegate_dispatch!($ty: [
            $crate::cosmic_protocols::toplevel_info::v1::client::zcosmic_toplevel_info_v1::ZcosmicToplevelInfoV1: ()
        ] => $crate::toplevel_info::ToplevelInfoState);
        $crate::wayland_client::delegate_dispatch!($ty: [
            $crate::cosmic_protocols::toplevel_info::v1::client::zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1: ()
        ] => $crate::toplevel_info::ToplevelInfoState);
    };
}
