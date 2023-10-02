use cosmic_protocols::export_dmabuf::v1::client::{
    zcosmic_export_dmabuf_frame_v1, zcosmic_export_dmabuf_manager_v1,
};
use smithay::{
    backend::{
        allocator::{
            dmabuf::{Dmabuf, DmabufFlags},
            Fourcc, Modifier,
        },
        drm::node::DrmNode,
        renderer::{
            gles::GlesRenderer,
            multigpu::{egl::EglGlesBackend, GpuManager},
            Bind, ExportMem,
        },
    },
    utils::{Point, Rectangle, Size},
};
use std::{collections::HashMap, fs, io, os::unix::io::OwnedFd};
use wayland_client::{
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
    ready: bool,
}

#[derive(Default)]
struct AppData {
    export_dmabuf_manager: Option<zcosmic_export_dmabuf_manager_v1::ZcosmicExportDmabufManagerV1>,
    outputs: Vec<(wl_output::WlOutput, String)>,
    frames: HashMap<String, DmaBufFrame>,
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
        let frame = app_data.frames.entry(output_name.clone()).or_default();

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
                frame.ready = true;
            }
            _ => {}
        }
    }
}

fn main() {
    let conn = Connection::connect_to_env().unwrap();
    let display = conn.display();
    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();
    let _registry = display.get_registry(&qh, ());

    let mut app_data = AppData::default();

    event_queue.roundtrip(&mut app_data).unwrap();
    event_queue.roundtrip(&mut app_data).unwrap();

    let manager = app_data.export_dmabuf_manager.as_ref().unwrap();
    for (output, name) in &app_data.outputs {
        manager.capture_output(0, output, &qh, name.clone());
    }

    while app_data.frames.values().filter(|x| x.ready).count() < app_data.outputs.len() {
        event_queue.blocking_dispatch(&mut app_data).unwrap();
    }

    let mut gpu_manager = GpuManager::new(EglGlesBackend::<GlesRenderer>::default()).unwrap();

    for (k, v) in app_data.frames {
        let mut builder = Dmabuf::builder(
            (v.width as i32, v.height as i32),
            v.format.unwrap(),
            v.flags.unwrap(),
        );
        for object in v.objects {
            builder.add_plane(
                object.fd,
                object.index,
                object.offset,
                object.stride,
                v.modifier.unwrap(),
            );
        }
        let dmabuf = builder.build().unwrap();

        let drm_node = v.node.as_ref().unwrap();
        let mut renderer = gpu_manager.single_renderer(drm_node).unwrap();
        renderer.bind(dmabuf).unwrap();
        let rectangle = Rectangle {
            loc: Point::default(),
            size: Size::from((v.width as i32, v.height as i32)),
        };
        let mapping = renderer
            .copy_framebuffer(rectangle, Fourcc::Argb8888)
            .unwrap();
        let data = renderer.map_texture(&mapping).unwrap();

        let path = format!("{}.png", k);
        let file = io::BufWriter::new(fs::File::create(&path).unwrap());
        let mut encoder = png::Encoder::new(file, v.width, v.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&data).unwrap();
        println!("Written image to '{}'.", path);
    }
}
