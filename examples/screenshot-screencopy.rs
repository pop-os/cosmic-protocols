use cosmic_protocols::screencopy::v1::client::{
    zcosmic_screencopy_manager_v1, zcosmic_screencopy_session_v1,
};
use memfd::MemfdOptions;
use std::{
    collections::HashMap,
    fs,
    io::{self, Read, Seek, SeekFrom},
    os::unix::io::AsFd,
};
use wayland_client::{
    protocol::{
        wl_buffer,
        wl_output::{self, Transform},
        wl_registry, wl_shm, wl_shm_pool,
    },
    Connection, Dispatch, QueueHandle,
};
use wayland_server::WEnum;

struct AppData {
    screencopy_manager: Option<zcosmic_screencopy_manager_v1::ZcosmicScreencopyManagerV1>,
    outputs: Vec<(wl_output::WlOutput, String)>,
    wl_shm: Option<wl_shm::WlShm>,
    formats: HashMap<String, (wl_shm::Format, (u32, u32))>,
    files: Vec<(
        zcosmic_screencopy_session_v1::ZcosmicScreencopySessionV1,
        Option<fs::File>,
    )>,
}

impl Default for AppData {
    fn default() -> Self {
        AppData {
            screencopy_manager: None,
            outputs: Vec::new(),
            wl_shm: None,
            formats: HashMap::new(),
            files: Vec::new(),
        }
    }
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
                "zcosmic_screencopy_manager_v1" => {
                    app_data.screencopy_manager = Some(registry
                        .bind::<zcosmic_screencopy_manager_v1::ZcosmicScreencopyManagerV1, _, _>(
                            name,
                            1,
                            qh,
                            (),
                        ));
                }
                "wl_shm" => {
                    app_data.wl_shm = Some(registry.bind::<wl_shm::WlShm, _, _>(name, 1, qh, ()));
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

impl Dispatch<wl_shm::WlShm, ()> for AppData {
    fn event(
        _: &mut Self,
        _: &wl_shm::WlShm,
        _: <wl_shm::WlShm as wayland_client::Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_shm_pool::WlShmPool, ()> for AppData {
    fn event(
        _: &mut Self,
        _: &wl_shm_pool::WlShmPool,
        _: <wl_shm_pool::WlShmPool as wayland_client::Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_buffer::WlBuffer, ()> for AppData {
    fn event(
        _: &mut Self,
        _: &wl_buffer::WlBuffer,
        _: <wl_buffer::WlBuffer as wayland_client::Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<zcosmic_screencopy_manager_v1::ZcosmicScreencopyManagerV1, ()> for AppData {
    fn event(
        _: &mut Self,
        _: &zcosmic_screencopy_manager_v1::ZcosmicScreencopyManagerV1,
        _: zcosmic_screencopy_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<AppData>,
    ) {
    }
}

impl Dispatch<zcosmic_screencopy_session_v1::ZcosmicScreencopySessionV1, String> for AppData {
    fn event(
        app_data: &mut Self,
        session: &zcosmic_screencopy_session_v1::ZcosmicScreencopySessionV1,
        event: zcosmic_screencopy_session_v1::Event,
        output_name: &String,
        _: &Connection,
        qh: &QueueHandle<AppData>,
    ) {
        match event {
            zcosmic_screencopy_session_v1::Event::BufferInfo {
                _type,
                node,
                format,
                width,
                height,
                stride,
            } => {
                let _ = node;
                if _type == WEnum::Value(zcosmic_screencopy_session_v1::BufferType::WlShm)
                    && (format == wl_shm::Format::Abgr8888 as u32
                        || format == wl_shm::Format::Xbgr8888 as u32)
                    && stride == width * 4
                {
                    app_data.formats.insert(
                        output_name.clone(),
                        (wl_shm::Format::try_from(format).unwrap(), (width, height)),
                    );
                }
            }

            zcosmic_screencopy_session_v1::Event::InitDone => {
                let (ref format, ref size) = app_data
                    .formats
                    .get(output_name)
                    .expect("No supported buffer format found");

                let fd = MemfdOptions::new()
                    .create("shm-pool")
                    .expect("Failed to create memfd");
                fd.as_file()
                    .set_len(size.0 as u64 * size.1 as u64 * 4)
                    .expect("Failed to resize memfd");
                let pool = app_data.wl_shm.as_ref().unwrap().create_pool(
                    fd.as_file().as_fd(),
                    (size.0 * size.1 * 4) as i32,
                    qh,
                    (),
                );

                let buffer = pool.create_buffer(
                    0,
                    size.0 as i32,
                    size.1 as i32,
                    size.0 as i32 * 4,
                    *format,
                    qh,
                    (),
                );
                session.attach_buffer(&buffer, None, 0);
                session.commit(zcosmic_screencopy_session_v1::Options::empty());

                app_data.files.push((session.clone(), Some(fd.into_file())));
            }

            zcosmic_screencopy_session_v1::Event::Transform { transform } => {
                if !matches!(transform.into_result(), Ok(Transform::Normal)) {
                    println!("Output {output_name} is transformed `{transform:?}`, which is not supported by this client. Screenshot will be rotated or flipped.");
                }
            }

            zcosmic_screencopy_session_v1::Event::Ready => {
                let (_, ref size) = app_data.formats.get(output_name).unwrap();
                let mut file = app_data
                    .files
                    .iter_mut()
                    .find(|(s, _)| s == session)
                    .map(|(_, file)| file.take())
                    .expect("Unknown session")
                    .expect("Ready twice");
                file.seek(SeekFrom::Start(0)).expect("Failed to seek memfd");
                let mut data = Vec::new();
                file.read_to_end(&mut data)
                    .expect("Failed to read screenshot from memory");

                let path = format!("{}.png", output_name);
                let file = io::BufWriter::new(fs::File::create(&path).unwrap());
                let mut encoder = png::Encoder::new(file, size.0 as u32, size.1 as u32);
                encoder.set_color(png::ColorType::Rgba);
                encoder.set_depth(png::BitDepth::Eight);
                let mut writer = encoder.write_header().unwrap();
                writer.write_image_data(&data).unwrap();
                println!("Written image to '{}'.", path);
            }

            zcosmic_screencopy_session_v1::Event::Failed { reason } => {
                let _ = app_data
                    .files
                    .iter_mut()
                    .find(|(s, _)| s == session)
                    .map(|(_, file)| file.take())
                    .expect("Unknown session")
                    .expect("Ready twice");
                println!("Screenshot failed to {output_name} with {reason:?}");
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

    let manager = app_data
        .screencopy_manager
        .as_ref()
        .expect("Screencopy protocol not supported");
    for (output, name) in &app_data.outputs {
        manager.capture_output(
            output,
            zcosmic_screencopy_manager_v1::CursorMode::Hidden,
            &qh,
            name.clone(),
        );
    }

    while app_data.files.len() < app_data.outputs.len()
        || app_data.files.iter().any(|(_, file)| file.is_some())
    {
        event_queue.blocking_dispatch(&mut app_data).unwrap();
    }
}
