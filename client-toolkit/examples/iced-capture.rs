// Subscription for images?
// figure out how to directly import dmabuf

use cosmic_client_toolkit::{
    export_dmabuf::{DmabufFrame, ExportDmabufHandler, ExportDmabufState},
    workspace::{WorkspaceGroup, WorkspaceHandler, WorkspaceState},
};
use cosmic_protocols::{
    export_dmabuf::v1::client::{zcosmic_export_dmabuf_frame_v1, zcosmic_export_dmabuf_manager_v1},
    workspace::v1::client::zcosmic_workspace_group_handle_v1,
};
use futures::{
    channel::mpsc,
    stream::{Stream, StreamExt},
};
use iced::{widget, Application};
use sctk::{
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
};
use smithay::backend::renderer::multigpu::{egl::EglGlesBackend, GpuManager};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use wayland_client::{backend::ObjectId, protocol::wl_output, Connection, Proxy, QueueHandle};

fn empty_image() -> iced::widget::image::Handle {
    // XXX BGRA?
    iced::widget::image::Handle::from_pixels(1, 1, vec![0, 0, 0, 255])
}

// XXX don't create `gpu_manager` every frame; had issues with it not being `Send`, and EGL
fn dmabuf_to_image(dmabuf: DmabufFrame) -> iced::widget::image::Handle {
    // error
    let mut gpu_manager = GpuManager::new(EglGlesBackend, None).unwrap();
    let width = dmabuf.width;
    let height = dmabuf.height;
    let bytes = dmabuf.import_to_bytes(&mut gpu_manager);
    iced::widget::image::Handle::from_pixels(width, height, bytes)
}

fn export_dmabuf_stream<
    F: FnMut() -> zcosmic_export_dmabuf_frame_v1::ZcosmicExportDmabufFrameV1 + Send + Sync + 'static,
>(
    frames: Arc<Mutex<HashMap<ObjectId, mpsc::UnboundedSender<Option<DmabufFrame>>>>>,
    mut capture: F,
) -> impl Stream<Item = iced::widget::image::Handle> + Unpin + Send + Sync {
    let (sender, receiver) = mpsc::unbounded();

    let frame = capture();
    frames.lock().unwrap().insert(frame.id(), sender.clone());
    Box::pin(receiver.filter_map(move |dmabuf| {
        let mut frames = frames.lock().unwrap();
        let frame = capture();
        frames.insert(frame.id(), sender.clone());
        let res = dmabuf.map(|dmabuf| dmabuf_to_image(dmabuf));
        async { res }
    }))
}

struct AppData {
    frames: Arc<Mutex<HashMap<ObjectId, mpsc::UnboundedSender<Option<DmabufFrame>>>>>,
    registry_state: RegistryState,
    output_state: OutputState,
    export_dmabuf_state: ExportDmabufState,
    workspace_state: WorkspaceState,
    workspaces_done: bool,
}

impl ProvidesRegistryState for AppData {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    sctk::registry_handlers!(OutputState, ExportDmabufState, WorkspaceState,);
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

impl ExportDmabufHandler for AppData {
    fn export_dmabuf_state(&mut self) -> &mut ExportDmabufState {
        &mut self.export_dmabuf_state
    }

    fn frame_ready(
        &mut self,
        frame: &zcosmic_export_dmabuf_frame_v1::ZcosmicExportDmabufFrameV1,
        dmabuf: DmabufFrame,
    ) {
        let _ = self
            .frames
            .lock()
            .unwrap()
            .remove(&frame.id())
            .unwrap()
            .unbounded_send(Some(dmabuf));
    }

    fn frame_cancel(&mut self, frame: &zcosmic_export_dmabuf_frame_v1::ZcosmicExportDmabufFrameV1) {
        if let Some(sender) = self.frames.lock().unwrap().remove(&frame.id()) {
            let _ = sender.unbounded_send(None);
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
    connection: Connection,
    outputs: Vec<(wl_output::WlOutput, String)>,
    workspace_groups: Vec<(
        zcosmic_workspace_group_handle_v1::ZcosmicWorkspaceGroupHandleV1,
        WorkspaceGroup,
    )>,
    export_dmabuf_manager: zcosmic_export_dmabuf_manager_v1::ZcosmicExportDmabufManagerV1,
    frames: Arc<Mutex<HashMap<ObjectId, mpsc::UnboundedSender<Option<DmabufFrame>>>>>,
    qh: QueueHandle<AppData>,
}

#[derive(Debug)]
enum Message {
    Image((ObjectId, iced::widget::image::Handle)),
}

struct App {
    flags: Flags,
    images: HashMap<ObjectId, iced::widget::image::Handle>,
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
                            widget::image(self.images.get(&output.id()).cloned().unwrap_or_else(empty_image)).width(iced::Length::Fill).height(iced::Length::Fill)
                        }.width(iced::Length::Fill).height(iced::Length::Fill)
                        .into()
                    })
                    .collect()
            )
            .spacing(24)
            .width(iced::Length::Fill)
            .height(iced::Length::Fill),
            widget::Row::with_children(
                self.flags
                    .workspace_groups
                    .iter()
                    .flat_map(|(_, group_info)| &group_info.workspaces)
                    .map(|(workspace, workspace_info)| {
                        widget::column! {
                            widget::text(workspace_info.name.as_ref().unwrap()),
                            widget::image(self.images.get(&workspace.id()).cloned().unwrap_or_else(empty_image))
                        }
                        .into()
                    })
                    .collect()
            )
            .spacing(24)
        ]
        .spacing(24)
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .into()
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        let output_img_stream = futures::stream::select_all::select_all(
            self.flags.outputs.iter().map(|(output, _)| {
                let export_dmabuf_manager = self.flags.export_dmabuf_manager.clone();
                let output = output.clone();
                let qh = self.flags.qh.clone();
                let id = output.id();
                let connection = self.flags.connection.clone();
                export_dmabuf_stream(self.flags.frames.clone(), move || {
                    let frame = export_dmabuf_manager.capture_output(0, &output, &qh, ());
                    let _ = connection.flush(); // XXX
                    frame
                })
                .map(move |img| (id.clone(), img))
            }),
        );
        iced::subscription::run("output-img", output_img_stream.map(Message::Image))
    }
}

fn main() {
    // TODO: Get raw window handle Iced is using?
    let connection = Connection::connect_to_env().unwrap();
    let mut event_queue = connection.new_event_queue();
    let qh = event_queue.handle();

    let mut app_data = AppData {
        frames: Arc::new(Mutex::new(HashMap::new())),
        registry_state: RegistryState::new(&connection, &qh),
        output_state: OutputState::new(),
        export_dmabuf_state: ExportDmabufState::new(),
        workspace_state: WorkspaceState::new(),
        workspaces_done: false,
    };
    while !app_data.registry_state.ready() || !app_data.workspaces_done {
        event_queue.blocking_dispatch(&mut app_data).unwrap();
    }
    event_queue.roundtrip(&mut app_data).unwrap();

    let export_dmabuf_manager = app_data
        .export_dmabuf_state
        .export_dmabuf_manager()
        .unwrap()
        .clone();

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
        connection,
        outputs,
        workspace_groups,
        export_dmabuf_manager,
        frames,
        qh,
    }))
    .unwrap();
}

sctk::delegate_output!(AppData);
sctk::delegate_registry!(AppData);
cosmic_client_toolkit::delegate_export_dmabuf!(AppData);
cosmic_client_toolkit::delegate_workspace!(AppData);
