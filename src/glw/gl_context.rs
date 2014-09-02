use color::Color4;
use cstr_cache;
use gl;
use gl::types::*;
use shader::Shader;
use std::mem;
use std::ptr;
use std::str;

#[allow(dead_code)]
unsafe fn println_c_str(str: *const u8) {
  let mut str = str;
  loop {
    let c = *str as char;
    if c == '\0' {
      println!("");
      return;
    }
    print!("{:c}", c);
    str = str.offset(1);
  }
}

/// A handle to an OpenGL context. Only create one of these per thread.
#[deriving(Send)]
pub struct GLContext {
  pub scache: cstr_cache::CStringCache,
}

impl GLContext {
  /// Create a new OpenGL context.
  pub fn new() -> GLContext {
    // TODO(cgaebel): Have a thread-local variable checking whether or not
    // there is only one GLContext, and fail if there's more than one.
    GLContext {
      scache: cstr_cache::CStringCache::new(),
    }
  }

  /// Stops the processing of any triangles hidden from view when rendering.
  pub fn enable_culling(&self) {
    gl::FrontFace(gl::CCW);
    gl::CullFace(gl::BACK);
    gl::Enable(gl::CULL_FACE);
  }

  #[allow(missing_doc)]
  pub fn enable_alpha_blending(&self) {
    gl::Enable(gl::BLEND);
    gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
  }

  #[allow(missing_doc)]
  pub fn enable_smooth_lines(&self) {
    gl::Enable(gl::LINE_SMOOTH);
    gl::LineWidth(2.5);
  }

  /// Allows us to use the OpenGL depth buffer, which makes OpenGL do logical
  /// things when two things are rendered at the same x and y coordinates, but
  /// different z coordinates.
  pub fn enable_depth_buffer(&self, depth: GLclampd) {
    gl::Enable(gl::DEPTH_TEST);
    gl::DepthFunc(gl::LESS);
    gl::ClearDepth(depth);
  }

  /// At the beginning of each frame, OpenGL clears the buffer. This sets the
  /// color the buffer is cleared to.
  pub fn set_background_color(&self, background_color: Color4<GLfloat>) {
    gl::ClearColor(
      background_color.r,
      background_color.g,
      background_color.b,
      background_color.a
    );
  }

  /// Replace the current OpenGL buffer with all pixels of the
  /// "background color", as set with `set_background_color`.
  pub fn clear_buffer(&self) {
    gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
  }

  /// Compiles a shader for the current graphics card.
  pub fn compile_shader(&self, src: String, ty: GLenum) -> GLuint {
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

  /// Links a vertex and fragment shader, returning the id of the
  /// resulting program.
  pub fn link_shader(&self, vertex_shader: GLuint, fragment_shader: GLuint) -> GLuint {
    let program = gl::CreateProgram();

    gl::AttachShader(program, vertex_shader);
    gl::AttachShader(program, fragment_shader);
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

  fn get_current_shader(&self) -> GLuint {
    unsafe {
      let mut ret: GLint = -1;
      gl::GetIntegerv(gl::CURRENT_PROGRAM, &mut ret);
      assert!(ret >= 0, "Need positive shader. Got {}.", ret);
      ret as GLuint
    }
  }

  /// Apply a given shader while rendering the body of the closure.
  pub fn use_shader<T>(&self, shader: &Shader, f: |&GLContext| -> T) -> T {
    // TODO(cgaebel): I heard that OpenGL MIGHT be synchronized on any of the
    // `Get` functions, which means this will be unnecessarily slow. One day
    // we should think about maintaining the shader stack ourselves.
    let old_shader = self.get_current_shader();
    gl::UseProgram(shader.id);
    let r = f(self);
    if old_shader != 0 { gl::UseProgram(old_shader); }
    r
  }

  #[allow(dead_code)]
  /// Returns the color of a pixel at (x, y). x and y must be the coordinates
  /// of a pixel in the window. This function will fail if they aren't.
  pub fn read_pixels(&self, x: uint, y: uint, window_height: uint, window_width: uint) -> Color4<u8> {
    assert!(x < window_width, "Expected pixel in range [0, {}), got {}.", window_width, x);
    assert!(y < window_width, "Expected pixel in range [0, {}), got {}.", window_height, y);

    unsafe {
      let pixels: Color4<u8> = Color4::of_rgba(0, 0, 0, 0);
      gl::ReadPixels(x as i32, y as i32, 1, 1, gl::RGB, gl::UNSIGNED_BYTE, mem::transmute(&pixels));
      pixels
    }
  }

  /// Prints opengl version information.
  pub fn print_stats(&self) {
    let opengl_version = gl::GetString(gl::VERSION);
    let glsl_version = gl::GetString(gl::SHADING_LANGUAGE_VERSION);
    print!("OpenGL version: ");
    unsafe { println_c_str(opengl_version); }
    print!("GLSL version: ");
    unsafe { println_c_str(glsl_version); }
    println!("");
  }
}
