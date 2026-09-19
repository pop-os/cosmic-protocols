use cosmic_protocols::keyboard_layout::v1::client::{
    zcosmic_keyboard_layout_manager_v1, zcosmic_keyboard_layout_v1,
};
use wayland_client::{
    Connection, Dispatch, QueueHandle, globals::GlobalList, protocol::wl_keyboard,
};

use crate::GlobalData;

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
    pub fn new<D>(globals: &GlobalList, qh: &QueueHandle<D>) -> Self
    where
        D: 'static,
    {
        let keyboard_layout_manager = globals
            .bind_singleton::<zcosmic_keyboard_layout_manager_v1::ZcosmicKeyboardLayoutManagerV1, _, _>(
                qh,
                1..=1,
                GlobalData,
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
        D: KeyboardLayoutHandler + 'static,
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

impl<D> Dispatch<zcosmic_keyboard_layout_manager_v1::ZcosmicKeyboardLayoutManagerV1, D>
    for GlobalData
{
    fn event(
        &self,
        _: &mut D,
        _: &zcosmic_keyboard_layout_manager_v1::ZcosmicKeyboardLayoutManagerV1,
        event: zcosmic_keyboard_layout_manager_v1::Event,
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

impl<D> Dispatch<zcosmic_keyboard_layout_v1::ZcosmicKeyboardLayoutV1, D> for KeyboardLayoutUserData
where
    D: KeyboardLayoutHandler,
{
    fn event(
        &self,
        state: &mut D,
        keyboard_layout: &zcosmic_keyboard_layout_v1::ZcosmicKeyboardLayoutV1,
        event: zcosmic_keyboard_layout_v1::Event,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        match event {
            zcosmic_keyboard_layout_v1::Event::Group { group } => {
                state.group(conn, qh, &self.keyboard, keyboard_layout, group);
            }
            _ => unreachable!(),
        }
    }
}
