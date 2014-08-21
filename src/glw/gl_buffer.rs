use gl;
use gl::types::*;
use gl_context::*;
use libc::types::common::c95;
use queue::Queue;
use shader::*;
use std::cmp;
use std::mem;
use std::ptr;
use std::raw;
use std::rc::Rc;
use vertex;

// TODO: Don't 1-1 vertex array objects with vertex buffers

/// Gets the id number for a given input of the shader program.
#[allow(non_snake_case_functions)]
fn glGetAttribLocation(shader_program: GLuint, name: &str) -> GLint {
  name.with_c_str(|ptr| unsafe { gl::GetAttribLocation(shader_program, ptr) })
}

/// Ensures a slice has a given alignment, and converts it to a raw pointer.
unsafe fn aligned_slice_to_ptr<T, U>(vs: &[T], alignment: uint) -> *const U {
  let vs_as_slice : raw::Slice<T> = mem::transmute(vs);
  assert!(
    (vs_as_slice.data as uint & (alignment - 1) == 0),
    "0x{:x} not {}-aligned",
    vs_as_slice.data as uint,
    alignment
  );
  assert!(vs_as_slice.data != ptr::null());
  vs_as_slice.data as *const U
}

/// A fixed-capacity array of GLfloats passed to OpenGL.
pub struct GLfloatBuffer {
  pub vertex_array: u32,
  pub vertex_buffer: u32,
  pub length: uint,
  pub capacity: uint,
  pub shader: Rc<Shader>,
  /// How to draw this buffer. Ex: gl::LINES, gl::TRIANGLES, etc.
  pub mode: GLenum,
  /// size of vertex attribs, in GLfloats.
  pub attrib_span: uint,
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

impl GLfloatBuffer {
  #[inline]
  /// Creates a new array of objects on the GPU.
  /// capacity is provided in units of size slice_span.
  pub unsafe fn new(
      _gl: &GLContext,
      shader_program: Rc<Shader>,
      attribs: &[vertex::AttribData],
      capacity: uint,
      mode: DrawMode) -> GLfloatBuffer {
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
    let attrib_span = {
      let mut attrib_span = 0;
      for attrib in attribs.iter() {
        attrib_span += attrib.size;
      }
      attrib_span * mem::size_of::<GLfloat>()
    };
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
          attrib_span as i32,
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
      gl::BufferData(
        gl::ARRAY_BUFFER,
        (capacity * mem::size_of::<GLfloat>()) as GLsizeiptr,
        ptr::null(),
        gl::DYNAMIC_DRAW,
      );

      match gl::GetError() {
        gl::NO_ERROR => {},
        gl::OUT_OF_MEMORY => fail!("Out of VRAM"),
        err => fail!("OpenGL error 0x{:x}", err),
      }
    }

    GLfloatBuffer {
      vertex_array:  vertex_array,
      vertex_buffer: vertex_buffer,
      length: 0,
      capacity: capacity,
      shader: shader_program,
      mode: mode.to_enum(),
      attrib_span: attrib_span / mem::size_of::<GLfloat>(),
    }
  }

  pub fn len(&self) -> uint {
    self.length
  }

  pub fn capacity(&self) -> uint {
    self.capacity
  }

  /// Analog of `std::vec::Vec::swap_remove`, but for GLfloatBuffer data.
  pub fn swap_remove(&mut self, _gl: &GLContext, i: uint, count: uint) {
    self.length -= count;
    assert!(i <= self.length);

    // In the `i == self.length` case, we don't bother with the swap;
    // decreasing `self.length` is enough.

    if i < self.length {
      assert!(
        i <= self.length - count,
        "GLfloatBuffer::swap_remove would cause copy in overlapping regions"
      );

      let va = self.vertex_array;
      let vb = self.vertex_buffer;

      gl::BindVertexArray(va);
      gl::BindBuffer(gl::ARRAY_BUFFER, vb);

      let byte_size = mem::size_of::<GLfloat>() as i64;
      gl::CopyBufferSubData(
        gl::ARRAY_BUFFER,
        gl::ARRAY_BUFFER,
        self.length as i64 * byte_size,
        i as i64 * byte_size,
        count as i64 * byte_size
      );
    }
  }

  /// Add more data into this buffer.
  pub unsafe fn push(&mut self, gl: &GLContext, vs: *const GLfloat, count: uint) {
    assert!(
      self.length + count <= self.capacity,
      "GLfloatBuffer::push {} into a {}/{} full GLfloatBuffer",
      count,
      self.length,
      self.capacity
    );

    self.update_inner(gl, self.length, vs, count);
    self.length += count;
  }

  pub fn update(&self, gl: &GLContext, idx: uint, vs: *const GLfloat, count: uint) {
    assert!(idx + count <= self.length);
    self.update_inner(gl, idx, vs, count);
  }

  fn update_inner(&self, _gl: &GLContext, idx: uint, vs: *const GLfloat, count: uint) {
    assert!(idx + count <= self.capacity);

    gl::BindVertexArray(self.vertex_array);
    gl::BindBuffer(gl::ARRAY_BUFFER, self.vertex_buffer);

    let byte_size = mem::size_of::<GLfloat>();
    unsafe {
      gl::BufferSubData(
          gl::ARRAY_BUFFER,
          (byte_size * idx) as i64,
          (byte_size * count) as i64,
          mem::transmute(vs)
      );
    }

    gl::Flush();
    gl::Finish();

    match gl::GetError() {
      gl::NO_ERROR => {},
      err => fail!("OpenGL error 0x{:x} in GLfloatBuffer::update", err),
    }
  }

  #[inline]
  /// Draws all the queued triangles to the screen.
  pub fn draw(&self, gl: &GLContext) {
    self.draw_slice(gl, 0, self.length);
  }

  /// Draw some subset of the triangle array.
  pub fn draw_slice(&self, gl: &GLContext, start: uint, len: uint) {
    assert!(start + len <= self.len());

    gl.use_shader(self.shader.deref(), |_gl| {
      gl::BindVertexArray(self.vertex_array);
      gl::BindBuffer(gl::ARRAY_BUFFER, self.vertex_buffer);

      gl::DrawArrays(self.mode, start as i32, (len / self.attrib_span) as i32);
    });
  }
}

#[unsafe_destructor]
impl Drop for GLfloatBuffer {
  #[inline]
  fn drop(&mut self) {
    unsafe {
      gl::DeleteBuffers(1, &self.vertex_buffer);
      gl::DeleteVertexArrays(1, &self.vertex_array);
    }
  }
}

/// A `GLfloatBuffer` that pushes slices of data at a time.
/// These slices are expected to be a fixed size (or multiples of that size).
/// Indexing operations and lengths are in terms of contiguous blocks of that
/// size (i.e. refering to index 2 when `slice_span` is 3 means referring to a
/// contiguous block of size 3 starting at index 6 in the underlying GLfloatBuffer.
pub struct GLSliceBuffer<T> {
  pub gl_buffer: GLfloatBuffer,
  /// Each index in the GLfloatBuffer is the index of a contiguous block of
  /// `slice_span` elements.
  pub slice_span: uint,

  /// in-memory buffer before sending to OpenGL.
  pub buffer: Queue<T>,
}

impl<T: Clone> GLSliceBuffer<T> {
  fn glfloat_ratio() -> uint {
    assert!(mem::size_of::<T>() % mem::size_of::<GLfloat>() == 0);
    mem::size_of::<T>() / mem::size_of::<GLfloat>()
  }

  pub unsafe fn new(
    gl: &GLContext,
    shader_program: Rc<Shader>,
    attribs: &[vertex::AttribData],
    slice_span: uint,
    capacity: uint,
    mode: DrawMode
  ) -> GLSliceBuffer<T> {
    let capacity = capacity * slice_span;
    let gl_buffer =
      GLfloatBuffer::new(
        gl,
        shader_program,
        attribs,
        capacity * GLSliceBuffer::<T>::glfloat_ratio(),
        mode
      );
    GLSliceBuffer {
      gl_buffer: gl_buffer,
      slice_span: slice_span,
      buffer: Queue::new(capacity),
    }
  }

  pub fn len(&self) -> uint {
    self.gl_buffer.len() / GLSliceBuffer::<T>::glfloat_ratio() + self.buffer.len()
  }

  pub fn capacity(&self) -> uint {
    self.gl_buffer.capacity() / GLSliceBuffer::<T>::glfloat_ratio()
  }

  pub fn swap_remove(&mut self, gl: &GLContext, i: uint) {
    let i = i * self.slice_span;
    assert!(i < self.len());
    assert!(self.gl_buffer.len() % GLSliceBuffer::<T>::glfloat_ratio() == 0);
    let gl_len = self.gl_buffer.len() / GLSliceBuffer::<T>::glfloat_ratio();
    if i < gl_len {
      let i = i * GLSliceBuffer::<T>::glfloat_ratio();
      self.gl_buffer.swap_remove(gl, i, self.slice_span * GLSliceBuffer::<T>::glfloat_ratio());
    } else {
      self.buffer.swap_remove(i - gl_len, self.slice_span);
    }
  }

  /// Add more data into this buffer; the data are not pushed to OpenGL until
  /// flush() is called!
  pub fn push(&mut self, vs: &[T]) {
    assert!(vs.len() % self.slice_span == 0);
    assert!(self.len() + vs.len() <= self.capacity(),
      "GLSliceBuffer::push {} into a {}/{} full GLSliceBuffer",
      vs.len(),
      self.len(),
      self.capacity()
    );

    let prev_len = self.len();

    self.buffer.push_all(vs);

    assert!(self.len() == prev_len + vs.len());
  }

  pub fn flush(&mut self, gl: &GLContext, max: Option<uint>) {
    if self.buffer.is_empty() {
      return;
    }

    assert!(self.buffer.len() % self.slice_span == 0);
    assert!(self.len() <= self.capacity());

    let prev_len = self.len();

    let count = match max {
      None => self.buffer.len(),
      Some(x) => cmp::min(x * self.slice_span, self.buffer.len()),
    };

    unsafe {
      let (l, h) = self.buffer.slices(0, count);
      let s = mem::size_of::<GLfloat>();
      self.gl_buffer.push(
        gl,
        aligned_slice_to_ptr(l, s),
        l.len() * GLSliceBuffer::<T>::glfloat_ratio()
      );
      self.gl_buffer.push(
        gl,
        aligned_slice_to_ptr(h, s),
        h.len() * GLSliceBuffer::<T>::glfloat_ratio()
      );
    }

    self.buffer.pop(count);

    assert!(self.len() == prev_len);
  }

  pub fn update(&self, gl: &GLContext, idx: uint, vs: &[T]) {
    assert!(vs.len() % self.slice_span == 0);
    let s = mem::size_of::<GLfloat>();
    unsafe {
      self.gl_buffer.update(
        gl,
        idx * self.slice_span * GLSliceBuffer::<T>::glfloat_ratio(),
        aligned_slice_to_ptr(vs, s),
        vs.len() * GLSliceBuffer::<T>::glfloat_ratio()
      );
    }
  }

  pub fn draw(&self, gl: &GLContext) {
    self.gl_buffer.draw(gl);
  }

  pub fn draw_slice(&self, gl: &GLContext, start: uint, len: uint) {
    let s = self.slice_span * GLSliceBuffer::<T>::glfloat_ratio();
    self.gl_buffer.draw_slice(gl, start * s, len * s);
  }
}
