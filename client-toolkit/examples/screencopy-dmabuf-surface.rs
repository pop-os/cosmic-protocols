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
    registry::{ProvidesRegistryState, RegistryState, SimpleGlobal},
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
    protocol::{wl_buffer, wl_keyboard, wl_output, wl_pointer, wl_seat, wl_shm, wl_surface, wl_subsurface},
    Connection, Proxy, QueueHandle, WEnum, Dispatch,
};
use wayland_protocols::wp::linux_dmabuf::zv1::client::{
    zwp_linux_buffer_params_v1::{self, ZwpLinuxBufferParamsV1},
    zwp_linux_dmabuf_feedback_v1::ZwpLinuxDmabufFeedbackV1,
};
use wayland_protocols::wp::viewporter::client::{
    wp_viewport::{self, WpViewport},
    wp_viewporter::{self, WpViewporter},
};

struct Subsurface {
    wl_surface: wl_surface::WlSurface,
    wl_subsurface: wl_subsurface::WlSubsurface,
    wp_viewport: WpViewport,
}

impl Subsurface {
    // XXX preserve ratio?
    fn set_position(&self, x: i32, y: i32, width: i32, height: i32) {
        self.wl_subsurface.set_position(x, y);
        self.wp_viewport.set_destination(width, height);
    }
}

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

    let wp_viewporter = SimpleGlobal::<wp_viewporter::WpViewporter, 1>::bind(&globals, &qh).unwrap().get().unwrap().clone();

    let viewport = wp_viewporter.get_viewport(&surface, &qh, ());

    let registry_state =  RegistryState::new(&globals);

    let mut simple_window = AppData {
        seat_state: SeatState::new(&globals, &qh),
        output_state: OutputState::new(&globals, &qh),
        shm,
        dmabuf_state,
        screencopy_state: ScreencopyState::new(&globals, &qh),
        device: Mutex::new(None),
        wl_buffer: None,
        wp_viewporter,
        viewport,
        workspace_state: WorkspaceState::new(&registry_state, &qh),
        registry_state,

        exit: false,
        first_configure: true,
        pool,
        width: 256,
        height: 256,
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
    device: Mutex<Option<gbm::Device<fs::File>>>,
    wl_buffer: Option<wl_buffer::WlBuffer>,
    wp_viewporter: WpViewporter,
    viewport: WpViewport,
    workspace_state: WorkspaceState,

    exit: bool,
    first_configure: bool,
    pool: SlotPool,
    width: u32,
    height: u32,
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
    }

    fn leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        surface: &wl_surface::WlSurface,
        _: u32,
    ) {
    }

    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
    }

    fn release_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
    }

    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _serial: u32,
        modifiers: Modifiers,
    ) {
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

        canvas.chunks_exact_mut(4).enumerate().for_each(|(index, chunk)| {
            let array: &mut [u8; 4] = chunk.try_into().unwrap();
            *array = [0, 0, 0, 255];
        });

        self.window.wl_surface().damage_buffer(0, 0, self.width as i32, self.height as i32);

        self.window.wl_surface().frame(qh, self.window.wl_surface().clone());

        if let Some(wl_buffer) = self.wl_buffer.as_ref() {
            self.viewport.set_destination(self.width as i32, self.height as i32);
            self.window.wl_surface().attach(Some(wl_buffer), 0, 0);
        } else {
            buffer.attach_to(self.window.wl_surface()).expect("buffer attach");
        }
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
        // TODO device per surface; multi-monitor. per sub-surface?
        // - so, should be using device that output is rendered with? but that can change?
        let mut device = self.device.lock().unwrap();
        if device.is_none() {
            *device = Some(gbm_device(feedback.main_device()).unwrap()); // XXX

            let output = self.output_state.outputs().next().unwrap();
            self.screencopy_state.screencopy_manager.capture_output(
                &output,
                zcosmic_screencopy_manager_v1::CursorMode::Hidden,
                &qh,
                SessionData {
                    session_data: ScreencopySessionData::default(),
                    buffer: Mutex::new(None),
                },
            );

            println!("CAPTURED");
        }
        //dbg!(feedback);
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
        let mut gbm = self.device.lock().unwrap();
        let gbm = gbm.as_mut().unwrap();
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
        *session_data.buffer.lock().unwrap() = Some(wl_buffer.clone());
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
        println!("READY!");
        let session_data = SessionData::for_session(session).unwrap();
        self.wl_buffer = Some(session_data.buffer.lock().unwrap().as_ref().unwrap().clone());
    }

    fn failed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        session: &zcosmic_screencopy_session_v1::ZcosmicScreencopySessionV1,
        reason: WEnum<zcosmic_screencopy_session_v1::FailureReason>,
    ) {
        println!("FAILED!");
    }
}

struct SessionData {
    session_data: ScreencopySessionData,
    buffer: Mutex<Option<wl_buffer::WlBuffer>>,
    // device: Arc<Mutex<gbm::Device<fs::File>>>,
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

impl WorkspaceHandler for AppData {
    fn workspace_state(&mut self) -> &mut WorkspaceState {
        &mut self.workspace_state
    }

    fn done(&mut self) {
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
cosmic_client_toolkit::delegate_workspace!(AppData);

impl ProvidesRegistryState for AppData {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState,];
}

// XXX
fn gbm_device(rdev: u64) -> Option<gbm::Device<fs::File>> {
    fs::read_dir("/dev/dri").unwrap().find_map(|i| {
        let i = i.unwrap();
        if i.metadata().unwrap().rdev() == rdev {
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

sctk::delegate_simple!(AppData, WpViewporter, 1);

impl Dispatch<WpViewport, ()> for AppData {
    fn event(
        _: &mut AppData,
        _: &WpViewport,
        _: wp_viewport::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<AppData>,
    ) {
        unreachable!("wp_viewport::Event is empty in version 1")
    }
}
