use cosmic_protocols::keymap::v1::client::zcosmic_keymap_manager_v1;
use sctk::registry::RegistryState;
use wayland_client::{protocol::wl_output, Connection, Dispatch, QueueHandle};

pub struct KeymapState {
    pub keymap_manager: Option<zcosmic_keymap_manager_v1::ZcosmicKeymapManagerV1>,
}

impl KeymapState {
    pub fn new<D>(registry: &RegistryState, qh: &QueueHandle<D>) -> Self
    where
        D: Dispatch<zcosmic_keymap_manager_v1::ZcosmicKeymapManagerV1, ()> + 'static,
    {
        let keymap_manager = registry
            .bind_one::<zcosmic_keymap_manager_v1::ZcosmicKeymapManagerV1, _, _>(qh, 1..=1, ())
            .ok();

        Self { keymap_manager }
    }
}

impl<D> Dispatch<zcosmic_keymap_manager_v1::ZcosmicKeymapManagerV1, (), D> for KeymapState
where
    D: Dispatch<zcosmic_keymap_manager_v1::ZcosmicKeymapManagerV1, ()>,
{
    fn event(
        state: &mut D,
        _: &zcosmic_keymap_manager_v1::ZcosmicKeymapManagerV1,
        event: zcosmic_keymap_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        match event {
            _ => unreachable!(),
        }
    }
}

#[macro_export]
macro_rules! delegate_keymap {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::cosmic_protocols::keymap::v1::client::zcosmic_keymap_manager_v1::ZcosmicKeymapManagerV1: ()
        ] => $crate::keymap::KeymapState);
    };
}
