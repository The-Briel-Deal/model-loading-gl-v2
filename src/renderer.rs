use std::{ffi::CString, ops::Deref, ptr::null};

use bytemuck::{cast, cast_slice, offset_of, Pod, Zeroable};
use glam::{vec3, Mat4, Vec3};
use glutin::prelude::GlDisplay;

use crate::{
    gl::get_gl_string,
    window::gl::{self, types::GLfloat},
};

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
    pub model_matrix: Mat4,
    view_matrix: Mat4,
    viewport_size: (i32, i32),
    gl: gl::Gl,
}

impl Renderer {
    pub fn new<D: GlDisplay>(gl_display: &D) -> Self {
        let gl = load_gl_fn_ptrs(gl_display);
        unsafe {
            gl.Enable(gl::DEPTH_TEST);

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

            let mut ibo = u32::zeroed();
            gl.CreateBuffers(1, &mut ibo);
            assert_ne!(ibo, 1);

            let index_data_as_bytes = cast_slice::<u32, u8>(&INDEX_DATA);
            gl.NamedBufferStorage(
                ibo,
                cast(index_data_as_bytes.len()),
                index_data_as_bytes.as_ptr() as *const _,
                gl::DYNAMIC_STORAGE_BIT,
            );

            gl.VertexArrayVertexBuffer(
                vao,
                0,
                vbo,
                0,
                std::mem::size_of::<Vertex>() as gl::types::GLsizei,
            );
            gl.VertexArrayElementBuffer(vao, ibo);

            let pos_attrib = gl.GetAttribLocation(program, b"aPosition\0".as_ptr() as *const _);
            gl.EnableVertexArrayAttrib(vao, pos_attrib as u32);
            gl.VertexArrayAttribFormat(vao, pos_attrib as u32, 3, gl::FLOAT, false as u8, 0);
            gl.VertexArrayAttribBinding(vao, pos_attrib as u32, 0);

            let color_attrib = gl.GetAttribLocation(program, b"aColor\0".as_ptr() as *const _);
            gl.EnableVertexArrayAttrib(vao, color_attrib as u32);
            gl.VertexArrayAttribFormat(
                vao,
                color_attrib as u32,
                (size_of::<Vec3>() / size_of::<f32>()) as i32,
                gl::UNSIGNED_INT,
                false as u8,
                offset_of!(Vertex, color) as u32,
            );
            gl.VertexArrayAttribBinding(vao, color_attrib as u32, 0);

            Self {
                program,
                vao,
                vbo,
                model_matrix: Mat4::from_rotation_x(-95.0_f32.to_radians()),
                view_matrix: Mat4::from_translation(vec3(0.0, 0.0, -3.0)),
                viewport_size: (800, 600),
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
            let projection_matrix = Mat4::perspective_rh_gl(
                45.0_f32.to_radians(),
                self.viewport_size.0 as f32 / self.viewport_size.1 as f32,
                0.1_f32,
                100.0_f32,
            );

            let combined_matrix = projection_matrix * self.view_matrix * self.model_matrix;
            // Set rotation Matrix
            let matrix_location = self
                .gl
                .GetUniformLocation(self.program, b"uMatrix\0".as_ptr().cast());
            self.gl.UniformMatrix4fv(
                matrix_location,
                1,
                cast(false),
                combined_matrix.to_cols_array().as_ptr(),
            );

            self.gl.UseProgram(self.program);

            self.gl.BindVertexArray(self.vao);
            self.gl.BindBuffer(gl::ARRAY_BUFFER, self.vbo);

            self.gl.ClearColor(red, green, blue, alpha);
            self.gl.Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
            self.gl
                .DrawElements(gl::TRIANGLES, 12, gl::UNSIGNED_INT, null());
        }
    }

    pub fn resize(&mut self, width: i32, height: i32) {
        unsafe {
            self.viewport_size = (width, height);
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
    pub position: Vec3,
    pub color: Vec3,
}
impl Default for Vertex {
    fn default() -> Self {
        Self::zeroed()
    }
}

static VERTEX_DATA: [Vertex; 4] = [
    Vertex {
        position: vec3(-0.5, -0.5, 0.0),
        color: vec3(1.0, 0.0, 0.0),
    },
    Vertex {
        position: vec3(0.0, 0.5, 0.0),
        color: vec3(0.0, 1.0, 0.0),
    },
    Vertex {
        position: vec3(0.5, -0.5, 0.0),
        color: vec3(0.0, 0.0, 1.0),
    },
    Vertex {
        position: vec3(0.0, 0.0, 0.5),
        color: vec3(0.0, 0.0, 0.0),
    },
];

#[rustfmt::skip]
static INDEX_DATA: [u32; 12] = [
    0, 1, 2,
    0, 1, 3,
    0, 2, 3,
    1, 2, 3,
];

const VERTEX_SHADER_SOURCE: &[u8] = b"
#version 460 core

in vec3 aPosition;
in vec3 aColor;

uniform mat4 uMatrix;

out vec3 vColor;

void main() {
    gl_Position = uMatrix * vec4(aPosition, 1.0);
    vColor = aColor;
}
\0";

const FRAGMENT_SHADER_SOURCE: &[u8] = b"
#version 460 core

in vec3 vColor;
out vec4 FragColor;

void main() {
    FragColor = vec4(vColor, 1.0);
}
\0";
