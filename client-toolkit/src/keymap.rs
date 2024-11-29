use cosmic_protocols::keymap::v1::client::{zcosmic_keymap_manager_v1, zcosmic_keymap_v1};
use sctk::registry::RegistryState;
use wayland_client::{protocol::wl_keyboard, Connection, Dispatch, QueueHandle};

pub trait KeymapHandler: Sized {
    fn group(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        keyboard: &wl_keyboard::WlKeyboard,
        keymap: &zcosmic_keymap_v1::ZcosmicKeymapV1,
        group: u32,
    );
}

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

    pub fn get_keymap<D>(
        &self,
        keyboard: &wl_keyboard::WlKeyboard,
        qh: &QueueHandle<D>,
    ) -> Option<zcosmic_keymap_v1::ZcosmicKeymapV1>
    where
        D: Dispatch<zcosmic_keymap_v1::ZcosmicKeymapV1, KeymapUserData> + 'static,
    {
        Some(self.keymap_manager.as_ref()?.get_keymap(
            keyboard,
            qh,
            KeymapUserData {
                keyboard: keyboard.clone(),
            },
        ))
    }
}

impl<D> Dispatch<zcosmic_keymap_manager_v1::ZcosmicKeymapManagerV1, (), D> for KeymapState
where
    D: Dispatch<zcosmic_keymap_manager_v1::ZcosmicKeymapManagerV1, ()>,
{
    fn event(
        _: &mut D,
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

#[doc(hidden)]
pub struct KeymapUserData {
    keyboard: wl_keyboard::WlKeyboard,
}

impl<D> Dispatch<zcosmic_keymap_v1::ZcosmicKeymapV1, KeymapUserData, D> for KeymapState
where
    D: Dispatch<zcosmic_keymap_v1::ZcosmicKeymapV1, KeymapUserData> + KeymapHandler,
{
    fn event(
        state: &mut D,
        keymap: &zcosmic_keymap_v1::ZcosmicKeymapV1,
        event: zcosmic_keymap_v1::Event,
        data: &KeymapUserData,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        match event {
            zcosmic_keymap_v1::Event::Group { group } => {
                state.group(conn, qh, &data.keyboard, keymap, group);
            }
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
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::cosmic_protocols::keymap::v1::client::zcosmic_keymap_v1::ZcosmicKeymapV1: $crate::keymap::KeymapUserData
        ] => $crate::keymap::KeymapState);
    };
}
