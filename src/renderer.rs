use std::{ffi::CString, ops::Deref};

use bytemuck::{cast_slice, offset_of, Pod, Zeroable};
use glam::{vec2, vec3, Vec2, Vec3};
use glutin::prelude::GlDisplay;

use crate::{gl::get_gl_string, window::gl::{self, types::GLfloat}};



fn load_gl_fn_ptrs<D: GlDisplay>(gl_display: &D) -> gl::Gl {
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

    gl
}

pub struct Renderer {
    program: gl::types::GLuint,
    vao: gl::types::GLuint,
    vbo: gl::types::GLuint,
    gl: gl::Gl,
}

impl Renderer {
    pub fn new<D: GlDisplay>(gl_display: &D) -> Self {
        let gl = load_gl_fn_ptrs(gl_display);
        unsafe {
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
            gl.CreateVertexArrays(1, &mut vao);
            assert_ne!(vao, 0);

            let mut vbo = std::mem::zeroed();
            gl.CreateBuffers(1, &mut vbo);
            assert_ne!(vbo, 0);
            let vertex_data_as_bytes = cast_slice::<Vertex, u8>(&VERTEX_DATA);
            gl.NamedBufferStorage(
                vbo,
                vertex_data_as_bytes.len() as isize,
                vertex_data_as_bytes.as_ptr() as *const _,
                gl::DYNAMIC_STORAGE_BIT,
            );

            gl.VertexArrayVertexBuffer(
                vao,
                0,
                vbo,
                0,
                std::mem::size_of::<Vertex>() as gl::types::GLsizei,
            );

            let pos_attrib = gl.GetAttribLocation(program, b"position\0".as_ptr() as *const _);
            gl.EnableVertexArrayAttrib(vao, pos_attrib as u32);
            gl.VertexArrayAttribFormat(vao, pos_attrib as u32, 2, gl::FLOAT, false as u8, 0);
            gl.VertexArrayAttribBinding(vao, pos_attrib as u32, 0);

            let color_attrib = gl.GetAttribLocation(program, b"color\0".as_ptr() as *const _);
            gl.EnableVertexArrayAttrib(vao, color_attrib as u32);
            gl.VertexArrayAttribFormat(
                vao,
                color_attrib as u32,
                (size_of::<Vec3>() / size_of::<f32>()) as i32,
                gl::FLOAT,
                false as u8,
                offset_of!(Vertex, color) as u32,
            );
            gl.VertexArrayAttribBinding(vao, color_attrib as u32, 0);

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

#[repr(C)]
#[derive(Pod, Clone, Copy, Zeroable)]
pub struct Vertex {
    pub position: Vec2,
    pub color: Vec3,
}
impl Default for Vertex {
    fn default() -> Self {
        Self::zeroed()
    }
}

static VERTEX_DATA: [Vertex; 3] = [
    Vertex {
        position: vec2(-0.5, -0.5),
        color: vec3(1.0, 0.0, 0.0),
    },
    Vertex {
        position: vec2(0.0, 0.5),
        color: vec3(0.0, 1.0, 0.0),
    },
    Vertex {
        position: vec2(0.5, -0.5),
        color: vec3(0.0, 0.0, 1.0),
    },
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
