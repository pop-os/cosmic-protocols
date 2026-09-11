use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fmt,
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
use crate::screencopy::{CaptureSource, ToplevelIconCaptureSource};

#[derive(Clone, Debug, Default)]
pub struct ToplevelGeometry {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToplevelIcon {
    /// An icon name following the XDG icon theme specification, when available.
    pub name: Option<Arc<str>>,
    generation: u64,
    toplevel: zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1,
}

impl ToplevelIcon {
    /// Returns the generation associated with this icon announcement.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Create a deferred capture source descriptor at the requested dimensions.
    ///
    /// The Wayland source request is sent by [`crate::screencopy::Capturer::create_session`]
    /// and targets the icon current when that request is processed. Callers that
    /// require the pixels to match this announcement should compare `generation`
    /// with the latest [`ToplevelInfo::icon_generation`] after capture and retry
    /// if it changed.
    pub fn capture_source(
        &self,
        width: u32,
        height: u32,
    ) -> Result<CaptureSource, ToplevelIconCaptureError> {
        validate_icon_capture_source(self.toplevel.version(), width, height)?;
        Ok(CaptureSource::ToplevelIcon(ToplevelIconCaptureSource::new(
            self.toplevel.clone(),
            width,
            height,
        )))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToplevelIconCaptureError {
    InvalidSize,
    UnsupportedVersion,
}

impl fmt::Display for ToplevelIconCaptureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSize => f.write_str("icon capture dimensions must be non-zero"),
            Self::UnsupportedVersion => {
                f.write_str("icon capture requires cosmic toplevel info version 4")
            }
        }
    }
}

impl Error for ToplevelIconCaptureError {}

fn validate_icon_capture_source(
    version: u32,
    width: u32,
    height: u32,
) -> Result<(), ToplevelIconCaptureError> {
    if version < 4 {
        Err(ToplevelIconCaptureError::UnsupportedVersion)
    } else if width == 0 || height == 0 {
        Err(ToplevelIconCaptureError::InvalidSize)
    } else {
        Ok(())
    }
}

fn next_icon_generation(generation: u64) -> u64 {
    generation.wrapping_add(1)
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
    /// Changes whenever the icon is replaced or removed.
    pub icon_generation: u64,
    /// Requires zcosmic_toplevel_info_v1 version 2
    pub cosmic_toplevel: Option<zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1>,
    pub foreign_toplevel: ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
}

#[derive(Debug)]
struct ToplevelData {
    current_info: Option<ToplevelInfo>,
    pending_info: ToplevelInfo,
    committed_info: ToplevelInfo,
    sync: ToplevelSyncState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SyncAction {
    None,
    New,
    Update,
}

#[derive(Clone, Copy, Debug)]
enum SyncStream {
    Cosmic,
    Foreign,
}

#[derive(Debug, Default)]
struct ToplevelSyncState {
    cosmic_dirty: bool,
    foreign_dirty: bool,
    cosmic_done: bool,
    foreign_done: bool,
}

impl ToplevelSyncState {
    fn cosmic_changed(&mut self) {
        self.cosmic_dirty = true;
    }

    fn foreign_changed(&mut self) {
        self.foreign_dirty = true;
    }

    fn cosmic_needs_commit(&self) -> bool {
        !self.cosmic_done || self.cosmic_dirty
    }

    fn foreign_needs_commit(&self) -> bool {
        !self.foreign_done || self.foreign_dirty
    }

    fn finish_cosmic(&mut self, initialized: bool) -> SyncAction {
        let dirty = std::mem::take(&mut self.cosmic_dirty);
        self.cosmic_done = true;
        if !initialized && self.foreign_done {
            SyncAction::New
        } else if initialized && dirty {
            SyncAction::Update
        } else {
            SyncAction::None
        }
    }

    fn finish_foreign(&mut self, initialized: bool) -> SyncAction {
        let dirty = std::mem::take(&mut self.foreign_dirty);
        self.foreign_done = true;
        if !initialized && self.cosmic_done {
            SyncAction::New
        } else if initialized && dirty {
            SyncAction::Update
        } else {
            SyncAction::None
        }
    }
}

fn finish_stream<T: Clone>(
    current: &mut Option<T>,
    pending: &T,
    committed: &mut T,
    sync: &mut ToplevelSyncState,
    stream: SyncStream,
    commit_fields: impl Fn(&mut T, &T),
) -> SyncAction {
    let needs_commit = match stream {
        SyncStream::Cosmic => sync.cosmic_needs_commit(),
        SyncStream::Foreign => sync.foreign_needs_commit(),
    };
    if needs_commit {
        commit_fields(committed, pending);
    }

    let action = match stream {
        SyncStream::Cosmic => sync.finish_cosmic(current.is_some()),
        SyncStream::Foreign => sync.finish_foreign(current.is_some()),
    };
    match action {
        SyncAction::New => *current = Some(committed.clone()),
        SyncAction::Update => commit_fields(current.as_mut().unwrap(), committed),
        SyncAction::None => {}
    }
    action
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
            icon_generation: 0,
            cosmic_toplevel: None,
            foreign_toplevel,
        };
        Self {
            current_info: None,
            committed_info: pending_info.clone(),
            pending_info,
            sync: ToplevelSyncState::default(),
        }
    }

    fn cosmic_toplevel(&self) -> Option<&zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1> {
        self.pending_info.cosmic_toplevel.as_ref()
    }

    fn foreign_toplevel(&self) -> &ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1 {
        &self.pending_info.foreign_toplevel
    }
}

fn commit_cosmic_fields(current: &mut ToplevelInfo, pending: &ToplevelInfo) {
    current.cosmic_toplevel = pending.cosmic_toplevel.clone();
    current.state.clone_from(&pending.state);
    current.output.clone_from(&pending.output);
    current.workspace.clone_from(&pending.workspace);
    current.geometry.clone_from(&pending.geometry);
    current.icon.clone_from(&pending.icon);
    current.icon_generation = pending.icon_generation;
}

fn commit_foreign_fields(current: &mut ToplevelInfo, pending: &ToplevelInfo) {
    current.title.clone_from(&pending.title);
    current.app_id.clone_from(&pending.app_id);
    current.identifier.clone_from(&pending.identifier);
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
                let updates = {
                    let info_state = state.toplevel_info_state();
                    info_state
                        .toplevels
                        .iter_mut()
                        .filter_map(|data| {
                            let action = finish_stream(
                                &mut data.current_info,
                                &data.pending_info,
                                &mut data.committed_info,
                                &mut data.sync,
                                SyncStream::Cosmic,
                                commit_cosmic_fields,
                            );
                            if action == SyncAction::None {
                                return None;
                            }
                            Some((
                                data.pending_info.foreign_toplevel.clone(),
                                action == SyncAction::New,
                            ))
                        })
                        .collect::<Vec<_>>()
                };
                for (handle, is_new) in updates {
                    if is_new {
                        state.new_toplevel(conn, qh, &handle);
                    } else {
                        state.update_toplevel(conn, qh, &handle);
                    }
                }
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
                data.sync.cosmic_changed();
            }
            zcosmic_toplevel_handle_v1::Event::OutputLeave { output } => {
                data.pending_info.output.remove(&output);
                data.pending_info.geometry.remove(&output);
                data.sync.cosmic_changed();
            }
            // Ignore legacy workspace handle events
            zcosmic_toplevel_handle_v1::Event::WorkspaceEnter { .. }
            | zcosmic_toplevel_handle_v1::Event::WorkspaceLeave { .. } => {}
            zcosmic_toplevel_handle_v1::Event::ExtWorkspaceEnter { workspace } => {
                data.pending_info.workspace.insert(workspace);
                data.sync.cosmic_changed();
            }
            zcosmic_toplevel_handle_v1::Event::ExtWorkspaceLeave { workspace } => {
                data.pending_info.workspace.remove(&workspace);
                data.sync.cosmic_changed();
            }
            zcosmic_toplevel_handle_v1::Event::State { state } => {
                data.pending_info.state.clear();
                for value in state.chunks_exact(4) {
                    if let Ok(state) = zcosmic_toplevel_handle_v1::State::try_from(
                        u32::from_ne_bytes(value[0..4].try_into().unwrap()),
                    ) {
                        data.pending_info.state.insert(state);
                    }
                }
                data.sync.cosmic_changed();
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
                data.sync.cosmic_changed();
            }
            zcosmic_toplevel_handle_v1::Event::IconChanged { icon_name } => {
                data.pending_info.icon_generation =
                    next_icon_generation(data.pending_info.icon_generation);
                data.pending_info.icon = Some(ToplevelIcon {
                    name: icon_name.map(Into::into),
                    generation: data.pending_info.icon_generation,
                    toplevel: toplevel.clone(),
                });
                data.sync.cosmic_changed();
            }
            zcosmic_toplevel_handle_v1::Event::IconRemoved => {
                data.pending_info.icon_generation =
                    next_icon_generation(data.pending_info.icon_generation);
                data.pending_info.icon = None;
                data.sync.cosmic_changed();
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
                toplevel_data.sync.cosmic_done = cosmic_toplevel.is_none();
                toplevel_data.pending_info.cosmic_toplevel = cosmic_toplevel;
                toplevel_data.committed_info.cosmic_toplevel =
                    toplevel_data.pending_info.cosmic_toplevel.clone();
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
                let action = finish_stream(
                    &mut data.current_info,
                    &data.pending_info,
                    &mut data.committed_info,
                    &mut data.sync,
                    SyncStream::Foreign,
                    commit_foreign_fields,
                );
                if action == SyncAction::None {
                    return;
                }

                if action == SyncAction::New {
                    state.new_toplevel(conn, qh, handle);
                } else {
                    state.update_toplevel(conn, qh, handle);
                }
            }
            ext_foreign_toplevel_handle_v1::Event::Title { title } => {
                data.pending_info.title = title;
                data.sync.foreign_changed();
            }
            ext_foreign_toplevel_handle_v1::Event::AppId { app_id } => {
                data.pending_info.app_id = app_id;
                data.sync.foreign_changed();
            }
            ext_foreign_toplevel_handle_v1::Event::Identifier { identifier } => {
                data.pending_info.identifier = identifier;
                data.sync.foreign_changed();
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
