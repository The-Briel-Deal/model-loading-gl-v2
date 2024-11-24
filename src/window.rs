use std::{
    ffi::{CStr, CString},
    ops::Deref,
};

use anyhow::Context;
use gl::types::GLfloat;
use glutin::{
    config::{Config, ConfigTemplateBuilder, GlConfig},
    context::{ContextAttributesBuilder, NotCurrentContext, PossiblyCurrentContext},
    display::GetGlDisplay,
    prelude::{GlDisplay, NotCurrentGlContext},
    surface::{GlSurface, Surface, SurfaceAttributesBuilder, WindowSurface},
};
use glutin_winit::{DisplayBuilder, GlWindow};
use winit::{
    application::ApplicationHandler,
    event_loop::EventLoop,
    raw_window_handle::HasWindowHandle,
    window::{Window, WindowAttributes},
};

pub mod gl {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/gl_bindings.rs"));
}

pub struct GfWindow {
    event_loop: Option<EventLoop<()>>,
    window: Window,
    config: Config,
    renderer: Option<Renderer>,
    surface: Option<Surface<WindowSurface>>,
    context: Option<PossiblyCurrentContext>,
    exit_state: anyhow::Result<()>,
}

impl GfWindow {
    pub fn new() -> anyhow::Result<Self> {
        let event_loop = EventLoop::builder().build().unwrap();
        let config_template_builder = ConfigTemplateBuilder::default();

        let config_picker = |configs: Box<dyn Iterator<Item = Config> + '_>| {
            configs
                .reduce(|acc, config| {
                    if config.num_samples() > acc.num_samples() {
                        config
                    } else {
                        acc
                    }
                })
                .unwrap()
        };
        let window_attributes = WindowAttributes::default().with_title("Model Testing Window");

        let (window, config) = DisplayBuilder::default()
            .with_window_attributes(Some(window_attributes))
            .build(&event_loop, config_template_builder, config_picker)
            .unwrap();
        let window = window.unwrap();

        Ok(GfWindow {
            event_loop: Some(event_loop),
            window,
            config,
            renderer: None,
            context: None,
            surface: None,
            exit_state: Ok(()),
        })
    }
    pub fn run(mut self) -> anyhow::Result<()> {
        self.event_loop
            .take()
            .context("No event loop.")?
            .run_app(&mut self)?;
        Ok(())
    }
    fn create_context(&self) -> anyhow::Result<NotCurrentContext> {
        let window_handle = self.window.window_handle()?.as_raw();
        let context_attributes = ContextAttributesBuilder::new().build(Some(window_handle));
        let gl_display = self.config.display();
        unsafe { Ok(gl_display.create_context(&self.config, &context_attributes)?) }
    }
}

impl ApplicationHandler for GfWindow {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let context = match self.create_context() {
            Ok(context) => context,
            Err(err) => {
                self.exit_state = Err(err);
                event_loop.exit();
                return;
            }
        };
        let surface_attributes_builder = SurfaceAttributesBuilder::new();
        let surface_attributes = self
            .window
            .build_surface_attributes(surface_attributes_builder)
            .unwrap();
        let surface = unsafe {
            self.config
                .display()
                .create_window_surface(&self.config, &surface_attributes)
                .unwrap()
        };
        let possibly_current_context = context.make_current(&surface).unwrap();

        // Renderer can't be instantiated until context is current
        let renderer = Renderer::new(&self.config.display());

        self.renderer = Some(renderer);
        self.context = Some(possibly_current_context);
        self.surface = Some(surface);
    }
    fn window_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if let winit::event::WindowEvent::RedrawRequested = event {
            self.renderer.as_ref().unwrap().draw();
            self.window.request_redraw();
            let _ = self
                .surface
                .as_ref()
                .unwrap()
                .swap_buffers(self.context.as_ref().unwrap());
        }
        dbg!("Window Event Called");
    }
}

fn get_gl_string(gl: &gl::Gl, variant: gl::types::GLenum) -> Option<&'static CStr> {
    unsafe {
        let s = gl.GetString(variant);
        (!s.is_null()).then(|| CStr::from_ptr(s.cast()))
    }
}

pub struct Renderer {
    program: gl::types::GLuint,
    vao: gl::types::GLuint,
    vbo: gl::types::GLuint,
    gl: gl::Gl,
}

impl Renderer {
    pub fn new<D: GlDisplay>(gl_display: &D) -> Self {
        unsafe {
            let gl = gl::Gl::load_with(|symbol| {
                let symbol = CString::new(symbol).unwrap();
                gl_display.get_proc_address(symbol.as_c_str()).cast()
            });

            if let Some(renderer) = get_gl_string(&gl, gl::RENDERER) {
                println!("Running on {}", renderer.to_string_lossy());
            }
            if let Some(version) = get_gl_string(&gl, gl::VERSION) {
                println!("OpenGL Version {}", version.to_string_lossy());
            }

            if let Some(shaders_version) = get_gl_string(&gl, gl::SHADING_LANGUAGE_VERSION) {
                println!("Shaders version on {}", shaders_version.to_string_lossy());
            }

            let vertex_shader = create_shader(&gl, gl::VERTEX_SHADER, VERTEX_SHADER_SOURCE);
            let fragment_shader = create_shader(&gl, gl::FRAGMENT_SHADER, FRAGMENT_SHADER_SOURCE);

            let program = gl.CreateProgram();

            gl.AttachShader(program, vertex_shader);
            gl.AttachShader(program, fragment_shader);

            gl.LinkProgram(program);

            gl.UseProgram(program);

            gl.DeleteShader(vertex_shader);
            gl.DeleteShader(fragment_shader);

            let mut vao = std::mem::zeroed();
            gl.GenVertexArrays(1, &mut vao);
            assert_ne!(vao, 0);
            gl.BindVertexArray(vao);

            let mut vbo = std::mem::zeroed();
            gl.GenBuffers(1, &mut vbo);
            assert_ne!(vbo, 0);
            gl.BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl.BufferData(
                gl::ARRAY_BUFFER,
                (VERTEX_DATA.len() * std::mem::size_of::<f32>()) as gl::types::GLsizeiptr,
                VERTEX_DATA.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );

            let pos_attrib = gl.GetAttribLocation(program, b"position\0".as_ptr() as *const _);
            let color_attrib = gl.GetAttribLocation(program, b"color\0".as_ptr() as *const _);
            gl.VertexAttribPointer(
                pos_attrib as gl::types::GLuint,
                2,
                gl::FLOAT,
                0,
                5 * std::mem::size_of::<f32>() as gl::types::GLsizei,
                std::ptr::null(),
            );
            gl.VertexAttribPointer(
                color_attrib as gl::types::GLuint,
                3,
                gl::FLOAT,
                0,
                5 * std::mem::size_of::<f32>() as gl::types::GLsizei,
                (2 * std::mem::size_of::<f32>()) as *const () as *const _,
            );
            gl.EnableVertexAttribArray(pos_attrib as gl::types::GLuint);
            gl.EnableVertexAttribArray(color_attrib as gl::types::GLuint);

            Self {
                program,
                vao,
                vbo,
                gl,
            }
        }
    }

    pub fn draw(&self) {
        self.draw_with_clear_color(0.1, 0.1, 0.1, 0.9)
    }

    pub fn draw_with_clear_color(
        &self,
        red: GLfloat,
        green: GLfloat,
        blue: GLfloat,
        alpha: GLfloat,
    ) {
        unsafe {
            self.gl.UseProgram(self.program);

            self.gl.BindVertexArray(self.vao);
            self.gl.BindBuffer(gl::ARRAY_BUFFER, self.vbo);

            self.gl.ClearColor(red, green, blue, alpha);
            self.gl.Clear(gl::COLOR_BUFFER_BIT);
            self.gl.DrawArrays(gl::TRIANGLES, 0, 3);
        }
    }

    pub fn resize(&self, width: i32, height: i32) {
        unsafe {
            self.gl.Viewport(0, 0, width, height);
        }
    }
}

impl Deref for Renderer {
    type Target = gl::Gl;

    fn deref(&self) -> &Self::Target {
        &self.gl
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            self.gl.DeleteProgram(self.program);
            self.gl.DeleteBuffers(1, &self.vbo);
            self.gl.DeleteVertexArrays(1, &self.vao);
        }
    }
}

unsafe fn create_shader(
    gl: &gl::Gl,
    shader: gl::types::GLenum,
    source: &[u8],
) -> gl::types::GLuint {
    let shader = gl.CreateShader(shader);
    gl.ShaderSource(
        shader,
        1,
        [source.as_ptr().cast()].as_ptr(),
        std::ptr::null(),
    );
    gl.CompileShader(shader);
    shader
}

#[rustfmt::skip]
static VERTEX_DATA: [f32; 15] = [
    -0.5, -0.5,  1.0,  0.0,  0.0,
     0.0,  0.5,  0.0,  1.0,  0.0,
     0.5, -0.5,  0.0,  0.0,  1.0,
];

const VERTEX_SHADER_SOURCE: &[u8] = b"
#version 100
precision mediump float;

attribute vec2 position;
attribute vec3 color;

varying vec3 v_color;

void main() {
    gl_Position = vec4(position, 0.0, 1.0);
    v_color = color;
}
\0";

const FRAGMENT_SHADER_SOURCE: &[u8] = b"
#version 100
precision mediump float;

varying vec3 v_color;

void main() {
    gl_FragColor = vec4(v_color, 1.0);
}
\0";
