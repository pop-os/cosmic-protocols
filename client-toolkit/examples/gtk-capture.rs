// Use same abstraction in an iced example

use cascade::cascade;
use cosmic_client_toolkit::{
    export_dmabuf::{DmabufFrame, ExportDmabufHandler, ExportDmabufState},
    workspace::{WorkspaceHandler, WorkspaceState},
};
use cosmic_protocols::export_dmabuf::v1::client::zcosmic_export_dmabuf_frame_v1;
use futures::{channel::mpsc, stream::StreamExt};
use gtk4::{gdk, glib, prelude::*};
use sctk::{
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
};
use smithay::backend::renderer::multigpu::{egl::EglGlesBackend, GpuManager};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use wayland_client::{
    backend::{Backend, ObjectId},
    protocol::wl_output,
    Connection, Proxy, QueueHandle,
};

struct AppData {
    frames: Arc<Mutex<HashMap<ObjectId, mpsc::UnboundedSender<DmabufFrame>>>>,
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
        self.frames
            .lock()
            .unwrap()
            .remove(&frame.id())
            .unwrap()
            .unbounded_send(dmabuf)
            .unwrap();
    }

    fn frame_cancel(&mut self, frame: &zcosmic_export_dmabuf_frame_v1::ZcosmicExportDmabufFrameV1) {
        self.frames.lock().unwrap().remove(&frame.id());
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

// XXX: Import dmabuf into GDK's GLContext and use gdk::GLTexture? GDK doesn't seem to expose
// EGLContext.
// Maybe use a single `Paintable` that is updated every frame?
fn frame_to_texture(
    frame: DmabufFrame,
    gpu_manager: &mut GpuManager<EglGlesBackend>,
) -> gdk::MemoryTexture {
    let width = frame.width as i32;
    let height = frame.height as i32;
    let bytes = frame.import_to_bytes(gpu_manager);
    gdk::GLContext::clear_current();
    gdk::MemoryTexture::new(
        width,
        height,
        gdk::MemoryFormat::R8g8b8a8,
        &glib::Bytes::from_owned(bytes),
        width as usize * 4,
    )
}

fn image_vbox<
    F: FnMut() -> zcosmic_export_dmabuf_frame_v1::ZcosmicExportDmabufFrameV1 + 'static,
>(
    app_data: &AppData,
    name: &str,
    mut capture: F,
) -> gtk4::Box {
    let picture = gtk4::Picture::new();
    let (sender, mut receiver) = mpsc::unbounded();

    let frames = app_data.frames.clone();
    glib::MainContext::default().spawn_local(glib::clone!(@strong picture => async move {
        let mut gpu_manager = GpuManager::new(EglGlesBackend, None).unwrap();
        loop {
            let frame = capture();
            frames.lock().unwrap().insert(frame.id(), sender.clone());
            let dmabuf = receiver.next().await.unwrap();
            let texture = frame_to_texture(dmabuf, &mut gpu_manager);
            picture.set_paintable(Some(&texture));
        }
    }));

    cascade! {
        gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        ..set_hexpand(true);
        ..append(&gtk4::Label::new(Some(&name)));
        ..append(&picture);
    }
}

fn main() {
    env_logger::init();
    gtk4::init().unwrap();

    let display = gdk::Display::default()
        .unwrap()
        .downcast::<gdk4_wayland::WaylandDisplay>()
        .unwrap();
    let wl_display = display.wl_display().c_ptr();
    let connection =
        Connection::from_backend(unsafe { Backend::from_foreign_display(wl_display as _) });
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
        .clone()
        .unwrap();

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

    let outputs_hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 24);
    for (output, name) in outputs {
        let image_vbox = image_vbox(
            &app_data,
            &name,
            glib::clone!(@strong export_dmabuf_manager, @strong qh, @strong name => move || {
                export_dmabuf_manager.capture_output(0, &output, &qh, ())
            }),
        );
        outputs_hbox.append(&image_vbox);
    }

    let workspaces_hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 24);
    for (_, group_info) in workspace_groups {
        for (workspace, workspace_info) in &group_info.workspaces {
            let name = workspace_info.name.clone().unwrap();
            if let Some(output) = group_info.output.clone() {
                let image_vbox = image_vbox(
                    &app_data,
                    &name,
                    glib::clone!(@strong export_dmabuf_manager, @strong workspace, @strong qh, @strong name => move || {
                        export_dmabuf_manager.capture_workspace(0, &workspace, &output, &qh, ())
                    }),
                );
                workspaces_hbox.append(&image_vbox);
            }
        }
    }

    cascade! {
        gtk4::Window::new();
        ..set_child(Some(&cascade! {
            gtk4::Box::new(gtk4::Orientation::Vertical, 24);
            ..append(&outputs_hbox);
            ..append(&workspaces_hbox);
        }));
        ..show();
    };

    // XXX Should it be possible to use `poll_dispatch_pending`?
    std::thread::spawn(move || loop {
        event_queue.blocking_dispatch(&mut app_data).unwrap();
    });

    glib::MainLoop::new(None, false).run();
}

sctk::delegate_output!(AppData);
sctk::delegate_registry!(AppData);
cosmic_client_toolkit::delegate_export_dmabuf!(AppData);
cosmic_client_toolkit::delegate_workspace!(AppData);
