use cosmic_client_toolkit::screencopy::{
    CaptureFrame, CaptureOptions, CaptureSession, CaptureSource, FailureReason, Formats,
    ScreencopyFrameData, ScreencopyFrameDataExt, ScreencopyHandler, ScreencopySessionData,
    ScreencopySessionDataExt, ScreencopyState,
};
use sctk::{
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    shm::{Shm, ShmHandler, raw::RawPool},
};
use std::{fs, io, sync::Mutex};
use wayland_client::{
    Connection, QueueHandle, WEnum, delegate_noop,
    globals::registry_queue_init,
    protocol::{wl_buffer, wl_output, wl_shm},
};

struct AppData {
    shm_state: Shm,
    registry_state: RegistryState,
    output_state: OutputState,
    screencopy_state: ScreencopyState,
    outputs_done: u32,
}

impl ProvidesRegistryState for AppData {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    sctk::registry_handlers!();
}

impl ShmHandler for AppData {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm_state
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
        let mut pool = RawPool::new(width as usize * height as usize * 4, &self.shm_state).unwrap();
        // TODO test format in &formats.shm_formats;
        let wl_buffer = pool.create_buffer(
            0,
            width as i32,
            height as i32,
            width as i32 * 4,
            wl_shm::Format::Abgr8888,
            (),
            qh,
        );
        session.capture(
            &wl_buffer,
            &[],
            qh,
            FrameData {
                frame_data: ScreencopyFrameData::default(),
                output_name: session.data::<SessionData>().unwrap().output_name.clone(),
                pool: Mutex::new(pool),
                size: formats.buffer_size,
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
        let mut pool = data.pool.lock().unwrap();
        let bytes = pool.mmap();

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
    pool: Mutex<RawPool>,
    size: (u32, u32),
}

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
    let shm_state = Shm::bind(&globals, &qh).unwrap();
    let screencopy_state = ScreencopyState::new(&globals, &qh);
    let output_state = OutputState::new(&globals, &qh);

    let mut data: AppData = AppData {
        output_state,
        shm_state,
        registry_state,
        screencopy_state,
        outputs_done: 0,
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

sctk::delegate_output!(AppData);
sctk::delegate_registry!(AppData);
sctk::delegate_shm!(AppData);
cosmic_client_toolkit::delegate_screencopy!(AppData);
delegate_noop!(AppData: ignore wl_buffer::WlBuffer);
