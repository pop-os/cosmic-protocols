use cosmic_client_toolkit::{
    egl,
    export_dmabuf::{DmabufFrame, ExportDmabufHandler, ExportDmabufState},
    gl,
};
use cosmic_protocols::export_dmabuf::v1::client::{
    zcosmic_export_dmabuf_frame_v1, zcosmic_export_dmabuf_manager_v1,
};
use glow::HasContext;
use glutin::{
    api::egl::display::Display,
    config, context,
    display::{AsRawDisplay, RawDisplay},
    prelude::*,
    surface::{SurfaceAttributesBuilder, WindowSurface},
};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle};
use sctk::{
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
};
use std::{
    collections::HashMap,
    num::NonZeroU32,
    sync::{mpsc, Arc, Mutex},
};
use wayland_client::{
    backend::{Backend, ObjectId},
    protocol::wl_output,
    Connection, Proxy, QueueHandle,
};
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

static VS: &str = "
    #version 100
    precision mediump float;

    attribute vec2 position;
    uniform vec4 rect;
    varying vec2 tex_pos;

    void main() {
        vec2 rect_pos = rect.xy;
        vec2 rect_size = rect.zw;
        gl_Position = vec4(rect_pos + (position * rect_size), 0.0, 1.0);
        tex_pos = position * vec2(1.0, -1.0);
    }
";
static FS: &str = "
    #version 100
    precision mediump float;

    uniform sampler2D tex;
    varying vec2 tex_pos;

    void main() {
        gl_FragColor = texture2D(tex, tex_pos);
    }
";

unsafe fn compile_shader(
    gl: &glow::Context,
    shader_type: u32,
    source: &str,
) -> Result<glow::NativeShader, String> {
    let shader = gl.create_shader(shader_type)?;
    gl.shader_source(shader, source);
    gl.compile_shader(shader);
    if gl.get_shader_compile_status(shader) {
        Ok(shader)
    } else {
        let err = gl.get_shader_info_log(shader);
        gl.delete_shader(shader);
        Err(err)
    }
}

// XXX leaks on ? error handling
unsafe fn compile_program(
    gl: &glow::Context,
    vs_source: &str,
    fs_source: &str,
) -> Result<glow::NativeProgram, String> {
    let vs = compile_shader(&gl, glow::VERTEX_SHADER, vs_source)?;
    let fs = compile_shader(&gl, glow::FRAGMENT_SHADER, fs_source)?;
    let program = gl.create_program()?;

    gl.attach_shader(program, vs);
    gl.attach_shader(program, fs);

    gl.link_program(program);

    gl.delete_shader(vs);
    gl.delete_shader(fs);
    gl.detach_shader(program, vs);
    gl.detach_shader(program, fs);

    if gl.get_program_link_status(program) {
        Ok(program)
    } else {
        let err = gl.get_program_info_log(program);
        gl.delete_program(program);
        Err(err)
    }
}

struct AppData {
    frames: Arc<Mutex<HashMap<ObjectId, mpsc::Sender<DmabufFrame>>>>,
    registry_state: RegistryState,
    output_state: OutputState,
    export_dmabuf_state: ExportDmabufState,
}

impl ProvidesRegistryState for AppData {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    sctk::registry_handlers!(OutputState, ExportDmabufState,);
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
            .send(dmabuf)
            .unwrap();
    }

    fn frame_cancel(&mut self, frame: &zcosmic_export_dmabuf_frame_v1::ZcosmicExportDmabufFrameV1) {
        self.frames.lock().unwrap().remove(&frame.id());
    }
}

struct Exporter {
    connection: Connection,
    qh: QueueHandle<AppData>,
    export_dmabuf_manager: zcosmic_export_dmabuf_manager_v1::ZcosmicExportDmabufManagerV1,
    frames: Arc<Mutex<HashMap<ObjectId, mpsc::Sender<DmabufFrame>>>>,
    outputs: Vec<(wl_output::WlOutput, String)>,
}

fn start_sctk(display: RawDisplayHandle) -> Exporter {
    let wl_display = match display {
        RawDisplayHandle::Wayland(display) => display,
        _ => panic!("Non-wayland display"),
    }
    .display;

    let connection =
        Connection::from_backend(unsafe { Backend::from_foreign_display(wl_display as _) });
    let mut event_queue = connection.new_event_queue();
    let qh = event_queue.handle();

    let frames = Arc::new(Mutex::new(HashMap::new()));

    let mut app_data = AppData {
        frames: frames.clone(),
        registry_state: RegistryState::new(&connection, &qh),
        output_state: OutputState::new(),
        export_dmabuf_state: ExportDmabufState::new(),
    };
    while !app_data.registry_state.ready() {
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

    std::thread::spawn(move || loop {
        event_queue.blocking_dispatch(&mut app_data).unwrap();
    });

    Exporter {
        connection,
        qh,
        export_dmabuf_manager,
        frames,
        outputs,
    }
}

fn main() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let exporter = start_sctk(event_loop.raw_display_handle());

    let gl_display = unsafe { Display::from_raw(event_loop.raw_display_handle()).unwrap() };
    let config_template = config::ConfigTemplateBuilder::default()
        .compatible_with_native_window(window.raw_window_handle())
        .with_surface_type(config::ConfigSurfaceTypes::WINDOW)
        .with_api(config::Api::GLES2)
        .build();
    let config = unsafe { gl_display.find_configs(config_template) }
        .unwrap()
        .next()
        .unwrap();
    let attrs = context::ContextAttributesBuilder::default()
        .with_context_api(context::ContextApi::Gles(Some(context::Version {
            major: 2,
            minor: 0,
        })))
        .build(Some(window.raw_window_handle()));
    let not_current_context = unsafe { gl_display.create_context(&config, &attrs).unwrap() };

    let (width, height) = window.inner_size().into();
    let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
        window.raw_window_handle(),
        NonZeroU32::new(width).unwrap(),
        NonZeroU32::new(height).unwrap(),
    );
    let surface = unsafe { gl_display.create_window_surface(&config, &attrs).unwrap() };
    let context = not_current_context.make_current(&surface).unwrap();

    let gl = unsafe { glow::Context::from_loader_function(egl::get_proc_address) };
    let rect_uniform;
    unsafe {
        let program = compile_program(&gl, VS, FS).unwrap();
        rect_uniform = gl.get_uniform_location(program, "rect").unwrap();
        gl.use_program(Some(program));

        let vbo = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
        let vertices = &[0, 0, 1, 0, 0, 1, 1, 1];
        gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, vertices, glow::STATIC_DRAW);

        let vao = gl.create_vertex_array().unwrap();
        gl.bind_vertex_array(Some(vao));
        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_f32(0, 2, glow::UNSIGNED_BYTE, false, 2, 0);

        gl.enable(glow::TEXTURE_2D);
        gl.active_texture(glow::TEXTURE0);
    }

    // XXX don't rely on dmabuf continuing to work
    let egl_display = match gl_display.raw_display() {
        RawDisplay::Egl(display) => display,
        _ => unreachable!(),
    };
    let textures: Vec<_> = exporter
        .outputs
        .iter()
        .map(|(output, _name)| {
            let (sender, receiver) = mpsc::channel();
            let mut frames = exporter.frames.lock().unwrap();
            let frame = exporter
                .export_dmabuf_manager
                .capture_output(0, output, &exporter.qh, ());
            frames.insert(frame.id(), sender);
            let _ = exporter.connection.flush(); // XXX
            drop(frames);
            let dmabuf = receiver.recv().unwrap();
            let egl_image = unsafe { egl::EGLImage::import_dmabuf(egl_display, &dmabuf).unwrap() };
            let texture = unsafe { gl::bind_eglimage_to_texture(&egl_image).unwrap() };
            let glow_texture = unsafe { glow::Context::create_texture_from_gl_name(texture) };
            (glow_texture, dmabuf.width, dmabuf.height)
        })
        .collect();

    let mut window_width = 0.0;
    let mut window_height = 0.0;
    event_loop.run(move |event, _event_loop_window_target, control_flow| {
        window.request_redraw();

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::Resized(size) => {
                    if size.width != 0 && size.height != 0 {
                        surface.resize(
                            &context,
                            NonZeroU32::new(size.width).unwrap(),
                            NonZeroU32::new(size.height).unwrap(),
                        );
                        unsafe {
                            gl.viewport(0, 0, size.width as _, size.height as _);
                        }
                        window_width = size.width as f32;
                        window_height = size.height as f32;
                    }
                }
                WindowEvent::CloseRequested => {
                    control_flow.set_exit();
                }
                _ => {}
            },
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            Event::RedrawEventsCleared => {
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                unsafe {
                    gl.clear_color(1.0, 1.0, 1.0, 1.0);
                    gl.clear(glow::COLOR_BUFFER_BIT);
                    let spacing = 24.0 / window_width;
                    let window_ratio = window_width / window_height;
                    let width =
                        (2.0 - spacing * (textures.len() as f32 - 1.0)) / textures.len() as f32;
                    let mut x = -1.0;
                    for (texture, tex_width, tex_height) in &textures {
                        let tex_ratio = *tex_width as f32 / *tex_height as f32;
                        let height = width * window_ratio / tex_ratio;
                        let y = -height / 2.0;

                        gl.bind_texture(glow::TEXTURE_2D, Some(*texture));
                        gl.uniform_4_f32(Some(&rect_uniform), x, y, width, height);
                        gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
                        x += width + spacing;
                    }
                }
                surface.swap_buffers(&context).unwrap();
            }
            _ => {}
        }
    });
}

sctk::delegate_output!(AppData);
sctk::delegate_registry!(AppData);
cosmic_client_toolkit::delegate_export_dmabuf!(AppData);
