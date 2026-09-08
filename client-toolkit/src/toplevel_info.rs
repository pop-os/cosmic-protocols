use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::Read,
    os::fd::OwnedFd,
    sync::{Arc, OnceLock},
};

use cosmic_protocols::toplevel_info::v1::client::{
    zcosmic_toplevel_handle_v1, zcosmic_toplevel_info_v1,
};
use sctk::registry::RegistryState;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, Weak, protocol::wl_output};
use wayland_protocols::ext::{
    foreign_toplevel_list::v1::client::{
        ext_foreign_toplevel_handle_v1, ext_foreign_toplevel_list_v1,
    },
    workspace::v1::client::ext_workspace_handle_v1,
};

use crate::GlobalData;

#[derive(Clone, Debug, Default)]
pub struct ToplevelGeometry {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToplevelIcon {
    Name(Arc<str>),
    Rgba {
        width: u32,
        height: u32,
        pixels: Arc<[u8]>,
    },
}

impl ToplevelIcon {
    const MAX_DIMENSION: u32 = 512;
    const MAX_BYTES: usize = 512 * 512 * 4;

    pub fn from_rgba(width: u32, height: u32, pixels: impl Into<Arc<[u8]>>) -> Option<Self> {
        if width == 0 || height == 0 || width > Self::MAX_DIMENSION || height > Self::MAX_DIMENSION
        {
            return None;
        }
        let pixels = pixels.into();
        let len = usize::try_from(width)
            .ok()?
            .checked_mul(usize::try_from(height).ok()?)?
            .checked_mul(4)?;
        if len > Self::MAX_BYTES || pixels.len() != len {
            return None;
        }
        Some(Self::Rgba {
            width,
            height,
            pixels,
        })
    }

    fn from_fd(data: OwnedFd, width: u32, height: u32) -> Option<Self> {
        let len = usize::try_from(width)
            .ok()?
            .checked_mul(usize::try_from(height).ok()?)?
            .checked_mul(4)?;
        if width == 0
            || height == 0
            || width > Self::MAX_DIMENSION
            || height > Self::MAX_DIMENSION
            || len > Self::MAX_BYTES
        {
            return None;
        }

        let mut file = File::from(data);
        let metadata = file.metadata().ok()?;
        if !metadata.is_file() || metadata.len() != u64::try_from(len).ok()? {
            return None;
        }

        let mut pixels = vec![0; len];
        file.read_exact(&mut pixels).ok()?;
        Self::from_rgba(width, height, pixels)
    }
}

fn apply_icon_fd(icon: &mut Option<ToplevelIcon>, data: OwnedFd, width: u32, height: u32) -> bool {
    let Some(new_icon) = ToplevelIcon::from_fd(data, width, height) else {
        return false;
    };
    *icon = Some(new_icon);
    true
}

fn apply_pid(current: &mut Option<u32>, pid: u32) {
    *current = (pid != 0).then_some(pid);
}

#[derive(Clone, Debug)]
pub struct ToplevelInfo {
    pub title: String,
    pub app_id: String,
    pub identifier: String,
    /// Requires zcosmic_toplevel_info_v1 version 2
    pub state: HashSet<zcosmic_toplevel_handle_v1::State>,
    /// Requires zcosmic_toplevel_info_v1 version 2
    pub output: HashSet<wl_output::WlOutput>,
    /// Requires zcosmic_toplevel_info_v1 version 2
    pub geometry: HashMap<wl_output::WlOutput, ToplevelGeometry>,
    /// Requires zcosmic_toplevel_info_v1 version 3
    pub workspace: HashSet<ext_workspace_handle_v1::ExtWorkspaceHandleV1>,
    /// Requires zcosmic_toplevel_info_v1 version 4
    pub icon: Option<ToplevelIcon>,
    /// Requires zcosmic_toplevel_info_v1 version 4
    pub pid: Option<u32>,
    /// Requires zcosmic_toplevel_info_v1 version 2
    pub cosmic_toplevel: Option<zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1>,
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
            identifier: String::new(),
            state: HashSet::new(),
            output: HashSet::new(),
            geometry: HashMap::new(),
            workspace: HashSet::new(),
            icon: None,
            pid: None,
            cosmic_toplevel: None,
            foreign_toplevel,
        };
        Self {
            current_info: None,
            pending_info,
            has_cosmic_info: false,
        }
    }

    fn cosmic_toplevel(&self) -> Option<&zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1> {
        self.pending_info.cosmic_toplevel.as_ref()
    }

    fn foreign_toplevel(&self) -> &ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1 {
        &self.pending_info.foreign_toplevel
    }
}

#[doc(hidden)]
#[derive(Default)]
pub struct ToplevelUserData {
    cosmic_toplevel: OnceLock<Option<Weak<zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1>>>,
}

/// Handler for `ext-foreign-toplevel-list-v1`, and optionally
/// `cosmic-toplevel-info-unstable-v1` which extends it with additional information.
#[derive(Debug)]
pub struct ToplevelInfoState {
    pub foreign_toplevel_list: ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1,
    pub cosmic_toplevel_info: Option<zcosmic_toplevel_info_v1::ZcosmicToplevelInfoV1>,
    toplevels: Vec<ToplevelData>,
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
                2..=4,
                GlobalData,
            )
            .ok();

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
        toplevel: &ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
    ) -> Option<&ToplevelInfo> {
        self.toplevels
            .iter()
            .find(|data| data.foreign_toplevel() == toplevel)?
            .current_info
            .as_ref()
    }

    pub fn toplevels(&self) -> impl Iterator<Item = &ToplevelInfo> {
        self.toplevels
            .iter()
            .filter_map(|data| data.current_info.as_ref())
    }
}

pub trait ToplevelInfoHandler: Sized {
    fn toplevel_info_state(&mut self) -> &mut ToplevelInfoState;

    fn new_toplevel(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        toplevel: &ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
    );

    fn update_toplevel(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        toplevel: &ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
    );

    fn toplevel_closed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        toplevel: &ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
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
            .find(|data| data.cosmic_toplevel() == Some(toplevel))
            .expect("Received event for dead toplevel");
        match event {
            zcosmic_toplevel_handle_v1::Event::OutputEnter { output } => {
                data.pending_info.output.insert(output);
            }
            zcosmic_toplevel_handle_v1::Event::OutputLeave { output } => {
                data.pending_info.output.remove(&output);
                data.pending_info.geometry.remove(&output);
            }
            // Ignore legacy workspace handle events
            zcosmic_toplevel_handle_v1::Event::WorkspaceEnter { .. }
            | zcosmic_toplevel_handle_v1::Event::WorkspaceLeave { .. } => {}
            zcosmic_toplevel_handle_v1::Event::ExtWorkspaceEnter { workspace } => {
                data.pending_info.workspace.insert(workspace);
            }
            zcosmic_toplevel_handle_v1::Event::ExtWorkspaceLeave { workspace } => {
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
            zcosmic_toplevel_handle_v1::Event::IconName { icon_name } => {
                data.pending_info.icon = Some(ToplevelIcon::Name(icon_name.into()));
            }
            zcosmic_toplevel_handle_v1::Event::Icon {
                data: icon_data,
                width,
                height,
            } => {
                apply_icon_fd(&mut data.pending_info.icon, icon_data, width, height);
            }
            zcosmic_toplevel_handle_v1::Event::IconRemoved => {
                data.pending_info.icon = None;
            }
            zcosmic_toplevel_handle_v1::Event::Pid { pid } => {
                apply_pid(&mut data.pending_info.pid, pid);
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
        + Dispatch<ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1, ToplevelUserData>
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
                let mut toplevel_data = ToplevelData::new(toplevel.clone());
                let cosmic_toplevel =
                    info_state
                        .cosmic_toplevel_info
                        .as_ref()
                        .map(|cosmic_toplevel_info| {
                            cosmic_toplevel_info.get_cosmic_toplevel(&toplevel, qh, GlobalData)
                        });
                toplevel
                    .data::<ToplevelUserData>()
                    .unwrap()
                    .cosmic_toplevel
                    .set(cosmic_toplevel.as_ref().map(|t| t.downgrade()))
                    .unwrap();
                toplevel_data.pending_info.cosmic_toplevel = cosmic_toplevel;
                info_state.toplevels.push(toplevel_data);
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

impl<D> Dispatch<ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1, ToplevelUserData, D>
    for ToplevelInfoState
where
    D: Dispatch<ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1, ToplevelUserData>
        + ToplevelInfoHandler,
{
    fn event(
        state: &mut D,
        handle: &ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
        event: ext_foreign_toplevel_handle_v1::Event,
        _data: &ToplevelUserData,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        let data = &mut state
            .toplevel_info_state()
            .toplevels
            .iter_mut()
            .find(|data| data.foreign_toplevel() == handle)
            .expect("Received event for dead toplevel");
        match event {
            ext_foreign_toplevel_handle_v1::Event::Closed => {
                state.toplevel_closed(conn, qh, handle);

                let toplevels = &mut state.toplevel_info_state().toplevels;
                if let Some(idx) = toplevels
                    .iter()
                    .position(|data| data.foreign_toplevel() == handle)
                {
                    toplevels.remove(idx);
                }
            }
            ext_foreign_toplevel_handle_v1::Event::Done => {
                if data.cosmic_toplevel().is_some() && !data.has_cosmic_info {
                    // Don't call `new_toplevel` if we have the `ext_foreign_toplevel_handle_v1`,
                    // but don't have any `zcosmic_toplevel_handle_v1` events yet.
                    return;
                }

                let is_new = data.current_info.is_none();
                data.current_info = Some(data.pending_info.clone());
                if is_new {
                    state.new_toplevel(conn, qh, handle);
                } else {
                    state.update_toplevel(conn, qh, handle);
                }
            }
            ext_foreign_toplevel_handle_v1::Event::Title { title } => {
                data.pending_info.title = title;
            }
            ext_foreign_toplevel_handle_v1::Event::AppId { app_id } => {
                data.pending_info.app_id = app_id;
            }
            ext_foreign_toplevel_handle_v1::Event::Identifier { identifier } => {
                data.pending_info.identifier = identifier;
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
            $crate::wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1: $crate::toplevel_info::ToplevelUserData
        ] => $crate::toplevel_info::ToplevelInfoState);
    };
}
