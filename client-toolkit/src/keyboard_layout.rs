use cosmic_protocols::keyboard_layout::v1::client::{
    zcosmic_keyboard_layout_manager_v1, zcosmic_keyboard_layout_v1,
};
use sctk::registry::RegistryState;
use wayland_client::{Connection, Dispatch, QueueHandle, protocol::wl_keyboard};

pub trait KeyboardLayoutHandler: Sized {
    fn group(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        keyboard: &wl_keyboard::WlKeyboard,
        keyboard_layout: &zcosmic_keyboard_layout_v1::ZcosmicKeyboardLayoutV1,
        group: u32,
    );
}

pub struct KeyboardLayoutState {
    pub keyboard_layout_manager:
        Option<zcosmic_keyboard_layout_manager_v1::ZcosmicKeyboardLayoutManagerV1>,
}

impl KeyboardLayoutState {
    pub fn new<D>(registry: &RegistryState, qh: &QueueHandle<D>) -> Self
    where
        D: Dispatch<zcosmic_keyboard_layout_manager_v1::ZcosmicKeyboardLayoutManagerV1, ()>
            + 'static,
    {
        let keyboard_layout_manager = registry
            .bind_one::<zcosmic_keyboard_layout_manager_v1::ZcosmicKeyboardLayoutManagerV1, _, _>(
                qh,
                1..=1,
                (),
            )
            .ok();

        Self {
            keyboard_layout_manager,
        }
    }

    pub fn get_keyboard_layout<D>(
        &self,
        keyboard: &wl_keyboard::WlKeyboard,
        qh: &QueueHandle<D>,
    ) -> Option<zcosmic_keyboard_layout_v1::ZcosmicKeyboardLayoutV1>
    where
        D: Dispatch<zcosmic_keyboard_layout_v1::ZcosmicKeyboardLayoutV1, KeyboardLayoutUserData>
            + 'static,
    {
        Some(self.keyboard_layout_manager.as_ref()?.get_keyboard_layout(
            keyboard,
            qh,
            KeyboardLayoutUserData {
                keyboard: keyboard.clone(),
            },
        ))
    }
}

impl<D> Dispatch<zcosmic_keyboard_layout_manager_v1::ZcosmicKeyboardLayoutManagerV1, (), D>
    for KeyboardLayoutState
where
    D: Dispatch<zcosmic_keyboard_layout_manager_v1::ZcosmicKeyboardLayoutManagerV1, ()>,
{
    fn event(
        _: &mut D,
        _: &zcosmic_keyboard_layout_manager_v1::ZcosmicKeyboardLayoutManagerV1,
        event: zcosmic_keyboard_layout_manager_v1::Event,
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
pub struct KeyboardLayoutUserData {
    keyboard: wl_keyboard::WlKeyboard,
}

impl<D> Dispatch<zcosmic_keyboard_layout_v1::ZcosmicKeyboardLayoutV1, KeyboardLayoutUserData, D>
    for KeyboardLayoutState
where
    D: Dispatch<zcosmic_keyboard_layout_v1::ZcosmicKeyboardLayoutV1, KeyboardLayoutUserData>
        + KeyboardLayoutHandler,
{
    fn event(
        state: &mut D,
        keyboard_layout: &zcosmic_keyboard_layout_v1::ZcosmicKeyboardLayoutV1,
        event: zcosmic_keyboard_layout_v1::Event,
        data: &KeyboardLayoutUserData,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        match event {
            zcosmic_keyboard_layout_v1::Event::Group { group } => {
                state.group(conn, qh, &data.keyboard, keyboard_layout, group);
            }
            _ => unreachable!(),
        }
    }
}

#[macro_export]
macro_rules! delegate_keyboard_layout {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::cosmic_protocols::keyboard_layout::v1::client::zcosmic_keyboard_layout_manager_v1::ZcosmicKeyboardLayoutManagerV1: ()
        ] => $crate::keyboard_layout::KeyboardLayoutState);
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::cosmic_protocols::keyboard_layout::v1::client::zcosmic_keyboard_layout_v1::ZcosmicKeyboardLayoutV1: $crate::keyboard_layout::KeyboardLayoutUserData
        ] => $crate::keyboard_layout::KeyboardLayoutState);
    };
}
