// TODO: sctk dispatch implementation? Helper there for globals?
// Use same abstraction in an iced example

use cascade::cascade;
use cosmic_protocols::{
    export_dmabuf::v1::client::{zcosmic_export_dmabuf_frame_v1, zcosmic_export_dmabuf_manager_v1},
    workspace::v1::client::{
        zcosmic_workspace_group_handle_v1, zcosmic_workspace_handle_v1,
        zcosmic_workspace_manager_v1,
    },
};
use gtk4::{gdk, glib, prelude::*};
use smithay::{
    backend::{
        allocator::{
            dmabuf::{Dmabuf, DmabufFlags},
            Fourcc, Modifier,
        },
        drm::node::DrmNode,
        renderer::{
            gles2::Gles2Texture,
            multigpu::{egl::EglGlesBackend, GpuManager},
            Bind, ExportMem,
        },
    },
    utils::{Point, Rectangle, Size},
};
use std::{collections::HashMap, mem, os::unix::io::OwnedFd};
use wayland_client::{
    backend::Backend,
    protocol::{wl_output, wl_registry},
    Connection, Dispatch, QueueHandle,
};

#[derive(Default, Clone)]
struct WorkspaceGroup {
    output: Option<wl_output::WlOutput>,
    workspaces: Vec<(
        zcosmic_workspace_handle_v1::ZcosmicWorkspaceHandleV1,
        Workspace,
    )>,
}

#[derive(Default, Clone)]
struct Workspace {
    name: Option<String>,
}

#[derive(Eq, Hash, PartialEq)]
enum Capture {
    Output(String),
    Workspace(String),
}

#[derive(Debug)]
struct Object {
    fd: OwnedFd,
    index: u32,
    offset: u32,
    stride: u32,
    plane_index: u32,
}

#[derive(Debug, Default)]
struct DmaBufFrame {
    node: Option<DrmNode>,
    width: u32,
    height: u32,
    objects: Vec<Object>,
    modifier: Option<Modifier>,
    format: Option<Fourcc>,
    flags: Option<DmabufFlags>,
}

#[derive(Default)]
struct AppData {
    export_dmabuf_manager: Option<zcosmic_export_dmabuf_manager_v1::ZcosmicExportDmabufManagerV1>,
    workspace_manager: Option<zcosmic_workspace_manager_v1::ZcosmicWorkspaceManagerV1>,
    outputs: Vec<(wl_output::WlOutput, String)>,
    workspace_groups: Vec<(
        zcosmic_workspace_group_handle_v1::ZcosmicWorkspaceGroupHandleV1,
        WorkspaceGroup,
    )>,
    frames: HashMap<Capture, (async_channel::Sender<DmaBufFrame>, DmaBufFrame)>,
}

impl Dispatch<wl_registry::WlRegistry, ()> for AppData {
    fn event(
        app_data: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<AppData>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version: _,
        } = event
        {
            match interface.as_str() {
                "zcosmic_export_dmabuf_manager_v1" => {
                    app_data.export_dmabuf_manager = Some(registry
                        .bind::<zcosmic_export_dmabuf_manager_v1::ZcosmicExportDmabufManagerV1, _, _>(
                            name,
                            1,
                            qh,
                            (),
                        ));
                }
                "zcosmic_workspace_manager_v1" => {
                    app_data.workspace_manager = Some(
                        registry
                            .bind::<zcosmic_workspace_manager_v1::ZcosmicWorkspaceManagerV1, _, _>(
                                name,
                                1,
                                qh,
                                (),
                            ),
                    );
                }
                "wl_output" => {
                    registry.bind::<wl_output::WlOutput, _, _>(name, 4, qh, ());
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<wl_output::WlOutput, ()> for AppData {
    fn event(
        app_data: &mut Self,
        output: &wl_output::WlOutput,
        event: wl_output::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<AppData>,
    ) {
        match event {
            wl_output::Event::Name { name } => {
                app_data.outputs.push((output.clone(), name));
            }
            _ => {}
        }
    }
}

impl Dispatch<zcosmic_workspace_manager_v1::ZcosmicWorkspaceManagerV1, ()> for AppData {
    fn event(
        app_data: &mut Self,
        _: &zcosmic_workspace_manager_v1::ZcosmicWorkspaceManagerV1,
        event: zcosmic_workspace_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<AppData>,
    ) {
        match event {
            zcosmic_workspace_manager_v1::Event::WorkspaceGroup { workspace_group } => {
                app_data
                    .workspace_groups
                    .push((workspace_group, WorkspaceGroup::default()));
            }
            _ => {}
        }
    }

    wayland_client::event_created_child!(AppData, zcosmic_workspace_manager_v1::ZcosmicWorkspaceManagerV1, [
        zcosmic_workspace_manager_v1::EVT_WORKSPACE_GROUP_OPCODE => (zcosmic_workspace_group_handle_v1::ZcosmicWorkspaceGroupHandleV1, ())
    ]);
}

impl Dispatch<zcosmic_workspace_group_handle_v1::ZcosmicWorkspaceGroupHandleV1, ()> for AppData {
    fn event(
        app_data: &mut Self,
        workspace_group: &zcosmic_workspace_group_handle_v1::ZcosmicWorkspaceGroupHandleV1,
        event: zcosmic_workspace_group_handle_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<AppData>,
    ) {
        let mut group_info = &mut app_data
            .workspace_groups
            .iter_mut()
            .find(|(group, _)| group == workspace_group)
            .unwrap()
            .1;
        match event {
            zcosmic_workspace_group_handle_v1::Event::OutputEnter { output } => {
                group_info.output = Some(output);
            }
            zcosmic_workspace_group_handle_v1::Event::Workspace { workspace } => {
                group_info
                    .workspaces
                    .push((workspace, Workspace::default()));
            }
            zcosmic_workspace_group_handle_v1::Event::Remove => {
                if let Some(idx) = app_data
                    .workspace_groups
                    .iter()
                    .position(|(group, _)| group == workspace_group)
                {
                    app_data.workspace_groups.remove(idx);
                }
            }
            _ => {}
        }
    }

    wayland_client::event_created_child!(AppData, zcosmic_workspace_group_handle_v1::ZcosmicWorkspaceGroupHandleV1, [
        zcosmic_workspace_group_handle_v1::EVT_WORKSPACE_OPCODE => (zcosmic_workspace_handle_v1::ZcosmicWorkspaceHandleV1, ())
    ]);
}

impl Dispatch<zcosmic_workspace_handle_v1::ZcosmicWorkspaceHandleV1, ()> for AppData {
    fn event(
        app_data: &mut Self,
        workspace: &zcosmic_workspace_handle_v1::ZcosmicWorkspaceHandleV1,
        event: zcosmic_workspace_handle_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<AppData>,
    ) {
        let (_, workspace_info) = app_data
            .workspace_groups
            .iter_mut()
            .find_map(|(_, group_info)| {
                group_info
                    .workspaces
                    .iter_mut()
                    .find(|(w, _)| w == workspace)
            })
            .unwrap();
        match event {
            zcosmic_workspace_handle_v1::Event::Name { name } => {
                workspace_info.name = Some(name);
            }
            zcosmic_workspace_handle_v1::Event::Remove => {
                for (_, group_info) in app_data.workspace_groups.iter_mut() {
                    if let Some(idx) = group_info
                        .workspaces
                        .iter()
                        .position(|(w, _)| w == workspace)
                    {
                        group_info.workspaces.remove(idx);
                    }
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<zcosmic_export_dmabuf_manager_v1::ZcosmicExportDmabufManagerV1, ()> for AppData {
    fn event(
        _: &mut Self,
        _: &zcosmic_export_dmabuf_manager_v1::ZcosmicExportDmabufManagerV1,
        _: zcosmic_export_dmabuf_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<AppData>,
    ) {
    }
}

impl Dispatch<zcosmic_export_dmabuf_frame_v1::ZcosmicExportDmabufFrameV1, Capture> for AppData {
    fn event(
        app_data: &mut Self,
        _: &zcosmic_export_dmabuf_frame_v1::ZcosmicExportDmabufFrameV1,
        event: zcosmic_export_dmabuf_frame_v1::Event,
        capture: &Capture,
        _: &Connection,
        _: &QueueHandle<AppData>,
    ) {
        let (sender, frame) = app_data.frames.get_mut(capture).unwrap();

        match event {
            zcosmic_export_dmabuf_frame_v1::Event::Device { ref node } => {
                let node = u64::from_ne_bytes([
                    node[0], node[1], node[2], node[3], node[4], node[5], node[6], node[7],
                ]);
                frame.node = Some(DrmNode::from_dev_id(node).unwrap());
            }
            zcosmic_export_dmabuf_frame_v1::Event::Frame {
                width,
                height,
                mod_high,
                mod_low,
                format,
                flags,
                ..
            } => {
                frame.width = width;
                frame.height = height;
                frame.format = Some(Fourcc::try_from(format).unwrap());
                frame.modifier = Some(Modifier::from(((mod_high as u64) << 32) + mod_low as u64));
                frame.flags = Some(DmabufFlags::from_bits(u32::from(flags)).unwrap());
            }
            zcosmic_export_dmabuf_frame_v1::Event::Object {
                fd,
                index,
                offset,
                stride,
                plane_index,
                ..
            } => {
                assert!(plane_index == frame.objects.last().map_or(0, |x| x.plane_index + 1));
                frame.objects.push(Object {
                    fd,
                    index,
                    offset,
                    stride,
                    plane_index,
                });
            }
            zcosmic_export_dmabuf_frame_v1::Event::Ready { .. } => {
                sender.send_blocking(mem::take(frame)).unwrap();
            }
            _ => {}
        }
    }
}

// TODO: Don't create new renderer every frame?
// XXX: Import dmabuf into GDK's GLContext and use gdk::GLTexture? GDK doesn't seem to expose
// EGLContext.
// Maybe use a single `Paintable` that is updated every frame?
fn frame_to_bytes(frame: DmaBufFrame, gpu_manager: &mut GpuManager<EglGlesBackend>) -> Vec<u8> {
    let mut builder = Dmabuf::builder(
        (frame.width as i32, frame.height as i32),
        frame.format.unwrap(),
        frame.flags.unwrap(),
    );
    for object in frame.objects {
        builder.add_plane(
            object.fd,
            object.index,
            object.offset,
            object.stride,
            frame.modifier.unwrap(),
        );
    }
    let dmabuf = builder.build().unwrap();

    let drm_node = frame.node.as_ref().unwrap();
    let mut renderer = gpu_manager
        .renderer::<Gles2Texture>(drm_node, drm_node)
        .unwrap();
    renderer.bind(dmabuf).unwrap();
    let rectangle = Rectangle {
        loc: Point::default(),
        size: Size::from((frame.width as i32, frame.height as i32)),
    };
    let mapping = renderer.copy_framebuffer(rectangle).unwrap();
    let bytes = Vec::from(renderer.map_texture(&mapping).unwrap());
    gdk::GLContext::clear_current();
    bytes
}

fn frame_to_texture(
    frame: DmaBufFrame,
    gpu_manager: &mut GpuManager<EglGlesBackend>,
) -> gdk::MemoryTexture {
    let width = frame.width as i32;
    let height = frame.height as i32;
    let bytes = frame_to_bytes(frame, gpu_manager);
    gdk::MemoryTexture::new(
        width,
        height,
        gdk::MemoryFormat::R8g8b8a8,
        &glib::Bytes::from_owned(bytes),
        width as usize * 4,
    )
}

fn image_vbox<F: FnMut() + 'static>(
    name: &str,
    mut capture: F,
) -> (gtk4::Box, async_channel::Sender<DmaBufFrame>) {
    let picture = gtk4::Picture::new();
    let (sender, receiver) = async_channel::unbounded();

    glib::MainContext::default().spawn_local(glib::clone!(@strong picture => async move {
        let mut gpu_manager = GpuManager::new(EglGlesBackend, None).unwrap();
        loop {
            capture();
            let frame = receiver.recv().await.unwrap();
            let texture = frame_to_texture(frame, &mut gpu_manager);
            picture.set_paintable(Some(&texture));
        }
    }));

    let image_vbox = cascade! {
        gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        ..set_hexpand(true);
        ..append(&gtk4::Label::new(Some(&name)));
        ..append(&picture);
    };
    (image_vbox, sender)
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
    let _registry = connection.display().get_registry(&qh, ());

    let mut app_data = AppData::default();

    event_queue.roundtrip(&mut app_data).unwrap();
    event_queue.roundtrip(&mut app_data).unwrap();

    // XXX update as outputs added/removed
    let outputs = app_data.outputs.clone();
    let workspace_groups = app_data.workspace_groups.clone();
    let dmabuf_manager = app_data.export_dmabuf_manager.clone().unwrap();

    let outputs_hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 24);
    for (output, name) in outputs {
        let (image_vbox, sender) = image_vbox(
            &name,
            glib::clone!(@strong dmabuf_manager, @strong qh, @strong name => move || {
                dmabuf_manager.capture_output(0, &output, &qh, Capture::Output(name.clone()));
            }),
        );
        app_data.frames.insert(
            Capture::Output(name.to_string()),
            (sender, DmaBufFrame::default()),
        );
        outputs_hbox.append(&image_vbox);
    }

    let workspaces_hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 24);
    for (_, group_info) in workspace_groups {
        for (workspace, workspace_info) in &group_info.workspaces {
            let name = workspace_info.name.clone().unwrap();
            if let Some(output) = group_info.output.clone() {
                let (image_vbox, sender) = image_vbox(
                    &name,
                    glib::clone!(@strong dmabuf_manager, @strong workspace, @strong qh, @strong name => move || {
                        dmabuf_manager.capture_workspace(0, &workspace, &output, &qh, Capture::Workspace(name.clone()));
                    }),
                );
                app_data.frames.insert(
                    Capture::Workspace(name.to_string()),
                    (sender, DmaBufFrame::default()),
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
