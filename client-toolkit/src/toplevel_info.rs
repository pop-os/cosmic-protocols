use std::collections::{HashMap, HashSet};

use cosmic_protocols::{
    toplevel_info::v1::client::{zcosmic_toplevel_handle_v1, zcosmic_toplevel_info_v1},
    workspace::v1::client::zcosmic_workspace_handle_v1,
};
use sctk::registry::RegistryState;
use wayland_client::{protocol::wl_output, Connection, Dispatch, QueueHandle};
use wayland_protocols::ext::foreign_toplevel_list::v1::client::{
    ext_foreign_toplevel_handle_v1, ext_foreign_toplevel_list_v1,
};

use crate::GlobalData;

#[derive(Clone, Debug, Default)]
pub struct ToplevelGeometry {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Clone, Debug)]
pub struct ToplevelInfo {
    pub title: String,
    pub app_id: String,
    pub identifier: Option<String>,
    pub state: HashSet<zcosmic_toplevel_handle_v1::State>,
    pub output: HashSet<wl_output::WlOutput>,
    pub geometry: HashMap<wl_output::WlOutput, ToplevelGeometry>,
    pub workspace: HashSet<zcosmic_workspace_handle_v1::ZcosmicWorkspaceHandleV1>,
    pub foreign_toplevel: ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
}

#[derive(Debug)]
struct ToplevelData {
    current_info: Option<ToplevelInfo>,
    pending_info: ToplevelInfo,
    has_cosmic_info: bool,
}

impl ToplevelData {
    fn new(foreign_toplevel: ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1) -> Self {
        let pending_info = ToplevelInfo {
            title: String::new(),
            app_id: String::new(),
            identifier: None,
            state: HashSet::new(),
            output: HashSet::new(),
            geometry: HashMap::new(),
            workspace: HashSet::new(),
            foreign_toplevel,
        };
        Self {
            current_info: None,
            pending_info,
            has_cosmic_info: false,
        }
    }
}

#[derive(Debug)]
pub struct ToplevelInfoState {
    pub foreign_toplevel_list: ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1,
    pub cosmic_toplevel_info: zcosmic_toplevel_info_v1::ZcosmicToplevelInfoV1,
    toplevels: Vec<(
        zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1,
        ToplevelData,
    )>,
}

impl ToplevelInfoState {
    pub fn try_new<D>(registry: &RegistryState, qh: &QueueHandle<D>) -> Option<Self>
    where
        D: Dispatch<zcosmic_toplevel_info_v1::ZcosmicToplevelInfoV1, GlobalData>
            + Dispatch<ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1, GlobalData>
            + 'static,
    {
        let foreign_toplevel_list = registry
            .bind_one::<ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1, _, _>(
                qh,
                1..=1,
                GlobalData,
            )
            .ok()?;
        let cosmic_toplevel_info = registry
            .bind_one::<zcosmic_toplevel_info_v1::ZcosmicToplevelInfoV1, _, _>(
                qh,
                2..=2,
                GlobalData,
            )
            .ok()?;

        Some(Self {
            foreign_toplevel_list,
            cosmic_toplevel_info,
            toplevels: Vec::new(),
        })
    }

    pub fn new<D>(registry: &RegistryState, qh: &QueueHandle<D>) -> Self
    where
        D: Dispatch<zcosmic_toplevel_info_v1::ZcosmicToplevelInfoV1, GlobalData>
            + Dispatch<ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1, GlobalData>
            + 'static,
    {
        Self::try_new(registry, qh).unwrap()
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

    fn info_done(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>) {}

    fn finished(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>) {}
}

impl<D> Dispatch<zcosmic_toplevel_info_v1::ZcosmicToplevelInfoV1, GlobalData, D>
    for ToplevelInfoState
where
    D: Dispatch<zcosmic_toplevel_info_v1::ZcosmicToplevelInfoV1, GlobalData>
        + Dispatch<zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1, GlobalData>
        + ToplevelInfoHandler
        + 'static,
{
    fn event(
        state: &mut D,
        _proxy: &zcosmic_toplevel_info_v1::ZcosmicToplevelInfoV1,
        event: zcosmic_toplevel_info_v1::Event,
        _: &GlobalData,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        match event {
            zcosmic_toplevel_info_v1::Event::Done => {
                state.info_done(conn, qh);
            }
            // Not used in protocol version 2
            zcosmic_toplevel_info_v1::Event::Toplevel { .. }
            | zcosmic_toplevel_info_v1::Event::Finished => {}
            _ => unreachable!(),
        }
    }

    wayland_client::event_created_child!(D, zcosmic_toplevel_info_v1::ZcosmicToplevelInfoV1, [
        zcosmic_toplevel_info_v1::EVT_TOPLEVEL_OPCODE => (zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1, GlobalData)
    ]);
}

impl<D> Dispatch<zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1, GlobalData, D>
    for ToplevelInfoState
where
    D: Dispatch<zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1, GlobalData>
        + ToplevelInfoHandler
        + 'static,
{
    fn event(
        state: &mut D,
        toplevel: &zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1,
        event: zcosmic_toplevel_handle_v1::Event,
        _: &GlobalData,
        _conn: &Connection,
        _qh: &QueueHandle<D>,
    ) {
        let data = &mut state
            .toplevel_info_state()
            .toplevels
            .iter_mut()
            .find(|(x, _)| x == toplevel)
            .expect("Received event for dead toplevel")
            .1;
        match event {
            zcosmic_toplevel_handle_v1::Event::OutputEnter { output } => {
                data.pending_info.output.insert(output);
            }
            zcosmic_toplevel_handle_v1::Event::OutputLeave { output } => {
                data.pending_info.output.remove(&output);
                data.pending_info.geometry.remove(&output);
            }
            zcosmic_toplevel_handle_v1::Event::WorkspaceEnter { workspace } => {
                data.pending_info.workspace.insert(workspace);
            }
            zcosmic_toplevel_handle_v1::Event::WorkspaceLeave { workspace } => {
                data.pending_info.workspace.remove(&workspace);
            }
            zcosmic_toplevel_handle_v1::Event::State { state } => {
                data.has_cosmic_info = true;
                data.pending_info.state.clear();
                for value in state.chunks_exact(4) {
                    if let Ok(state) = zcosmic_toplevel_handle_v1::State::try_from(
                        u32::from_ne_bytes(value[0..4].try_into().unwrap()),
                    ) {
                        data.pending_info.state.insert(state);
                    }
                }
            }
            zcosmic_toplevel_handle_v1::Event::Geometry {
                output,
                x,
                y,
                width,
                height,
            } => {
                data.pending_info.geometry.insert(
                    output,
                    ToplevelGeometry {
                        x,
                        y,
                        width,
                        height,
                    },
                );
            }
            // Not used in protocol version 2
            zcosmic_toplevel_handle_v1::Event::AppId { .. }
            | zcosmic_toplevel_handle_v1::Event::Title { .. }
            | zcosmic_toplevel_handle_v1::Event::Done { .. }
            | zcosmic_toplevel_handle_v1::Event::Closed { .. } => {}
            _ => unreachable!(),
        }
    }
}

impl<D> Dispatch<ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1, GlobalData, D>
    for ToplevelInfoState
where
    D: Dispatch<ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1, GlobalData>
        + Dispatch<ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1, GlobalData>
        + Dispatch<zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1, GlobalData>
        + ToplevelInfoHandler
        + 'static,
{
    fn event(
        state: &mut D,
        proxy: &ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1,
        event: ext_foreign_toplevel_list_v1::Event,
        _: &GlobalData,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        match event {
            ext_foreign_toplevel_list_v1::Event::Toplevel { toplevel } => {
                let info_state = state.toplevel_info_state();
                let cosmic_toplevel = info_state
                    .cosmic_toplevel_info
                    .get_cosmic_toplevel(&toplevel, qh, GlobalData);
                let toplevel_data = ToplevelData::new(toplevel);
                info_state.toplevels.push((cosmic_toplevel, toplevel_data));
            }
            ext_foreign_toplevel_list_v1::Event::Finished => {
                state.finished(conn, qh);
                proxy.destroy();
            }
            _ => unreachable!(),
        }
    }

    wayland_client::event_created_child!(D, ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1, [
        ext_foreign_toplevel_list_v1::EVT_TOPLEVEL_OPCODE => (ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1, Default::default())
    ]);
}

impl<D> Dispatch<ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1, GlobalData, D>
    for ToplevelInfoState
where
    D: Dispatch<ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1, GlobalData>
        + ToplevelInfoHandler,
{
    fn event(
        state: &mut D,
        handle: &ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
        event: ext_foreign_toplevel_handle_v1::Event,
        _data: &GlobalData,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        let (toplevel, data) = &mut state
            .toplevel_info_state()
            .toplevels
            .iter_mut()
            .find(|(_, data)| data.pending_info.foreign_toplevel == *handle)
            .expect("Received event for dead toplevel");
        match event {
            ext_foreign_toplevel_handle_v1::Event::Closed => {
                let toplevel = toplevel.clone();
                state.toplevel_closed(conn, qh, &toplevel);

                let toplevels = &mut state.toplevel_info_state().toplevels;
                if let Some(idx) = toplevels.iter().position(|(handle, _)| handle == &toplevel) {
                    toplevels.remove(idx);
                }
            }
            ext_foreign_toplevel_handle_v1::Event::Done => {
                if !data.has_cosmic_info {
                    // Don't call `new_toplevel` if we have the `ext_foreign_toplevel_handle_v1`,
                    // but don't have any `zcosmic_toplevel_handle_v1` events yet.
                    return;
                }

                let is_new = data.current_info.is_none();
                data.current_info = Some(data.pending_info.clone());
                let toplevel = toplevel.clone();
                if is_new {
                    state.new_toplevel(conn, qh, &toplevel);
                } else {
                    state.update_toplevel(conn, qh, &toplevel);
                }
            }
            ext_foreign_toplevel_handle_v1::Event::Title { title } => {
                data.pending_info.title = title;
            }
            ext_foreign_toplevel_handle_v1::Event::AppId { app_id } => {
                data.pending_info.app_id = app_id;
            }
            ext_foreign_toplevel_handle_v1::Event::Identifier { identifier } => {
                data.pending_info.identifier = Some(identifier);
            }
            _ => unreachable!(),
        }
    }
}

#[macro_export]
macro_rules! delegate_toplevel_info {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::cosmic_protocols::toplevel_info::v1::client::zcosmic_toplevel_info_v1::ZcosmicToplevelInfoV1: $crate::GlobalData
        ] => $crate::toplevel_info::ToplevelInfoState);
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::cosmic_protocols::toplevel_info::v1::client::zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1: $crate::GlobalData
        ] => $crate::toplevel_info::ToplevelInfoState);
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1: $crate::GlobalData
        ] => $crate::toplevel_info::ToplevelInfoState);
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1: $crate::GlobalData
        ] => $crate::toplevel_info::ToplevelInfoState);
    };
}
