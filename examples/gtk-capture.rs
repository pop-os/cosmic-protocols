// TODO: sctk dispatch implementation? Helper there for globals?
// Use same abstraction in an iced example

use cosmic_protocols::export_dmabuf::v1::client::{
    zcosmic_export_dmabuf_frame_v1, zcosmic_export_dmabuf_manager_v1,
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
    outputs: Vec<(wl_output::WlOutput, String)>,
    frames: HashMap<String, (async_channel::Sender<DmaBufFrame>, DmaBufFrame)>,
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

impl Dispatch<zcosmic_export_dmabuf_frame_v1::ZcosmicExportDmabufFrameV1, String> for AppData {
    fn event(
        app_data: &mut Self,
        _: &zcosmic_export_dmabuf_frame_v1::ZcosmicExportDmabufFrameV1,
        event: zcosmic_export_dmabuf_frame_v1::Event,
        output_name: &String,
        _: &Connection,
        _: &QueueHandle<AppData>,
    ) {
        let (sender, frame) = app_data.frames.get_mut(output_name).unwrap();

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

    let window = gtk4::Window::new();
    let hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 24);
    for (output, name) in outputs.clone() {
        let vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        vbox.append(&gtk4::Label::new(Some(&name)));
        vbox.set_hexpand(true);
        let picture = gtk4::Picture::new();
        vbox.append(&picture);
        hbox.append(&vbox);

        let (sender, receiver) = async_channel::unbounded();
        app_data
            .frames
            .insert(name.to_string(), (sender, DmaBufFrame::default()));

        let qh = qh.clone();
        let mut gpu_manager = GpuManager::new(EglGlesBackend, None).unwrap();
        let dmabuf_manager = app_data.export_dmabuf_manager.clone().unwrap();
        glib::MainContext::default().spawn_local(async move {
            loop {
                dmabuf_manager.capture_output(0, &output, &qh, name.to_string());
                let frame = receiver.recv().await.unwrap();
                let texture = frame_to_texture(frame, &mut gpu_manager);
                picture.set_paintable(Some(&texture));
            }
        });
    }
    window.set_child(Some(&hbox));
    window.show();

    // XXX No success using `poll_dispatch_pending`?
    // XXX Busy loop?
    std::thread::spawn(move || loop {
        event_queue.dispatch_pending(&mut app_data).unwrap();
    });

    glib::MainLoop::new(None, false).run();
}
