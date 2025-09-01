use cosmic_client_toolkit::screencopy::{
    CaptureFrame, CaptureOptions, CaptureSession, CaptureSource, FailureReason, Formats,
    ScreencopyFrameData, ScreencopyFrameDataExt, ScreencopyHandler, ScreencopySessionData,
    ScreencopySessionDataExt, ScreencopyState,
};
use sctk::{
    dmabuf::{DmabufFeedback, DmabufHandler, DmabufState},
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
};
use smithay::{
    backend::{
        allocator::dmabuf::Dmabuf,
        egl::{EGLContext, EGLDevice, EGLDisplay},
        renderer::{
            ExportMem, ImportDma,
            gles::{GlesRenderer, GlesTexture},
        },
    },
    utils::Rectangle,
};
use std::{
    fs, io,
    os::{fd::AsFd, unix::fs::MetadataExt},
    path::PathBuf,
    sync::Mutex,
};
use wayland_client::{
    Connection, QueueHandle, WEnum, delegate_noop,
    globals::registry_queue_init,
    protocol::{wl_buffer, wl_output},
};
use wayland_protocols::wp::linux_dmabuf::zv1::client::{
    zwp_linux_buffer_params_v1::{self, ZwpLinuxBufferParamsV1},
    zwp_linux_dmabuf_feedback_v1::ZwpLinuxDmabufFeedbackV1,
};

struct AppData {
    registry_state: RegistryState,
    output_state: OutputState,
    screencopy_state: ScreencopyState,
    dmabuf_state: DmabufState,
    outputs_done: u32,
    egl_devices: Vec<EGLDevice>,
}

impl ProvidesRegistryState for AppData {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    sctk::registry_handlers!();
}

impl DmabufHandler for AppData {
    fn dmabuf_state(&mut self) -> &mut DmabufState {
        &mut self.dmabuf_state
    }

    fn dmabuf_feedback(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _proxy: &ZwpLinuxDmabufFeedbackV1,
        _feedback: DmabufFeedback,
    ) {
    }

    fn created(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _params: &ZwpLinuxBufferParamsV1,
        _buffer: wl_buffer::WlBuffer,
    ) {
    }

    fn failed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _params: &ZwpLinuxBufferParamsV1,
    ) {
    }

    fn released(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _buffer: &wl_buffer::WlBuffer,
    ) {
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

impl ScreencopyHandler for AppData {
    fn screencopy_state(&mut self) -> &mut ScreencopyState {
        &mut self.screencopy_state
    }

    fn init_done(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        session: &CaptureSession,
        formats: &Formats,
    ) {
        let (width, height) = formats.buffer_size;

        let dev_id = formats.dmabuf_device.unwrap();
        let (_dev_path, gbm) = find_gbm_device(dev_id).unwrap().unwrap();
        // Abgr8888 not supported testing on sway
        //let format = gbm::Format::Abgr8888;
        let format = gbm::Format::Argb8888;
        let (_, modifiers) = &formats
            .dmabuf_formats
            .iter()
            .find(|(f, _)| *f == format as u32)
            .unwrap();
        let bo = gbm
            .create_buffer_object_with_modifiers2::<()>(
                width,
                height,
                format,
                modifiers.iter().map(|m| gbm::Modifier::from(*m)),
                gbm::BufferObjectFlags::empty(),
            )
            .unwrap();

        let params = self.dmabuf_state.create_params(qh).unwrap();
        for i in 0..bo.plane_count() as i32 {
            let fd = bo.fd_for_plane(i).unwrap();
            let offset = bo.offset(i);
            let stride = bo.stride_for_plane(i);
            params.add(fd.as_fd(), i as _, offset, stride, bo.modifier().into());
        }
        let (wl_buffer, _) = params.create_immed(
            width as _,
            height as _,
            format as u32,
            zwp_linux_buffer_params_v1::Flags::empty(),
            qh,
        );

        let egl_device = self
            .egl_devices
            .iter()
            .find(|d| {
                d.try_get_render_node()
                    .ok()
                    .flatten()
                    .is_some_and(|node| node.dev_id() == dev_id)
            })
            .unwrap();
        let egl_display = unsafe { EGLDisplay::new(egl_device.clone()).unwrap() };
        let egl_context = EGLContext::new(&egl_display).unwrap();
        let mut gles_renderer = unsafe { GlesRenderer::new(egl_context).unwrap() };

        let mut dmabuf_builder = Dmabuf::builder(
            (width as i32, height as i32),
            format,
            bo.modifier(),
            smithay::backend::allocator::dmabuf::DmabufFlags::empty(),
        );
        for i in 0..bo.plane_count() as i32 {
            let fd = bo.fd_for_plane(i).unwrap();
            let offset = bo.offset(i);
            let stride = bo.stride_for_plane(i);
            dmabuf_builder.add_plane(fd, i as _, offset, stride);
        }
        let dmabuf = dmabuf_builder.build().unwrap();
        let gles_texture = gles_renderer.import_dmabuf(&dmabuf, None).unwrap();

        session.capture(
            &wl_buffer,
            &[],
            qh,
            FrameData {
                frame_data: ScreencopyFrameData::default(),
                output_name: session.data::<SessionData>().unwrap().output_name.clone(),
                size: formats.buffer_size,
                gles_renderer: Mutex::new(gles_renderer),
                gles_texture,
                format,
            },
        );
    }

    fn stopped(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &CaptureSession) {}

    fn ready(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        capture_frame: &CaptureFrame,
        _: cosmic_client_toolkit::screencopy::Frame,
    ) {
        let data = capture_frame.data::<FrameData>().unwrap();

        let mut gles_renderer = data.gles_renderer.lock().unwrap();
        let mapping = gles_renderer
            .copy_texture(
                &data.gles_texture,
                Rectangle::from_size((data.size.0 as _, data.size.1 as _).into()),
                data.format,
            )
            .unwrap();
        let bytes = gles_renderer.map_texture(&mapping).unwrap();

        // Convert BGRA to RGBA
        let mut bytes = bytes.to_vec();
        for pixel in bytes.chunks_mut(4) {
            let b = pixel[0];
            let r = pixel[2];
            pixel[0] = r;
            pixel[2] = b;
        }

        let path = format!("{}.png", data.output_name);
        let file = io::BufWriter::new(fs::File::create(&path).unwrap());
        let mut encoder = png::Encoder::new(file, data.size.0, data.size.1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&bytes).unwrap();
        println!("Written image to '{}'.", path);

        self.outputs_done += 1;
    }

    fn failed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &CaptureFrame,
        reason: WEnum<FailureReason>,
    ) {
        println!("Failed to capture output: {:?}", reason);
        self.outputs_done += 1;
    }
}

struct SessionData {
    session_data: ScreencopySessionData,
    output_name: String,
}

impl ScreencopySessionDataExt for SessionData {
    fn screencopy_session_data(&self) -> &ScreencopySessionData {
        &self.session_data
    }
}

struct FrameData {
    frame_data: ScreencopyFrameData,
    output_name: String,
    size: (u32, u32),
    gles_renderer: Mutex<GlesRenderer>,
    gles_texture: GlesTexture,
    format: gbm::Format,
}
// SAFETY: Not actually using multiple threads in example
unsafe impl Send for FrameData {}
unsafe impl Sync for FrameData {}

impl ScreencopyFrameDataExt for FrameData {
    fn screencopy_frame_data(&self) -> &ScreencopyFrameData {
        &self.frame_data
    }
}

fn main() {
    let conn = Connection::connect_to_env().unwrap();
    let (globals, mut event_queue) = registry_queue_init(&conn).unwrap();
    let qh = event_queue.handle();

    let registry_state = RegistryState::new(&globals);
    let screencopy_state = ScreencopyState::new(&globals, &qh);
    let output_state = OutputState::new(&globals, &qh);
    let dmabuf_state = DmabufState::new(&globals, &qh);

    let mut data: AppData = AppData {
        output_state,
        registry_state,
        screencopy_state,
        dmabuf_state,
        outputs_done: 0,
        egl_devices: EGLDevice::enumerate().unwrap().collect(),
    };

    event_queue.roundtrip(&mut data).unwrap();

    let mut num_outputs = 0;
    let _sessions = data
        .output_state
        .outputs()
        .map(|output| {
            num_outputs += 1;
            let info = data.output_state.info(&output).unwrap();
            data.screencopy_state
                .capturer()
                .create_session(
                    &CaptureSource::Output(output),
                    CaptureOptions::empty(),
                    &qh,
                    SessionData {
                        output_name: info.name.clone().unwrap(),
                        session_data: ScreencopySessionData::default(),
                    },
                )
                .unwrap()
        })
        .collect::<Vec<_>>();

    while data.outputs_done < num_outputs {
        event_queue.blocking_dispatch(&mut data).unwrap();
    }
}

fn find_gbm_device(dev: u64) -> io::Result<Option<(PathBuf, gbm::Device<fs::File>)>> {
    for i in std::fs::read_dir("/dev/dri")? {
        let i = i?;
        if i.metadata()?.rdev() == dev {
            let file = fs::File::options().read(true).write(true).open(i.path())?;
            return Ok(Some((i.path(), gbm::Device::new(file)?)));
        }
    }
    Ok(None)
}

sctk::delegate_output!(AppData);
sctk::delegate_registry!(AppData);
sctk::delegate_dmabuf!(AppData);
cosmic_client_toolkit::delegate_screencopy!(AppData, session: [SessionData], frame: [FrameData]);
delegate_noop!(AppData: ignore wl_buffer::WlBuffer);
