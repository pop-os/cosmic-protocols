use cosmic_protocols::toplevel_management::v1::client::zcosmic_toplevel_manager_v1;
use wayland_client::{Connection, Dispatch, QueueHandle, globals::GlobalList};

use crate::GlobalData;

pub struct ToplevelManagerState {
    pub manager: zcosmic_toplevel_manager_v1::ZcosmicToplevelManagerV1,
}

impl ToplevelManagerState {
    pub fn try_new<D>(globals: &GlobalList, qh: &QueueHandle<D>) -> Option<Self>
    where
        D: ToplevelManagerHandler + 'static,
    {
        let manager = globals
            .bind_singleton::<zcosmic_toplevel_manager_v1::ZcosmicToplevelManagerV1, _, _>(
                qh,
                1..=4,
                GlobalData,
            )
            .ok()?;

        Some(Self { manager })
    }

    pub fn new<D>(globals: &GlobalList, qh: &QueueHandle<D>) -> Self
    where
        D: ToplevelManagerHandler + 'static,
    {
        Self::try_new(globals, qh).unwrap()
    }
}

impl<D> Dispatch<zcosmic_toplevel_manager_v1::ZcosmicToplevelManagerV1, D> for GlobalData
where
    D: ToplevelManagerHandler + 'static,
{
    fn event(
        &self,
        state: &mut D,
        _proxy: &zcosmic_toplevel_manager_v1::ZcosmicToplevelManagerV1,
        event: <zcosmic_toplevel_manager_v1::ZcosmicToplevelManagerV1 as wayland_client::Proxy>::Event,
        conn: &wayland_client::Connection,
        qhandle: &QueueHandle<D>,
    ) {
        match event {
            zcosmic_toplevel_manager_v1::Event::Capabilities { capabilities } => {
                let capabilities = capabilities
                    .chunks(4)
                    .map(|chunk| {
                        zcosmic_toplevel_manager_v1::ZcosmicToplelevelManagementCapabilitiesV1(
                            u32::from_ne_bytes(chunk.try_into().unwrap()),
                        )
                    })
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
        capabilities: Vec<zcosmic_toplevel_manager_v1::ZcosmicToplelevelManagementCapabilitiesV1>,
    );
}
