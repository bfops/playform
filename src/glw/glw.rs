//! An ownership-semantics based handle to OpenGL. This prevents us from
//! accidentally modifying OpenGL state from multiple threads.
//!
//! GLW stands for "OpenGL wrapper".
#![feature(globs)]
#![feature(phase)]
#![feature(unsafe_destructor)]

extern crate gl;
extern crate libc;
#[phase(plugin, link)]
extern crate log;
extern crate nalgebra;

pub mod camera;
pub mod color;
mod cstr_cache;
pub mod gl_buffer;
pub mod gl_context;
pub mod light;
pub mod queue;
pub mod shader;
pub mod texture;
pub mod vertex;
