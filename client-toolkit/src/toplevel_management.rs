use cosmic_protocols::toplevel_management::v1::client::zcosmic_toplevel_manager_v1;
use sctk::registry::RegistryState;
use wayland_client::{Connection, Dispatch, QueueHandle, WEnum};

pub struct ToplevelManagerState {
    pub manager: zcosmic_toplevel_manager_v1::ZcosmicToplevelManagerV1,
}

impl ToplevelManagerState {
    pub fn new<D>(registry: &RegistryState, qh: &QueueHandle<D>) -> Self
    where
        D: Dispatch<zcosmic_toplevel_manager_v1::ZcosmicToplevelManagerV1, ()> + 'static,
    {
        let manager = registry
            .bind_one::<zcosmic_toplevel_manager_v1::ZcosmicToplevelManagerV1, _, _>(qh, 1..=2, ())
            .unwrap();

        Self { manager }
    }
}

impl<D> Dispatch<zcosmic_toplevel_manager_v1::ZcosmicToplevelManagerV1, (), D>
    for ToplevelManagerState
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
                let capabilities = capabilities
                    .chunks(4)
                    .map(|chunk| WEnum::from(u32::from_ne_bytes(chunk.try_into().unwrap())))
                    .collect();
                state.capabilities(conn, qhandle, capabilities)
            }
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
        capabilities: Vec<
            WEnum<zcosmic_toplevel_manager_v1::ZcosmicToplelevelManagementCapabilitiesV1>,
        >,
    );
}

#[macro_export]
macro_rules! delegate_toplevel_manager {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::cosmic_protocols::toplevel_management::v1::client::zcosmic_toplevel_manager_v1::ZcosmicToplevelManagerV1: ()
        ] => $crate::toplevel_management::ToplevelManagerState);
    };
}
