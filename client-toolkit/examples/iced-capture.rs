// figure out how to directly import dmabuf

use cosmic_client_toolkit::{
    screencopy::{BufferInfo, ScreencopyHandler, ScreencopyState},
    workspace::{WorkspaceGroup, WorkspaceHandler, WorkspaceState},
};
use cosmic_protocols::{
    screencopy::v1::client::{zcosmic_screencopy_manager_v1, zcosmic_screencopy_session_v1},
    workspace::v1::client::zcosmic_workspace_group_handle_v1,
};
use futures::{
    channel::mpsc,
    stream::{Stream, StreamExt},
};
use iced::{
    widget::{self, image},
    Application,
};
use nix::sys::memfd;
use sctk::{
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    shm::{raw::RawPool, ShmHandler, ShmState},
};
use std::{
    collections::HashMap,
    ffi::CStr,
    sync::{Arc, Mutex},
};
use wayland_client::{
    backend::ObjectId,
    globals::registry_queue_init,
    protocol::{wl_buffer, wl_output, wl_shm, wl_shm_pool},
    Connection, Dispatch, Proxy, QueueHandle, WEnum,
};

fn empty_image() -> image::Handle {
    image::Handle::from_pixels(1, 1, vec![0, 0, 0, 255])
}

fn screencopy_stream<
    F: FnMut() -> zcosmic_screencopy_session_v1::ZcosmicScreencopySessionV1 + Send + Sync + 'static,
>(
    frames: Arc<Mutex<HashMap<ObjectId, Frame>>>,
    mut capture: F,
) -> impl Stream<Item = image::Handle> + Unpin + Send + Sync {
    let (sender, receiver) = mpsc::unbounded();

    let frame = capture();
    frames.lock().unwrap().insert(
        frame.id(),
        Frame {
            buffer: None,
            sender: sender.clone(),
            first_frame: true,
        },
    );
    Box::pin(receiver.filter_map(move |image| {
        let mut frames = frames.lock().unwrap();
        let frame = capture();
        frames.insert(
            frame.id(),
            Frame {
                buffer: None,
                sender: sender.clone(),
                first_frame: false,
            },
        );
        async { image }
    }))
}

struct Frame {
    buffer: Option<(RawPool, wl_buffer::WlBuffer)>,
    sender: mpsc::UnboundedSender<Option<image::Handle>>,
    first_frame: bool,
}

struct AppData {
    frames: Arc<Mutex<HashMap<ObjectId, Frame>>>,
    registry_state: RegistryState,
    output_state: OutputState,
    screencopy_state: ScreencopyState,
    workspace_state: WorkspaceState,
    workspaces_done: bool,
    shm_state: ShmState,
}

impl ProvidesRegistryState for AppData {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    sctk::registry_handlers!(OutputState,);
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

impl ShmHandler for AppData {
    fn shm_state(&mut self) -> &mut ShmState {
        &mut self.shm_state
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
        // XXX
        let buffer_info = buffer_infos
            .iter()
            .find(|x| {
                x.type_ == WEnum::Value(zcosmic_screencopy_session_v1::BufferType::WlShm)
                    && x.format == wl_shm::Format::Abgr8888.into()
            })
            .unwrap();
        let buf_len = buffer_info.stride * buffer_info.height;

        let mut pool = RawPool::new(buf_len as usize, &self.shm_state).unwrap();
        let buffer = pool.create_buffer(
            0,
            buffer_info.width as i32,
            buffer_info.height as i32,
            buffer_info.stride as i32,
            wl_shm::Format::Abgr8888,
            (),
            qh,
        );

        let mut frames = self.frames.lock().unwrap();
        let mut frame = frames.get_mut(&session.id()).unwrap();

        session.attach_buffer(&buffer, None, 0); // XXX age?
        if frame.first_frame {
            session.commit(zcosmic_screencopy_session_v1::Options::empty());
        } else {
            session.commit(zcosmic_screencopy_session_v1::Options::OnDamage);
        }
        conn.flush().unwrap();

        frame.buffer = Some((pool, buffer));
    }

    fn ready(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        session: &zcosmic_screencopy_session_v1::ZcosmicScreencopySessionV1,
    ) {
        let mut frame = self.frames.lock().unwrap().remove(&session.id()).unwrap();
        let (pool, buffer) = frame.buffer.as_mut().unwrap();
        buffer.destroy();
        let image = image::Handle::from_pixels(1920, 1080, pool.mmap().to_vec());
        let _ = frame.sender.unbounded_send(Some(image));
    }

    fn failed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        session: &zcosmic_screencopy_session_v1::ZcosmicScreencopySessionV1,
        reason: WEnum<zcosmic_screencopy_session_v1::FailureReason>,
    ) {
        if let Some(frame) = self.frames.lock().unwrap().remove(&session.id()) {
            let _ = frame.sender.unbounded_send(None);
        }
    }
}

impl WorkspaceHandler for AppData {
    fn workspace_state(&mut self) -> &mut WorkspaceState {
        &mut self.workspace_state
    }

    fn done(&mut self) {
        self.workspaces_done = true;
    }
}

struct Flags {
    conn: Connection,
    outputs: Vec<(wl_output::WlOutput, String)>,
    workspace_groups: Vec<WorkspaceGroup>,
    screencopy_manager: zcosmic_screencopy_manager_v1::ZcosmicScreencopyManagerV1,
    frames: Arc<Mutex<HashMap<ObjectId, Frame>>>,
    qh: QueueHandle<AppData>,
}

#[derive(Debug)]
enum Message {
    Image((ObjectId, image::Handle)),
}

struct App {
    flags: Flags,
    images: HashMap<ObjectId, image::Handle>,
}

impl iced::Application for App {
    type Message = Message;
    type Executor = iced::executor::Default;
    type Flags = Flags;
    type Theme = iced::Theme;

    fn new(flags: Flags) -> (Self, iced::Command<Message>) {
        (
            Self {
                flags,
                images: HashMap::new(),
            },
            iced::Command::none(),
        )
    }

    fn title(&self) -> String {
        "Dmabuf Capture".to_string()
    }

    fn update(&mut self, message: Message) -> iced::Command<Message> {
        match message {
            Message::Image((id, image)) => {
                self.images.insert(id, image);
            }
        };
        iced::Command::none()
    }

    fn view(&self) -> iced::Element<Message> {
        widget::column![
            widget::Row::with_children(
                self.flags
                    .outputs
                    .iter()
                    .map(|(output, name)| {
                        widget::column! {
                            widget::text(name),
                            widget::image(self.images.get(&output.id()).cloned().unwrap_or_else(empty_image)).width(iced::Length::Fill)
                        }.width(iced::Length::Fill)
                        .into()
                    })
                    .collect()
            )
            .spacing(24)
            .width(iced::Length::Fill),
            widget::Row::with_children(
                self.flags
                    .workspace_groups
                    .iter()
                    .flat_map(|group| &group.workspaces)
                    .map(|workspace| {
                        widget::column! {
                            widget::text(workspace.name.as_ref().unwrap()),
                            widget::image(self.images.get(&workspace.handle.id()).cloned().unwrap_or_else(empty_image))
                        }.width(iced::Length::Fill)
                        .into()
                    })
                    .collect()
            )
            .spacing(24)
        ]
        .spacing(24)
        .width(iced::Length::Fill)
        .into()
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        let output_img_stream = futures::stream::select_all::select_all(
            self.flags.outputs.iter().map(|(output, _)| {
                let screencopy_manager = self.flags.screencopy_manager.clone();
                let output = output.clone();
                let qh = self.flags.qh.clone();
                let id = output.id();
                let conn = self.flags.conn.clone();
                screencopy_stream(self.flags.frames.clone(), move || {
                    let frame = screencopy_manager.capture_output(
                        &output,
                        zcosmic_screencopy_manager_v1::CursorMode::Hidden,
                        &qh,
                        Default::default(),
                    );
                    let _ = conn.flush(); // XXX
                    frame
                })
                .map(move |img| (id.clone(), img))
            }),
        );
        let workspace_img_stream = futures::stream::select_all::select_all(
            self.flags.workspace_groups.iter().flat_map(|group| {
                group.workspaces.iter().filter_map(|workspace| {
                    group.output.clone().map(|output| {
                        let screencopy_manager = self.flags.screencopy_manager.clone();
                        let workspace_handle = workspace.handle.clone();
                        let qh = self.flags.qh.clone();
                        let id = workspace_handle.id();
                        let conn = self.flags.conn.clone();
                        screencopy_stream(self.flags.frames.clone(), move || {
                            let frame = screencopy_manager.capture_workspace(
                                &workspace_handle,
                                &output,
                                zcosmic_screencopy_manager_v1::CursorMode::Hidden,
                                &qh,
                                Default::default(),
                            );
                            let _ = conn.flush(); // XXX
                            frame
                        })
                        .map(move |img| (id.clone(), img))
                    })
                })
            }),
        );
        let output_subscription =
            iced::subscription::run("output-img", output_img_stream.map(Message::Image));
        let workspace_subscription =
            iced::subscription::run("workspace-img", workspace_img_stream.map(Message::Image));
        iced::Subscription::batch([output_subscription, workspace_subscription])
    }
}

fn main() {
    // TODO: Get raw window handle Iced is using?
    let conn = Connection::connect_to_env().unwrap();
    let (globals, mut event_queue) = registry_queue_init(&conn).unwrap();
    let qh = event_queue.handle();

    let registry_state = RegistryState::new(&globals);
    let mut app_data = AppData {
        frames: Arc::new(Mutex::new(HashMap::new())),
        output_state: OutputState::new(&globals, &qh),
        screencopy_state: ScreencopyState::new(&globals, &qh),
        workspace_state: WorkspaceState::new(&registry_state, &qh),
        shm_state: ShmState::bind(&globals, &qh).unwrap(),
        registry_state,
        workspaces_done: false,
    };
    while !app_data.workspaces_done {
        event_queue.blocking_dispatch(&mut app_data).unwrap();
    }
    event_queue.roundtrip(&mut app_data).unwrap();

    let screencopy_manager = app_data.screencopy_state.screencopy_manager.clone();

    // XXX update as outputs added/removed
    let outputs: Vec<_> = app_data
        .output_state
        .outputs()
        .map(|output| {
            let info = app_data.output_state.info(&output).unwrap();
            (output, info.name.unwrap().to_string())
        })
        .collect();

    let workspace_groups = app_data.workspace_state.workspace_groups().to_owned();
    let frames = app_data.frames.clone();

    std::thread::spawn(move || loop {
        event_queue.blocking_dispatch(&mut app_data).unwrap();
    });

    App::run(iced::Settings::with_flags(Flags {
        conn,
        outputs,
        workspace_groups,
        screencopy_manager,
        frames,
        qh,
    }))
    .unwrap();
}

impl Dispatch<wl_buffer::WlBuffer, ()> for AppData {
    fn event(
        _app_data: &mut Self,
        buffer: &wl_buffer::WlBuffer,
        event: wl_buffer::Event,
        _: &(),
        _: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

sctk::delegate_output!(AppData);
sctk::delegate_registry!(AppData);
sctk::delegate_shm!(AppData);
cosmic_client_toolkit::delegate_screencopy!(AppData);
cosmic_client_toolkit::delegate_workspace!(AppData);
