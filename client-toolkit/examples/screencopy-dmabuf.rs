use cosmic_client_toolkit::{
    screencopy::{
        BufferInfo, ScreencopyHandler, ScreencopySessionData, ScreencopySessionDataExt,
        ScreencopyState,
    },
    workspace::{WorkspaceGroup, WorkspaceHandler, WorkspaceState},
};
use cosmic_protocols::{
    screencopy::v1::client::{zcosmic_screencopy_manager_v1, zcosmic_screencopy_session_v1},
    workspace::v1::client::zcosmic_workspace_group_handle_v1,
};
use sctk::{
    dmabuf::{DmabufFeedback, DmabufHandler, DmabufState},
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    shm::{raw::RawPool, Shm, ShmHandler},
};
use std::{
    fs,
    os::{fd::AsFd, unix::{ffi::OsStrExt, fs::MetadataExt}},
    str,
    sync::{Arc, Mutex},
};
use wayland_client::{
    backend::ObjectId,
    globals::registry_queue_init,
    protocol::{wl_buffer, wl_output, wl_shm, wl_shm_pool},
    Connection, Dispatch, Proxy, QueueHandle, WEnum,
};
use wayland_protocols::wp::linux_dmabuf::zv1::client::{
    zwp_linux_buffer_params_v1::{self, ZwpLinuxBufferParamsV1},
    zwp_linux_dmabuf_feedback_v1::ZwpLinuxDmabufFeedbackV1,
};

struct AppData {
    dmabuf_state: DmabufState,
    output_state: OutputState,
    registry_state: RegistryState,
    screencopy_state: ScreencopyState,
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

impl DmabufHandler for AppData {
    fn dmabuf_state(&mut self) -> &mut DmabufState {
        &mut self.dmabuf_state
    }
    fn dmabuf_feedback(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        proxy: &ZwpLinuxDmabufFeedbackV1,
        feedback: DmabufFeedback,
    ) {
    }
    fn created(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        params: &ZwpLinuxBufferParamsV1,
        buffer: wl_buffer::WlBuffer,
    ) {
    }
    fn failed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        params: &ZwpLinuxBufferParamsV1,
    ) {
    }
    fn released(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        buffer: &wl_buffer::WlBuffer,
    ) {
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
        let buffer_info = buffer_infos
            .iter()
            .find(|x| {
                x.type_ == WEnum::Value(zcosmic_screencopy_session_v1::BufferType::Dmabuf)
                    && x.format == wl_shm::Format::Abgr8888.into()
            })
            .unwrap();
        let session_data = SessionData::for_session(session).unwrap();
        let gbm = session_data.device.lock().unwrap();
        let buffer = gbm
            .create_buffer_object::<()>(
                buffer_info.width,
                buffer_info.height,
                gbm::Format::Abgr8888,
                gbm::BufferObjectFlags::LINEAR,
            )
            .unwrap();
        let fd = buffer.fd().unwrap();
        let mut params = self.dmabuf_state.create_params(qh).unwrap();
        params.add(
            fd.as_fd(),
            0,
            buffer.offset(0).unwrap(),
            buffer.stride().unwrap(),
            buffer.modifier().unwrap().into(),
        );
        let (wl_buffer, _) = params.create_immed(
            buffer_info.width as i32,
            buffer_info.height as i32,
            gbm::Format::Abgr8888 as u32,
            zwp_linux_buffer_params_v1::Flags::empty(),
            qh,
        );
        session.attach_buffer(&wl_buffer, None, 0);
        session.commit(zcosmic_screencopy_session_v1::Options::empty());
        dbg!(buffer_info);
    }

    fn ready(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        session: &zcosmic_screencopy_session_v1::ZcosmicScreencopySessionV1,
    ) {
    }

    fn failed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        session: &zcosmic_screencopy_session_v1::ZcosmicScreencopySessionV1,
        reason: WEnum<zcosmic_screencopy_session_v1::FailureReason>,
    ) {
    }
}

struct SessionData {
    session_data: ScreencopySessionData,
    device: Arc<Mutex<gbm::Device<fs::File>>>,
}

impl SessionData {
    pub fn for_session(
        session: &zcosmic_screencopy_session_v1::ZcosmicScreencopySessionV1,
    ) -> Option<&Self> {
        Some(session.data::<SessionData>()?)
    }
}

impl ScreencopySessionDataExt for SessionData {
    fn screencopy_session_data(&self) -> &ScreencopySessionData {
        &self.session_data
    }
}

fn main() {
    let conn = Connection::connect_to_env().unwrap();
    let (globals, mut event_queue) = registry_queue_init(&conn).unwrap();
    let qh: QueueHandle<AppData> = event_queue.handle();
    let mut app_data = AppData {
        dmabuf_state: DmabufState::new(&globals, &qh),
        output_state: OutputState::new(&globals, &qh),
        registry_state: RegistryState::new(&globals),
        screencopy_state: ScreencopyState::new(&globals, &qh),
    };

    let gbm_devices = gbm_devices()
        .map(|x| Arc::new(Mutex::new(x)))
        .collect::<Vec<_>>();

    event_queue.roundtrip(&mut app_data).unwrap();

    for output in app_data.output_state.outputs() {
        for device in gbm_devices.iter().cloned() {
            app_data.screencopy_state.screencopy_manager.capture_output(
                &output,
                zcosmic_screencopy_manager_v1::CursorMode::Hidden,
                &qh,
                SessionData {
                    session_data: ScreencopySessionData::default(),
                    device,
                },
            );
        }
    }

    loop {
        event_queue.blocking_dispatch(&mut app_data).unwrap();
    }
}

fn gbm_devices() -> impl Iterator<Item = gbm::Device<fs::File>> {
    fs::read_dir("/dev/dri").unwrap().filter_map(|i| {
        let i = i.unwrap();
        if str::from_utf8(i.file_name().as_bytes())
            .unwrap()
            .starts_with("card")
        {
            let file = fs::File::options()
                .read(true)
                .write(true)
                .open(i.path())
                .unwrap();
            Some(gbm::Device::new(file).unwrap())
        } else {
            None
        }
    })
}

sctk::delegate_dmabuf!(AppData);
sctk::delegate_output!(AppData);
sctk::delegate_registry!(AppData);
cosmic_client_toolkit::delegate_screencopy!(AppData, session: [SessionData]);
