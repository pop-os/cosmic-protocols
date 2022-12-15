use cosmic_protocols::{toplevel_management::v1::client::zcosmic_toplevel_manager_v1, toplevel_info::v1::client::zcosmic_toplevel_info_v1};
use sctk::registry::RegistryState;
use wayland_client::{QueueHandle, Dispatch, Connection};

pub struct ToplevelManagerState {
    pub manager: zcosmic_toplevel_manager_v1::ZcosmicToplevelManagerV1
}

impl ToplevelManagerState {
    pub fn new<D>(registry: &RegistryState, qh: &QueueHandle<D>) -> Self
    where
        D: Dispatch<zcosmic_toplevel_manager_v1::ZcosmicToplevelManagerV1, ()> + 'static,
    {
        let manager = registry
            .bind_one::<zcosmic_toplevel_manager_v1::ZcosmicToplevelManagerV1, _, _>(qh, 1..=1, ())
            .unwrap();

        Self {
            manager
        }
    }
}

impl<D> Dispatch<zcosmic_toplevel_manager_v1::ZcosmicToplevelManagerV1, (), D> for ToplevelManagerState
where
    D: Dispatch<zcosmic_toplevel_manager_v1::ZcosmicToplevelManagerV1, ()>
        + Dispatch<zcosmic_toplevel_manager_v1::ZcosmicToplevelManagerV1, ()>
        + ToplevelManagerHandler
        + 'static,
{
    fn event(
        state: &mut D,
        _proxy: &zcosmic_toplevel_manager_v1::ZcosmicToplevelManagerV1,
        event: <zcosmic_toplevel_manager_v1::ZcosmicToplevelManagerV1 as wayland_client::Proxy>::Event,
        _data: &(),
        conn: &wayland_client::Connection,
        qhandle: &QueueHandle<D>,
    ) {
        match event {
            zcosmic_toplevel_manager_v1::Event::Capabilities { capabilities } => {
                state.capabilities(conn, qhandle, capabilities)
            },
            _ => unimplemented!(),
        }
    }
}

pub trait ToplevelManagerHandler: Sized {
    fn toplevel_manager_state(&mut self) -> &mut ToplevelManagerState;

    fn capabilities(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        capabilities: Vec<u8>
    );
}

#[macro_export]
macro_rules! delegate_toplevel_manager {
    ($ty: ty) => {
        $crate::wayland_client::delegate_dispatch!($ty: [
            $crate::cosmic_protocols::toplevel_management::v1::client::zcosmic_toplevel_manager_v1::ZcosmicToplevelManagerV1: ()
        ] => $crate::toplevel_management::ToplevelManagerState);
    };
}
