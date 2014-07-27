//! OpenGL arrays of verticies, all shaded with the same program.
use gl;
use gl::types::*;
use std::mem;
use std::ptr;
use std::raw;
use libc::types::common::c95;
use vertex;

/// Gets the id number for a given input of the shader program.
#[allow(non_snake_case_functions)]
unsafe fn glGetAttribLocation(shader_program: GLuint, name: &str) -> GLint {
  name.with_c_str(|ptr| gl::GetAttribLocation(shader_program, ptr))
}

unsafe fn aligned_slice_to_ptr<T>(vs: &[T], alignment: uint) -> *const c95::c_void {
  let vs_as_slice : raw::Slice<T> = mem::transmute(vs);
  assert_eq!(vs_as_slice.data as uint & (alignment - 1), 0);
  vs_as_slice.data as *const c95::c_void
}

/// A fixed-capacity array of GLfloat-based structures passed to OpenGL.
pub struct GLBuffer<T> {
  vertex_array: u32,
  vertex_buffer: u32,
  length: uint,
  capacity: uint,
}

impl<T: Clone> GLBuffer<T> {
  #[inline]
  /// An empty `GLBuffer`.
  pub unsafe fn null() -> GLBuffer<T> {
    GLBuffer {
      vertex_array: -1 as u32,
      vertex_buffer: -1 as u32,
      length: 0,
      capacity: 0,
    }
  }

  #[inline]
  /// Creates a new array of objects on the GPU.
  pub unsafe fn new(shader_program: GLuint, attribs: &[vertex::AttribData], capacity: uint) -> GLBuffer<T> {
    let mut vertex_array = 0;
    let mut vertex_buffer = 0;
    gl::GenVertexArrays(1, &mut vertex_array);
    gl::GenBuffers(1, &mut vertex_buffer);

    gl::BindVertexArray(vertex_array);
    gl::BindBuffer(gl::ARRAY_BUFFER, vertex_buffer);

    let mut offset = 0;
    for attrib in attribs.iter() {
      let shader_attrib = glGetAttribLocation(shader_program, attrib.name) as GLuint;
      assert!(shader_attrib != -1, "shader attribute \"{}\" not found", attrib.name);

      gl::EnableVertexAttribArray(shader_attrib);
      gl::VertexAttribPointer(
          shader_attrib,
          attrib.size as i32,
          gl::FLOAT,
          gl::FALSE as GLboolean,
          mem::size_of::<T>() as i32,
          ptr::null().offset(offset),
      );
      offset += (attrib.size * mem::size_of::<GLfloat>()) as int;
    }

    // Check that the attribs are sized correctly.
    assert_eq!(offset, mem::size_of::<T>() as int);

    gl::BufferData(
      gl::ARRAY_BUFFER,
      (capacity * mem::size_of::<T>()) as GLsizeiptr,
      ptr::null(),
      gl::DYNAMIC_DRAW,
    );

    GLBuffer {
      vertex_array: vertex_array,
      vertex_buffer: vertex_buffer,
      length: 0,
      capacity: capacity,
    }
  }

  /// Analog of vec::Vector::swap_remove`, but for GLBuffer data.
  pub unsafe fn swap_remove(&mut self, span: uint, i: uint) {
    let i = i * span;
    assert!(i < self.length);
    self.length -= span;
    if i == self.length {
      // just remove, no swap.
      return;
    }

    gl::BindVertexArray(self.vertex_array);
    gl::BindBuffer(gl::ARRAY_BUFFER, self.vertex_buffer);

    let byte_size = mem::size_of::<T>() as i64;
    gl::CopyBufferSubData(
      gl::ARRAY_BUFFER,
      gl::ARRAY_BUFFER,
      self.length as i64 * byte_size,
      i as i64 * byte_size,
      span as i64 * byte_size
    );
  }

  #[inline]
  /// Add a set of triangles to the set of triangles to render.
  pub unsafe fn push(&mut self, vs: &[T]) {
    assert!(
      self.length + vs.len() <= self.capacity,
      "GLBuffer::push: {} into a {}/{} full GLbuffer", vs.len(), self.length, self.capacity);

    gl::BindVertexArray(self.vertex_array);
    gl::BindBuffer(gl::ARRAY_BUFFER, self.vertex_buffer);

    let size = mem::size_of::<T>() as i64;
    gl::BufferSubData(
      gl::ARRAY_BUFFER,
      size * self.length as i64,
      size * vs.len() as i64,
      aligned_slice_to_ptr(vs, 4)
    );

    self.length += vs.len();
  }

  #[inline]
  /// Draws all the queued triangles to the screen.
  pub fn draw(&self, mode: GLenum) {
    self.draw_slice(mode, 0, self.length);
  }

  /// Draw some subset of the triangle array.
  pub fn draw_slice(&self, mode: GLenum, start: uint, len: uint) {
    gl::BindVertexArray(self.vertex_array);
    gl::BindBuffer(gl::ARRAY_BUFFER, self.vertex_buffer);

    gl::DrawArrays(mode, start as i32, len as i32);
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
