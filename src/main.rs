#![feature(globs)] // Allow global imports

extern crate gl;
extern crate graphics;
extern crate native;
extern crate piston;
extern crate sdl2_game_window;

use sdl2_game_window::GameWindowSDL2;

use piston::*;
use gl::types::*;
use std::mem;
use std::ptr;
use std::str;

pub struct App {
  vao: u32,
  vbo: u32,
}

impl Game for App {
  fn load(&mut self) {
    let vs = compile_shader(VS_SRC, gl::VERTEX_SHADER);
    let fs = compile_shader(FS_SRC, gl::FRAGMENT_SHADER);
    let program = link_program(vs, fs);

    unsafe {
        // Create Vertex Array Object
        gl::GenVertexArrays(1, &mut self.vao);
        gl::BindVertexArray(self.vao);

        // Create a Vertex Buffer Object and copy the vertex data to it
        gl::GenBuffers(1, &mut self.vbo);
        gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);
        gl::BufferData(gl::ARRAY_BUFFER,
                        (VERTEX_DATA.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
                        mem::transmute(&VERTEX_DATA[0]),
                        gl::STATIC_DRAW);

        gl::UseProgram(program);
        "out_color".with_c_str(|ptr| gl::BindFragDataLocation(program, 0, ptr));

        // Specify the layout of the vertex data
        let pos_attr = "position".with_c_str(|ptr| gl::GetAttribLocation(program, ptr) as u32);
        gl::EnableVertexAttribArray(pos_attr);
        gl::VertexAttribPointer(pos_attr, 3, gl::FLOAT,
                                gl::FALSE as GLboolean, 0, ptr::null());

        gl::ClearColor(0.0, 0.0, 0.0, 1.0);
    }
  }

  fn render(&mut self, _:&RenderArgs) {
    gl::Clear(gl::COLOR_BUFFER_BIT);

    // Draw a triangle from the 3 vertices
    gl::DrawArrays(gl::TRIANGLES, 0, 3);
  }
}

impl App {
  pub fn new() -> App {
    App {
      vao: 0,
      vbo: 0,
    }
  }

  pub fn destroy(&mut self) {
    unsafe {
      gl::DeleteBuffers(1, &self.vbo);
      gl::DeleteVertexArrays(1, &self.vao);
    }
  }
}

// Vertex data
static VERTEX_DATA: [GLfloat, ..9] = [
     0.0,  0.5, 0.0,
     0.5, -0.5, 0.0,
    -0.5, -0.5, 0.0,
];

// Shader sources
static VS_SRC: &'static str =
   "#version 150\n\
in vec3 position;\n\
void main() {\n\
gl_Position = vec4(position, 1.0);\n\
}";

static FS_SRC: &'static str =
   "#version 150\n\
out vec4 out_color;\n\
void main() {\n\
out_color = vec4(1.0, 1.0, 1.0, 1.0);\n\
}";

fn compile_shader(src: &str, ty: GLenum) -> GLuint {
    let shader = gl::CreateShader(ty);
    unsafe {
        // Attempt to compile the shader
        src.with_c_str(|ptr| gl::ShaderSource(shader, 1, &ptr, ptr::null()));
        gl::CompileShader(shader);

        // Get the compile status
        let mut status = gl::FALSE as GLint;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);

        // Fail on error
        if status != (gl::TRUE as GLint) {
            let mut len = 0;
            gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
            let mut buf = Vec::from_elem(len as uint - 1, 0u8); // subtract 1 to skip the trailing null character
            gl::GetShaderInfoLog(shader, len, ptr::mut_null(), buf.as_mut_ptr() as *mut GLchar);
            fail!("{}", str::from_utf8(buf.as_slice()).expect("ShaderInfoLog not valid utf8"));
        }
    }
    shader
}

fn link_program(vs: GLuint, fs: GLuint) -> GLuint {
    let program = gl::CreateProgram();

    gl::AttachShader(program, vs);
    gl::AttachShader(program, fs);
    gl::LinkProgram(program);

    unsafe {
        // Get the link status
        let mut status = gl::FALSE as GLint;
        gl::GetProgramiv(program, gl::LINK_STATUS, &mut status);

        // Fail on error
        if status != (gl::TRUE as GLint) {
            let mut len: GLint = 0;
            gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
            let mut buf = Vec::from_elem(len as uint - 1, 0u8); // subtract 1 to skip the trailing null character
            gl::GetProgramInfoLog(program, len, ptr::mut_null(), buf.as_mut_ptr() as *mut GLchar);
            fail!("{}", str::from_utf8(buf.as_slice()).expect("ProgramInfoLog not valid utf8"));
        }
    }

    program
}

fn main() {
  let mut window = GameWindowSDL2::new(
    GameWindowSettings {
      title: "playform".to_string(),
      size: [800, 600],
      fullscreen: false,
      exit_on_esc: false,
    }
  );

  App::new().run(&mut window, &GameIteratorSettings {
    updates_per_second: 30,
    max_frames_per_second: 30,
  });
}
