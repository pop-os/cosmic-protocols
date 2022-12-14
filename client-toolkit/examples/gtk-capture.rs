use cascade::cascade;
use cosmic_client_toolkit::{
    export_dmabuf::{DmabufFrame, ExportDmabufHandler, ExportDmabufState},
    toplevel_info::{ToplevelInfoHandler, ToplevelInfoState},
    workspace::{WorkspaceHandler, WorkspaceState},
};
use cosmic_protocols::export_dmabuf::v1::client::zcosmic_export_dmabuf_frame_v1;
use cosmic_protocols::toplevel_info::v1::client::zcosmic_toplevel_handle_v1;
use futures::{channel::mpsc, stream::StreamExt};
use gtk4::{gdk, glib, prelude::*};
use sctk::{
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
};
use smithay::backend::renderer::{
    gles2::Gles2Renderer,
    multigpu::{egl::EglGlesBackend, GpuManager},
};
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    sync::{Arc, Mutex},
};
use wayland_client::{
    backend::{Backend, ObjectId},
    globals::registry_queue_init,
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
    toplevel_info_state: ToplevelInfoState,
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

impl ToplevelInfoHandler for AppData {
    fn toplevel_info_state(&mut self) -> &mut ToplevelInfoState {
        &mut self.toplevel_info_state
    }

    fn new_toplevel(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        toplevel: &zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1,
    ) {
    }

    fn update_toplevel(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        toplevel: &zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1,
    ) {
    }

    fn toplevel_closed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        toplevel: &zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1,
    ) {
    }
}

// XXX: Import dmabuf into GDK's GLContext and use gdk::GLTexture?
// Maybe use a single `Paintable` that is updated every frame?
fn frame_to_texture(
    frame: DmabufFrame,
    gpu_manager: &mut GpuManager<EglGlesBackend<Gles2Renderer>>,
) -> gdk::Texture {
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
    .upcast()
}

unsafe fn frame_to_texture_direct(
    frame: DmabufFrame,
    egl_display: *const std::ffi::c_void,
    gl_context: &gdk::GLContext,
) -> gdk::Texture {
    use cosmic_client_toolkit::{egl, gl};
    let egl_image = egl::EGLImage::import_dmabuf(egl_display, &frame).unwrap();
    gl_context.make_current();
    let texture = gl::bind_eglimage_to_texture(&egl_image).unwrap();
    let bytes = gl::texture_read_pixels(texture, frame.width as i32, frame.height as i32).unwrap();
    gdk::GLContext::clear_current();
    gdk::GLTexture::with_release_func(
        gl_context,
        texture,
        frame.width as i32,
        frame.height as i32,
        move || cosmic_client_toolkit::gl::delete_texture(texture),
    )
    .upcast()
}

fn image_vbox<
    F: FnMut() -> zcosmic_export_dmabuf_frame_v1::ZcosmicExportDmabufFrameV1 + 'static,
>(
    app_data: &AppData,
    name: &str,
    egl_display: *const std::ffi::c_void,
    gl_context: Rc<RefCell<Option<gdk::GLContext>>>,
    mut capture: F,
) -> gtk4::Box {
    let picture = gtk4::Picture::new();
    let (sender, mut receiver) = mpsc::unbounded();

    let frames = app_data.frames.clone();
    glib::MainContext::default().spawn_local(glib::clone!(@strong picture => async move {
        let mut gpu_manager = GpuManager::new(EglGlesBackend::<Gles2Renderer>::default(), None).unwrap();
        loop {
            let frame = capture();
            frames.lock().unwrap().insert(frame.id(), sender.clone());
            let dmabuf = receiver.next().await.unwrap();
            //let texture = frame_to_texture(dmabuf, &mut gpu_manager);
            if let Some(gl_context) = &*gl_context.borrow() {
                let texture = unsafe { frame_to_texture_direct(dmabuf, egl_display, &gl_context) };
                picture.set_paintable(Some(&texture));
            }
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
    let egl_display = display.egl_display().unwrap().as_ptr();
    //let gl_context = display.create_gl_context().unwrap();
    let gl_context = Rc::new(RefCell::new(None));

    let conn = Connection::from_backend(unsafe { Backend::from_foreign_display(wl_display as _) });
    let (globals, mut event_queue) = registry_queue_init(&conn).unwrap();
    let qh = event_queue.handle();

    let registry_state = RegistryState::new(&globals);
    let mut app_data = AppData {
        frames: Arc::new(Mutex::new(HashMap::new())),
        output_state: OutputState::new(&globals, &qh),
        export_dmabuf_state: ExportDmabufState::new(&registry_state, &qh),
        workspace_state: WorkspaceState::new(&registry_state, &qh),
        toplevel_info_state: ToplevelInfoState::new(&registry_state, &qh),
        registry_state,
        workspaces_done: false,
    };
    while !app_data.workspaces_done {
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

    let toplevels: Vec<_> = app_data
        .toplevel_info_state
        .toplevels()
        .map(|(toplevel, toplevel_info)| (toplevel.clone(), toplevel_info.clone()))
        .collect();

    let workspace_groups = app_data.workspace_state.workspace_groups().to_owned();

    let outputs_hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 24);
    for (output, name) in outputs {
        let image_vbox = image_vbox(
            &app_data,
            &name,
            egl_display,
            gl_context.clone(),
            glib::clone!(@strong export_dmabuf_manager, @strong qh, @strong name => move || {
                export_dmabuf_manager.capture_output(0, &output, &qh, ())
            }),
        );
        outputs_hbox.append(&image_vbox);
    }

    let workspaces_hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 24);
    for group in workspace_groups {
        for workspace in &group.workspaces {
            if let Some(output) = group.output.clone() {
                let name = workspace.name.clone().unwrap();
                let workspace_handle = workspace.handle.clone();
                let image_vbox = image_vbox(
                    &app_data,
                    &name,
                    egl_display,
                    gl_context.clone(),
                    glib::clone!(@strong export_dmabuf_manager, @strong qh, @strong name => move || {
                        export_dmabuf_manager.capture_workspace(0, &workspace_handle, &output, &qh, ())
                    }),
                );
                workspaces_hbox.append(&image_vbox);
            }
        }
    }

    let toplevel_hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 24);
    for (toplevel, toplevel_info) in toplevels {
        if let Some(toplevel_info) = toplevel_info {
            let name = toplevel_info.title.clone();
            let image_vbox = image_vbox(
                &app_data,
                &name,
                egl_display,
                gl_context.clone(),
                glib::clone!(@strong export_dmabuf_manager, @strong qh => move || {
                    export_dmabuf_manager.capture_toplevel(0, &toplevel, &qh, ())
                }),
            );
            toplevel_hbox.append(&image_vbox);
        }
    }

    cascade! {
        gtk4::Window::new();
        ..set_child(Some(&cascade! {
            gtk4::Box::new(gtk4::Orientation::Vertical, 24);
            ..append(&outputs_hbox);
            ..append(&workspaces_hbox);
            ..append(&toplevel_hbox);
        }));
        ..connect_realize(glib::clone!(@strong gl_context => move |window| {
            *gl_context.borrow_mut() = Some(window.surface().create_gl_context().unwrap());
        }));
        ..connect_unrealize(glib::clone!(@strong gl_context => move |_| {
            *gl_context.borrow_mut() = None;
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
cosmic_client_toolkit::delegate_toplevel_info!(AppData);
