//! An ownership-semantics based handle to OpenGL. This prevents us from
//! accidentally modifying OpenGL state from multiple threads.
//!
//! GLW stands for "OpenGL wrapper".
pub use color::Color4;
use cstr_cache;
use libc::types::common::c95;
use nalgebra::na::{Mat3, Mat4, Vec3, Eye, Outer};
use gl;
use gl::types::*;
pub use gl::types::GLfloat;
use std::mem;
use std::ptr;
use std::raw;
use std::rc::Rc;
use std::str;
use vertex;

pub struct Shader {
  id: GLuint,
  vs: GLuint,
  fs: GLuint,
}

impl Shader {
  pub fn new(gl: &mut GLContext, vertex_shader: &str, fragment_shader: &str) -> Shader {
    let vs = gl.compile_shader(vertex_shader, gl::VERTEX_SHADER);
    let fs = gl.compile_shader(fragment_shader, gl::FRAGMENT_SHADER);
    let id = gl.link_shader(vs, fs);
    Shader { id: id, vs: vs, fs: fs }
  }

  /// Sets the variable `projection_matrix` in some shader.
  pub fn set_projection_matrix(&self, gl: &mut GLContext, m: &Mat4<GLfloat>) {
    let var_name = gl.scache.convert("projection_matrix").as_ptr();
    gl.use_shader(self, |_gl| {
      unsafe {
        let loc = gl::GetUniformLocation(self.id, var_name);
        assert!(loc != -1, "couldn't read projection_matrix");

        match gl::GetError() {
          gl::NO_ERROR => {},
          err => fail!("OpenGL error 0x{:x} in GetUniformLocation", err),
        }

        let p = mem::transmute(m);
        gl::UniformMatrix4fv(loc, 1, 0, p);

        match gl::GetError() {
          gl::NO_ERROR => {},
          err => fail!("OpenGL error 0x{:x} in UniformMat4fv", err),
        }
      }
    })
  }

  pub fn set_camera(&self, gl: &mut GLContext, c: &Camera) {
    self.set_projection_matrix(gl, &c.projection_matrix());
  }
}

impl Drop for Shader {
  fn drop(&mut self) {
    gl::DeleteProgram(self.id);
    gl::DeleteShader(self.vs);
    gl::DeleteShader(self.fs);
  }
}

/// Gets the id number for a given input of the shader program.
#[allow(non_snake_case_functions)]
fn glGetAttribLocation(shader_program: GLuint, name: &str) -> GLint {
  name.with_c_str(|ptr| unsafe { gl::GetAttribLocation(shader_program, ptr) })
}

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

/// Ensures a slice has a given alignment, and converts it to a raw pointer.
unsafe fn aligned_slice_to_ptr<T>(vs: &[T], alignment: uint) -> *const c95::c_void {
  let vs_as_slice : raw::Slice<T> = mem::transmute(vs);
  assert!(
    (vs_as_slice.data as uint & (alignment - 1) == 0),
    "0x{:x} not {}-aligned",
    vs_as_slice.data as uint,
    alignment
  );
  assert!(vs_as_slice.data != ptr::null());
  vs_as_slice.data as *const c95::c_void
}

/// A fixed-capacity array of GLfloat-based structures passed to OpenGL.
pub struct GLBuffer<T> {
  vertex_array: u32,
  vertex_buffer: u32,
  /// Each index in the GLBuffer is the index of a contiguous block of
  /// t_span elements.
  t_span: uint,
  /// in units of single Ts.
  length:   uint,
  capacity: uint,
  shader: Rc<Shader>,
  /// How to draw this buffer. Ex: gl::LINES, gl::TRIANGLES, etc.
  mode: GLenum,

  /// in-memory buffer before sending to OpenGL.
  buffer: Vec<T>,
}

pub enum DrawMode {
  Lines,
  Triangles,
}

impl DrawMode {
  fn to_enum(&self) -> GLenum {
    match self {
      &Lines     => gl::LINES,
      &Triangles => gl::TRIANGLES,
    }
  }
}

impl<T: Clone> GLBuffer<T> {
  #[inline]
  /// Creates a new array of objects on the GPU.
  /// capacity is provided in units of size t_span.
  pub unsafe fn new(
      _gl: &GLContext,
      shader_program: Rc<Shader>,
      attribs: &[vertex::AttribData],
      t_span: uint,
      capacity: uint,
      mode: DrawMode) -> GLBuffer<T> {
    let capacity = capacity * t_span;
    let mut vertex_array = 0;
    let mut vertex_buffer = 0;

    // TODO(cgaebel): Error checking?

    unsafe {
      gl::GenVertexArrays(1, &mut vertex_array);
      gl::GenBuffers(1, &mut vertex_buffer);
    }

    match gl::GetError() {
      gl::NO_ERROR => {},
      err => fail!("OpenGL error 0x{:x}", err),
    }

    gl::BindVertexArray(vertex_array);
    gl::BindBuffer(gl::ARRAY_BUFFER, vertex_buffer);

    match gl::GetError() {
      gl::NO_ERROR => {},
      err => fail!("OpenGL error 0x{:x}", err),
    }

    let mut offset = 0;
    for attrib in attribs.iter() {
      let shader_attrib = glGetAttribLocation((*shader_program).id, attrib.name) as GLuint;
      assert!(shader_attrib != -1, "shader attribute \"{}\" not found", attrib.name);

      gl::EnableVertexAttribArray(shader_attrib);
      unsafe {
        gl::VertexAttribPointer(
          shader_attrib,
          attrib.size as i32,
          gl::FLOAT,
          gl::FALSE as GLboolean,
          mem::size_of::<T>() as i32,
          ptr::null().offset(offset),
        );
      }
      offset += (attrib.size * mem::size_of::<GLfloat>()) as int;
    }

    match gl::GetError() {
      gl::NO_ERROR => {},
      err => fail!("OpenGL error 0x{:x}", err),
    }

    unsafe {
      // Check that the attribs are sized correctly.
      assert_eq!(offset, mem::size_of::<T>() as int);

      gl::BufferData(
        gl::ARRAY_BUFFER,
        (capacity * mem::size_of::<T>()) as GLsizeiptr,
        ptr::null(),
        gl::DYNAMIC_DRAW,
      );

      match gl::GetError() {
        gl::NO_ERROR => {},
        gl::OUT_OF_MEMORY => fail!("Out of VRAM"),
        err => fail!("OpenGL error 0x{:x}", err),
      }
    }

    GLBuffer {
      vertex_array:  vertex_array,
      vertex_buffer: vertex_buffer,
      t_span: t_span,
      length: 0,
      capacity: capacity,
      shader: shader_program,
      mode: mode.to_enum(),
      buffer: Vec::new(),
    }
  }

  /// Analog of vec::Vector::swap_remove`, but for GLBuffer data.
  pub fn swap_remove(&mut self, _gl: &GLContext, i: uint) {
    let i = i * self.t_span;
    assert!(i < self.length);
    self.length -= self.t_span;
    if i == self.length {
      // just remove, no swap.
      return;
    }

    let va = self.vertex_array;
    let vb = self.vertex_buffer;

    gl::BindVertexArray(va);
    gl::BindBuffer(gl::ARRAY_BUFFER, vb);

    let byte_size = mem::size_of::<T>() as i64;
    gl::CopyBufferSubData(
      gl::ARRAY_BUFFER,
      gl::ARRAY_BUFFER,
      self.length as i64 * byte_size,
      i as i64 * byte_size,
      self.t_span as i64 * byte_size
    );
  }

  /// Add more data into this buffer; the data are not pushed to OpenGL until
  /// flush() is called!
  pub fn push(&mut self, vs: &[T]) {
    assert!(vs.len() % self.t_span == 0);
    assert!(
      self.length + self.buffer.len() + vs.len() <= self.capacity,
      "GLBuffer::push into a {}/{} full GLbuffer",
      self.length + self.buffer.len(),
      self.capacity
    );

    self.buffer.push_all(vs);
  }

  /// Send all the in-memory buffered data to OpenGL buffers.
  pub fn flush(&mut self, _gl: &GLContext) {
    assert!(self.buffer.len() % self.t_span == 0);
    if self.buffer.len() > 0 {
      self.update(_gl, self.length / self.t_span, self.buffer.slice(0, self.buffer.len()));

      self.length += self.buffer.len();
      self.buffer.clear();
    }
  }

  pub fn update(&self, _gl: &GLContext, idx: uint, vs: &[T]) {
    assert!(vs.len() % self.t_span == 0);
    assert!(idx * self.t_span + vs.len() <= self.capacity);

    gl::BindVertexArray(self.vertex_array);
    gl::BindBuffer(gl::ARRAY_BUFFER, self.vertex_buffer);

    let byte_size = mem::size_of::<T>();
    unsafe {
      gl::BufferSubData(
          gl::ARRAY_BUFFER,
          (byte_size * idx * self.t_span) as i64,
          (byte_size * vs.len()) as i64,
          aligned_slice_to_ptr(vs.slice(0, vs.len()), mem::size_of::<GLfloat>())
      );
    }

    match gl::GetError() {
      gl::NO_ERROR => {},
      err => fail!("OpenGL error 0x{:x} in GLBuffer::update", err),
    }
  }

  #[inline]
  /// Draws all the queued triangles to the screen.
  pub fn draw(&self, gl: &GLContext) {
    self.draw_slice(gl, 0, self.length);
  }

  /// Draw some subset of the triangle array.
  pub fn draw_slice(&self, gl: &GLContext, start: uint, len: uint) {
    gl.use_shader(self.shader.deref(), |_gl| {
      gl::BindVertexArray(self.vertex_array);
      gl::BindBuffer(gl::ARRAY_BUFFER, self.vertex_buffer);

      gl::DrawArrays(self.mode, start as i32, len as i32);
    });
  }
}

#[unsafe_destructor]
impl<T> Drop for GLBuffer<T> {
  #[inline]
  fn drop(&mut self) {
    unsafe {
      gl::DeleteBuffers(1, &self.vertex_buffer);
      gl::DeleteVertexArrays(1, &self.vertex_array);
    }
  }
}

// TODO(cgaebel): Handle texture creation from an SDL surface.

/// A GPU-allocated texture.
pub struct Texture {
  pub id: GLuint,
}

impl Texture {
  pub fn bind_2d(&self, _gl: &GLContext) {
    gl::BindTexture(gl::TEXTURE_2D, self.id);
  }

  #[allow(dead_code)]
  pub fn bind_3d(&self, _gl: &GLContext) {
    gl::BindTexture(gl::TEXTURE_3D, self.id);
  }
}

impl Drop for Texture {
  fn drop(&mut self) {
    unsafe { gl::DeleteTextures(1, &self.id); }
  }
}

pub struct Camera {
  // projection matrix components
  pub translation: Mat4<GLfloat>,
  pub rotation: Mat4<GLfloat>,
  pub fov: Mat4<GLfloat>,
  pub position: Vec3<GLfloat>,
}

/// Create a 3D translation matrix.
pub fn translation(t: Vec3<GLfloat>) -> Mat4<GLfloat> {
  Mat4 {
    m11: 1.0, m12: 0.0, m13: 0.0, m14: t.x,
    m21: 0.0, m22: 1.0, m23: 0.0, m24: t.y,
    m31: 0.0, m32: 0.0, m33: 1.0, m34: t.z,
    m41: 0.0, m42: 0.0, m43: 0.0, m44: 1.0,
  }
}

pub fn from_axis_angle3(axis: Vec3<GLfloat>, angle: GLfloat) -> Mat3<GLfloat> {
  let (s, c) = angle.sin_cos();
  let Vec3 { x: xs, y: ys, z: zs } = axis * s;
  let vsub1c = axis * (1.0 - c);
  Outer::outer(&vsub1c, &vsub1c) +
    Mat3 {
      m11: c,   m12: -zs, m13: ys,
      m21: zs,  m22: c,   m23: -xs,
      m31: -ys, m32: xs,  m33: c,
    }
}

/// Create a matrix from a rotation around an arbitrary axis.
pub fn from_axis_angle4(axis: Vec3<GLfloat>, angle: GLfloat) -> Mat4<GLfloat> {
  let (s, c) = angle.sin_cos();
  let sub1c = 1.0 - c;
  let Vec3 { x: xs, y: ys, z: zs } = axis * s;
  let (x, y, z) = (axis.x, axis.y, axis.z);
  Mat4 {
    m11: x*x*sub1c + c,  m12: x*y*sub1c - zs, m13: x*z*sub1c + ys, m14: 0.0,
    m21: y*x*sub1c + zs, m22: y*y*sub1c + c,  m23: y*z*sub1c - xs, m24: 0.0,
    m31: z*x*sub1c - ys, m32: z*y*sub1c + xs, m33: z*z*sub1c + c,  m34: 0.0,
    m41: 0.0,            m42: 0.0,            m43: 0.0,            m44: 1.0,
  }
}

/// Create a 3D perspective initialization matrix.
pub fn perspective(fovy: GLfloat, aspect: GLfloat, near: GLfloat, far: GLfloat) -> Mat4<GLfloat> {
  Mat4 {
    m11: fovy / aspect, m12: 0.0,   m13: 0.0,                         m14: 0.0,
    m21: 0.0,           m22: fovy,  m23: 0.0,                         m24: 0.0,
    m31: 0.0,           m32: 0.0,   m33: (near + far) / (near - far), m34: 2.0 * near * far / (near - far),
    m41: 0.0,           m42: 0.0,   m43: -1.0,                        m44: 0.0,
  }
}

#[allow(dead_code)]
pub fn ortho(left: GLfloat, right: GLfloat, bottom: GLfloat, top: GLfloat, near: GLfloat, far: GLfloat) -> Mat4<GLfloat> {
  Mat4 {
    m11: 2.0 / (right - left),  m12: 0.0,                   m13: 0.0,                 m14: (left + right) / (left - right),
    m21: 0.0,                   m22: 2.0 / (top - bottom),  m23: 0.0,                 m24: (bottom + top) / (bottom - top),
    m31: 0.0,                   m32: 0.0,                   m33: 2.0 / (near - far),  m34: (near + far) / (near - far),
    m41: 0.0,                   m42: 0.0,                   m43: 0.0,                 m44: 1.0,
  }
}

/// Create a XY symmetric ortho matrix.
pub fn sortho(dx: GLfloat, dy: GLfloat, near: GLfloat, far: GLfloat) -> Mat4<GLfloat> {
  Mat4 {
    m11: 1.0 / dx,  m12: 0.0,       m13: 0.0,                 m14: 0.0,
    m21: 0.0,       m22: 1.0 / dy,  m23: 0.0,                 m24: 0.0,
    m31: 0.0,       m32: 0.0,       m33: 2.0 / (near - far),  m34: (near + far) / (near - far),
    m41: 0.0,       m42: 0.0,       m43: 0.0,                 m44: 1.0,
  }
}

impl Camera {
  /// this Camera sits at (0, 0, 0),
  /// maps [-1, 1] in x horizontally,
  /// maps [-1, 1] in y vertically,
  /// and [0, -1] in z in depth.
  pub fn unit() -> Camera {
    Camera {
      translation: Eye::new_identity(4),
      rotation: Eye::new_identity(4),
      fov: Eye::new_identity(4),
      position: Vec3::new(0.0, 0.0, 0.0),
    }
  }

  pub fn projection_matrix(&self) -> Mat4<GLfloat> {
    self.fov * self.rotation * self.translation
  }

  /// Shift the camera by a vector.
  pub fn translate(&mut self, v: Vec3<GLfloat>) {
    self.translation = self.translation * translation(-v);
    self.position = self.position + v;
  }

  /// Rotate about a given vector, by `r` radians.
  pub fn rotate(&mut self, v: Vec3<GLfloat>, r: GLfloat) {
    self.rotation = self.rotation * from_axis_angle4(v, -r);
  }
}

/// A handle to an OpenGL context. Only create one of these per thread.
#[deriving(Send)]
pub struct GLContext {
  scache: cstr_cache::CStringCache,
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
  pub fn enable_depth_buffer(&self) {
    gl::Enable(gl::DEPTH_TEST);
    gl::DepthFunc(gl::LESS);
    gl::ClearDepth(100.0);
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
  fn compile_shader(&self, src: &str, ty: GLenum) -> GLuint {
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
        fail!("{}", str::from_utf8(buf.slice(0, buf.len())).expect("ShaderInfoLog not valid utf8"));
      }
    }
    shader
  }

  /// Links a vertex and fragment shader, returning the id of the
  /// resulting program.
  fn link_shader(&self, vertex_shader: GLuint, fragment_shader: GLuint) -> GLuint {
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
            fail!("{}", str::from_utf8(buf.slice(0, buf.len())).expect("ProgramInfoLog not valid utf8"));
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
