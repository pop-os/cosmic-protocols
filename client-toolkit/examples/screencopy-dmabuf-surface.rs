use std::{convert::TryInto, time::Duration};
use std::{
    fs,
    os::{fd::AsFd, unix::{ffi::OsStrExt, fs::MetadataExt}},
    str,
    sync::{Arc, Mutex},
};

use cosmic_client_toolkit::{
    screencopy::{
        BufferInfo, ScreencopyHandler, ScreencopySessionData, ScreencopySessionDataExt,
        ScreencopyState,
    },
    workspace::{WorkspaceGroup, WorkspaceHandler, WorkspaceState},
};
use cosmic_protocols::{
    screencopy::v1::client::{zcosmic_screencopy_manager_v1, zcosmic_screencopy_session_v1},
    workspace::v1::client::zcosmic_workspace_group_handle_v1,
};
use sctk::reexports::calloop::{EventLoop, LoopHandle};
use sctk::reexports::calloop_wayland_source::WaylandSource;
use sctk::{
    compositor::{CompositorHandler, CompositorState},
    dmabuf::{DmabufFeedback, DmabufHandler, DmabufState},
    delegate_compositor, delegate_keyboard, delegate_output, delegate_pointer, delegate_registry,
    delegate_seat, delegate_shm, delegate_xdg_shell, delegate_xdg_window,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers},
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
        Capability, SeatHandler, SeatState,
    },
    shell::{
        xdg::{
            window::{Window, WindowConfigure, WindowDecorations, WindowHandler},
            XdgShell,
        },
        WaylandSurface,
    },
    shm::{
        slot::{Buffer, SlotPool},
        Shm, ShmHandler,
    },
};
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_buffer, wl_keyboard, wl_output, wl_pointer, wl_seat, wl_shm, wl_surface},
    Connection, Proxy, QueueHandle, WEnum,
};
use wayland_protocols::wp::linux_dmabuf::zv1::client::{
    zwp_linux_buffer_params_v1::{self, ZwpLinuxBufferParamsV1},
    zwp_linux_dmabuf_feedback_v1::ZwpLinuxDmabufFeedbackV1,
};

fn main() {
    env_logger::init();

    let conn = Connection::connect_to_env().unwrap();

    let (globals, event_queue) = registry_queue_init(&conn).unwrap();
    let qh = event_queue.handle();
    let mut event_loop: EventLoop<AppData> =
        EventLoop::try_new().expect("Failed to initialize the event loop!");
    let loop_handle = event_loop.handle();
    WaylandSource::new(conn.clone(), event_queue).insert(loop_handle).unwrap();

    let compositor = CompositorState::bind(&globals, &qh).expect("wl_compositor not available");
    let xdg_shell = XdgShell::bind(&globals, &qh).expect("xdg shell is not available");
    let shm = Shm::bind(&globals, &qh).expect("wl shm is not available.");

    let surface = compositor.create_surface(&qh);

    let window = xdg_shell.create_window(surface.clone(), WindowDecorations::RequestServer, &qh);
    window.set_title("A wayland window");
    window.set_app_id("io.github.smithay.client-toolkit.AppData");
    window.set_min_size(Some((256, 256)));
    window.commit();

    let pool = SlotPool::new(256 * 256 * 4, &shm).expect("Failed to create pool");

    let dmabuf_state = DmabufState::new(&globals, &qh);
    dmabuf_state.get_surface_feedback(&surface, &qh);
    // XXX get_surface_feeedback

    let mut simple_window = AppData {
        registry_state: RegistryState::new(&globals),
        seat_state: SeatState::new(&globals, &qh),
        output_state: OutputState::new(&globals, &qh),
        shm,
        dmabuf_state,
        screencopy_state: ScreencopyState::new(&globals, &qh),

        exit: false,
        first_configure: true,
        pool,
        width: 256,
        height: 256,
        shift: None,
        buffer: None,
        window,
        keyboard: None,
        keyboard_focus: false,
        pointer: None,
        loop_handle: event_loop.handle(),
    };

    loop {
        event_loop.dispatch(Duration::from_millis(16), &mut simple_window).unwrap();

        if simple_window.exit {
            println!("exiting example");
            break;
        }
    }
}

struct AppData {
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    shm: Shm,
    dmabuf_state: DmabufState,
    screencopy_state: ScreencopyState,

    exit: bool,
    first_configure: bool,
    pool: SlotPool,
    width: u32,
    height: u32,
    shift: Option<u32>,
    buffer: Option<Buffer>,
    window: Window,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    keyboard_focus: bool,
    pointer: Option<wl_pointer::WlPointer>,
    loop_handle: LoopHandle<'static, AppData>,
}

impl CompositorHandler for AppData {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
        // Not needed for this example.
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
        // Not needed for this example.
    }

    fn frame(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        self.draw(conn, qh);
    }
}

impl OutputHandler for AppData {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
}

impl WindowHandler for AppData {
    fn request_close(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &Window) {
        self.exit = true;
    }

    fn configure(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        _window: &Window,
        configure: WindowConfigure,
        _serial: u32,
    ) {
        println!("Window configured to: {:?}", configure);

        self.buffer = None;
        self.width = configure.new_size.0.map(|v| v.get()).unwrap_or(256);
        self.height = configure.new_size.1.map(|v| v.get()).unwrap_or(256);

        // Initiate the first draw.
        if self.first_configure {
            self.first_configure = false;
            self.draw(conn, qh);
        }
    }
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
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            println!("Set keyboard capability");
            let keyboard = self
                .seat_state
                .get_keyboard_with_repeat(
                    qh,
                    &seat,
                    None,
                    self.loop_handle.clone(),
                    Box::new(|_state, _wl_kbd, event| {
                        println!("Repeat: {:?} ", event);
                    }),
                )
                .expect("Failed to create keyboard");

            self.keyboard = Some(keyboard);
        }

        if capability == Capability::Pointer && self.pointer.is_none() {
            println!("Set pointer capability");
            let pointer = self.seat_state.get_pointer(qh, &seat).expect("Failed to create pointer");
            self.pointer = Some(pointer);
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.keyboard.is_some() {
            println!("Unset keyboard capability");
            self.keyboard.take().unwrap().release();
        }

        if capability == Capability::Pointer && self.pointer.is_some() {
            println!("Unset pointer capability");
            self.pointer.take().unwrap().release();
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl KeyboardHandler for AppData {
    fn enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        surface: &wl_surface::WlSurface,
        _: u32,
        _: &[u32],
        keysyms: &[Keysym],
    ) {
        if self.window.wl_surface() == surface {
            println!("Keyboard focus on window with pressed syms: {keysyms:?}");
            self.keyboard_focus = true;
        }
    }

    fn leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        surface: &wl_surface::WlSurface,
        _: u32,
    ) {
        if self.window.wl_surface() == surface {
            println!("Release keyboard focus on window");
            self.keyboard_focus = false;
        }
    }

    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        println!("Key press: {event:?}");
    }

    fn release_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        println!("Key release: {event:?}");
    }

    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _serial: u32,
        modifiers: Modifiers,
    ) {
        println!("Update modifiers: {modifiers:?}");
    }
}

impl PointerHandler for AppData {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        use PointerEventKind::*;
        for event in events {
            // Ignore events for other surfaces
            if &event.surface != self.window.wl_surface() {
                continue;
            }

            match event.kind {
                Enter { .. } => {
                    println!("Pointer entered @{:?}", event.position);
                }
                Leave { .. } => {
                    println!("Pointer left");
                }
                Motion { .. } => {}
                Press { button, .. } => {
                    println!("Press {:x} @ {:?}", button, event.position);
                    self.shift = self.shift.xor(Some(0));
                }
                Release { button, .. } => {
                    println!("Release {:x} @ {:?}", button, event.position);
                }
                Axis { horizontal, vertical, .. } => {
                    println!("Scroll H:{horizontal:?}, V:{vertical:?}");
                }
            }
        }
    }
}

impl ShmHandler for AppData {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl AppData {
    pub fn draw(&mut self, _conn: &Connection, qh: &QueueHandle<Self>) {
        let width = self.width;
        let height = self.height;
        let stride = self.width as i32 * 4;

        let buffer = self.buffer.get_or_insert_with(|| {
            self.pool
                .create_buffer(width as i32, height as i32, stride, wl_shm::Format::Argb8888)
                .expect("create buffer")
                .0
        });

        let canvas = match self.pool.canvas(buffer) {
            Some(canvas) => canvas,
            None => {
                // This should be rare, but if the compositor has not released the previous
                // buffer, we need double-buffering.
                let (second_buffer, canvas) = self
                    .pool
                    .create_buffer(
                        self.width as i32,
                        self.height as i32,
                        stride,
                        wl_shm::Format::Argb8888,
                    )
                    .expect("create buffer");
                *buffer = second_buffer;
                canvas
            }
        };

        // Draw to the window:
        {
            let shift = self.shift.unwrap_or(0);
            canvas.chunks_exact_mut(4).enumerate().for_each(|(index, chunk)| {
                let x = ((index + shift as usize) % width as usize) as u32;
                let y = (index / width as usize) as u32;

                let a = 0xFF;
                let r = u32::min(((width - x) * 0xFF) / width, ((height - y) * 0xFF) / height);
                let g = u32::min((x * 0xFF) / width, ((height - y) * 0xFF) / height);
                let b = u32::min(((width - x) * 0xFF) / width, (y * 0xFF) / height);
                let color = (a << 24) + (r << 16) + (g << 8) + b;

                let array: &mut [u8; 4] = chunk.try_into().unwrap();
                *array = color.to_le_bytes();
            });

            if let Some(shift) = &mut self.shift {
                *shift = (*shift + 1) % width;
            }
        }

        // Damage the entire window
        self.window.wl_surface().damage_buffer(0, 0, self.width as i32, self.height as i32);

        // Request our next frame
        self.window.wl_surface().frame(qh, self.window.wl_surface().clone());

        // Attach and commit to present.
        buffer.attach_to(self.window.wl_surface()).expect("buffer attach");
        self.window.commit();
    }
}

impl DmabufHandler for AppData {
    fn dmabuf_state(&mut self) -> &mut DmabufState {
        &mut self.dmabuf_state
    }
    fn dmabuf_feedback(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        proxy: &ZwpLinuxDmabufFeedbackV1,
        feedback: DmabufFeedback,
    ) {
        dbg!(feedback);
        // XXX main_device?
        // Or get feedback for subsurface?
    }
    fn created(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        params: &ZwpLinuxBufferParamsV1,
        buffer: wl_buffer::WlBuffer,
    ) {
    }
    fn failed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        params: &ZwpLinuxBufferParamsV1,
    ) {
    }
    fn released(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        buffer: &wl_buffer::WlBuffer,
    ) {
    }
}

impl ScreencopyHandler for AppData {
    fn screencopy_state(&mut self) -> &mut ScreencopyState {
        &mut self.screencopy_state
    }

    fn init_done(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        session: &zcosmic_screencopy_session_v1::ZcosmicScreencopySessionV1,
        buffer_infos: &[BufferInfo],
    ) {
        let buffer_info = buffer_infos
            .iter()
            .find(|x| {
                x.type_ == WEnum::Value(zcosmic_screencopy_session_v1::BufferType::Dmabuf)
                    && x.format == wl_shm::Format::Abgr8888.into()
            })
            .unwrap();
        let session_data = SessionData::for_session(session).unwrap();
        let gbm = session_data.device.lock().unwrap();
        let buffer = gbm
            .create_buffer_object::<()>(
                buffer_info.width,
                buffer_info.height,
                gbm::Format::Abgr8888,
                gbm::BufferObjectFlags::LINEAR,
            )
            .unwrap();
        let fd = buffer.fd().unwrap();
        let mut params = self.dmabuf_state.create_params(qh).unwrap();
        params.add(
            fd.as_fd(),
            0,
            buffer.offset(0).unwrap(),
            buffer.stride().unwrap(),
            buffer.modifier().unwrap().into(),
        );
        let (wl_buffer, _) = params.create_immed(
            buffer_info.width as i32,
            buffer_info.height as i32,
            gbm::Format::Abgr8888 as u32,
            zwp_linux_buffer_params_v1::Flags::empty(),
            qh,
        );
        session.attach_buffer(&wl_buffer, None, 0);
        session.commit(zcosmic_screencopy_session_v1::Options::empty());
        dbg!(buffer_info);
    }

    fn ready(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        session: &zcosmic_screencopy_session_v1::ZcosmicScreencopySessionV1,
    ) {
    }

    fn failed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        session: &zcosmic_screencopy_session_v1::ZcosmicScreencopySessionV1,
        reason: WEnum<zcosmic_screencopy_session_v1::FailureReason>,
    ) {
    }
}

struct SessionData {
    session_data: ScreencopySessionData,
    device: Arc<Mutex<gbm::Device<fs::File>>>,
}

impl SessionData {
    pub fn for_session(
        session: &zcosmic_screencopy_session_v1::ZcosmicScreencopySessionV1,
    ) -> Option<&Self> {
        Some(session.data::<SessionData>()?)
    }
}

impl ScreencopySessionDataExt for SessionData {
    fn screencopy_session_data(&self) -> &ScreencopySessionData {
        &self.session_data
    }
}

delegate_compositor!(AppData);
delegate_output!(AppData);
delegate_shm!(AppData);

delegate_seat!(AppData);
delegate_keyboard!(AppData);
delegate_pointer!(AppData);

delegate_xdg_shell!(AppData);
delegate_xdg_window!(AppData);

delegate_registry!(AppData);

sctk::delegate_dmabuf!(AppData);
cosmic_client_toolkit::delegate_screencopy!(AppData, session: [SessionData]);

impl ProvidesRegistryState for AppData {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState,];
}

fn gbm_devices() -> impl Iterator<Item = gbm::Device<fs::File>> {
    fs::read_dir("/dev/dri").unwrap().filter_map(|i| {
        let i = i.unwrap();
        i.metadata().unwrap().rdev();
        if str::from_utf8(i.file_name().as_bytes())
            .unwrap()
            .starts_with("card")
        {
            let file = fs::File::options()
                .read(true)
                .write(true)
                .open(i.path())
                .unwrap();
            Some(gbm::Device::new(file).unwrap())
        } else {
            None
        }
    })
}
