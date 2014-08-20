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
  pub vertex_array: u32,
  pub vertex_buffer: u32,
  pub length:   uint,
  pub capacity: uint,
  pub shader: Rc<Shader>,
  /// How to draw this buffer. Ex: gl::LINES, gl::TRIANGLES, etc.
  pub mode: GLenum,
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
  /// capacity is provided in units of size slice_span.
  pub unsafe fn new(
      _gl: &GLContext,
      shader_program: Rc<Shader>,
      attribs: &[vertex::AttribData],
      capacity: uint,
      mode: DrawMode) -> GLBuffer<T> {
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
      assert!(
        offset == mem::size_of::<T>() as int,
        "GLBuffer attribs incorrectly sized"
      );

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
      length: 0,
      capacity: capacity,
      shader: shader_program,
      mode: mode.to_enum(),
    }
  }

  pub fn len(&self) -> uint {
    self.length
  }

  pub fn capacity(&self) -> uint {
    self.capacity
  }

  /// Analog of `std::vec::Vec::swap_remove`, but for GLBuffer data.
  pub fn swap_remove(&mut self, _gl: &GLContext, i: uint, count: uint) {
    self.length -= count;
    assert!(i <= self.length);

    // In the `i == self.length` case, we don't bother with the swap;
    // decreasing `self.length` is enough.

    if i < self.length {
      assert!(
        i <= self.length - count,
        "GLBuffer::swap_remove would cause copy in overlapping regions"
      );

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
        count as i64 * byte_size
      );
    }
  }

  /// Add more data into this buffer.
  pub fn push(&mut self, gl: &GLContext, vs: &[T]) {
    assert!(
      self.length + vs.len() <= self.capacity,
      "GLBuffer::push {} into a {}/{} full GLBuffer",
      vs.len(),
      self.length,
      self.capacity
    );

    self.update_inner(gl, self.length, vs);
    self.length += vs.len();
  }

  pub fn update(&self, gl: &GLContext, idx: uint, vs: &[T]) {
    assert!(idx + vs.len() <= self.length);
    self.update_inner(gl, idx, vs);
  }

  fn update_inner(&self, _gl: &GLContext, idx: uint, vs: &[T]) {
    assert!(idx + vs.len() <= self.capacity);

    gl::BindVertexArray(self.vertex_array);
    gl::BindBuffer(gl::ARRAY_BUFFER, self.vertex_buffer);

    let byte_size = mem::size_of::<T>();
    unsafe {
      gl::BufferSubData(
          gl::ARRAY_BUFFER,
          (byte_size * idx) as i64,
          (byte_size * vs.len()) as i64,
          aligned_slice_to_ptr(vs.as_slice(), mem::size_of::<GLfloat>())
      );
    }

    gl::Flush();
    gl::Finish();

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
    assert!(start + len <= self.len());

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

/// A `GLBuffer` that pushes slices of data at a time.
/// These slices are expected to be a fixed size (or multiples of that size).
/// Indexing operations and lengths are in terms of contiguous blocks of that
/// size (i.e. refering to index 2 when `slice_span` is 3 means referring to a
/// contiguous block of size 3 starting at index 6 in the underlying GLBuffer.
pub struct GLSliceBuffer<T> {
  pub gl_buffer: GLBuffer<T>,
  /// Each index in the GLBuffer is the index of a contiguous block of
  /// `slice_span` elements.
  pub slice_span: uint,

  /// in-memory buffer before sending to OpenGL.
  pub buffer: Queue<T>,
}

impl<T: Clone> GLSliceBuffer<T> {
  pub unsafe fn new(
    gl: &GLContext,
    shader_program: Rc<Shader>,
    attribs: &[vertex::AttribData],
    slice_span: uint,
    capacity: uint,
    mode: DrawMode
  ) -> GLSliceBuffer<T> {
    let capacity = capacity * slice_span;
    let gl_buffer = GLBuffer::new(gl, shader_program, attribs, capacity, mode);
    GLSliceBuffer {
      gl_buffer: gl_buffer,
      slice_span: slice_span,
      buffer: Queue::new(capacity),
    }
  }

  pub fn len(&self) -> uint {
    self.gl_buffer.len() + self.buffer.len()
  }

  pub fn capacity(&self) -> uint {
    self.gl_buffer.capacity()
  }

  pub fn swap_remove(&mut self, gl: &GLContext, i: uint) {
    let i = i * self.slice_span;
    assert!(i < self.len());
    if i < self.gl_buffer.len() {
      self.gl_buffer.swap_remove(gl, i, self.slice_span);
    } else {
      let slice_span = self.slice_span;
      let len = self.gl_buffer.len();
      self.buffer.swap_remove(i - len, slice_span);
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

    {
      let (l, h) = self.buffer.slices(0, count);
      self.gl_buffer.push(gl, l);
      self.gl_buffer.push(gl, h);
    }

    self.buffer.pop(count);

    assert!(self.len() == prev_len);
  }

  pub fn update(&self, gl: &GLContext, idx: uint, vs: &[T]) {
    assert!(vs.len() % self.slice_span == 0);
    self.gl_buffer.update(gl, idx * self.slice_span, vs);
  }

  pub fn draw(&self, gl: &GLContext) {
    self.gl_buffer.draw(gl);
  }

  pub fn draw_slice(&self, gl: &GLContext, start: uint, len: uint) {
    self.gl_buffer.draw_slice(gl, start * self.slice_span, len * self.slice_span);
  }
}
