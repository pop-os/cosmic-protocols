use cosmic_client_toolkit::keymap::KeymapState;
use sctk::{
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        keyboard::{KeyEvent, KeyboardHandler, Keymap, Keysym, Modifiers},
        Capability, SeatHandler, SeatState,
    },
};
use std::{
    io::{self, Write},
    str::FromStr,
};
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_keyboard, wl_seat, wl_surface},
    Connection, QueueHandle,
};
use xkbcommon::xkb;

struct AppData {
    registry_state: RegistryState,
    seat_state: SeatState,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    keymap_state: KeymapState,
    keymap: Option<xkb::Keymap>,
}

impl ProvidesRegistryState for AppData {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers![SeatState,];
}

impl SeatHandler for AppData {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard {
            let keyboard = self.seat_state.get_keyboard(qh, &seat, None).unwrap();
            self.keyboard = Some(keyboard);
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        _capability: Capability,
    ) {
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl KeyboardHandler for AppData {
    fn enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _surface: &wl_surface::WlSurface,
        _: u32,
        _: &[u32],
        _keysyms: &[Keysym],
    ) {
    }

    fn leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _surface: &wl_surface::WlSurface,
        _: u32,
    ) {
    }

    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        _event: KeyEvent,
    ) {
    }

    fn release_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        _event: KeyEvent,
    ) {
    }

    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _serial: u32,
        _modifiers: Modifiers,
        _layout: u32,
    ) {
    }

    fn update_keymap(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        keymap: Keymap<'_>,
    ) {
        let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
        let keymap = xkb::Keymap::new_from_string(
            &context,
            keymap.as_string(),
            xkb::KEYMAP_FORMAT_TEXT_V1,
            xkb::KEYMAP_COMPILE_NO_FLAGS,
        )
        .unwrap();
        self.keymap = Some(keymap);
    }
}

fn main() {
    let conn = Connection::connect_to_env().unwrap();
    let (globals, mut event_queue) = registry_queue_init(&conn).unwrap();
    let qh = event_queue.handle();

    let registry_state = RegistryState::new(&globals);
    let seat_state = SeatState::new(&globals, &qh);
    let keymap_state = KeymapState::new(&registry_state, &qh);
    let mut app_data = AppData {
        registry_state,
        seat_state,
        keymap_state,
        keyboard: None,
        keymap: None,
    };

    while app_data.keymap.is_none() {
        event_queue.blocking_dispatch(&mut app_data).unwrap();
    }

    let keymap = app_data.keymap.as_ref().unwrap();
    for (n, name) in keymap.layouts().enumerate() {
        println!("{}: {}", n, name);
    }
    print!("Choose layout: ");

    io::stdout().flush().unwrap();
    let mut line = String::new();
    io::stdin().read_line(&mut line).unwrap();
    let index = u32::from_str(line.trim()).unwrap();

    let keymap_manager = app_data.keymap_state.keymap_manager.as_ref().unwrap();
    let keyboard = app_data.keyboard.as_ref().unwrap();
    keymap_manager.set_group(keyboard, index);

    event_queue.roundtrip(&mut app_data).unwrap();
}

sctk::delegate_registry!(AppData);
sctk::delegate_seat!(AppData);
sctk::delegate_keyboard!(AppData);
cosmic_client_toolkit::delegate_keymap!(AppData);
