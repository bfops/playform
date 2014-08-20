//! An ownership-semantics based handle to OpenGL. This prevents us from
//! accidentally modifying OpenGL state from multiple threads.
//!
//! GLW stands for "OpenGL wrapper".
#![feature(globs)]
#![feature(unsafe_destructor)]

extern crate gl;
extern crate libc;
extern crate nalgebra;

use libc::types::common::c95;
use nalgebra::na::{Mat3, Mat4, Vec3, Eye, Outer};
use std::cmp;
use std::mem;
use std::ptr;
use std::raw;
use std::rc::Rc;
use std::str;
use queue::Queue;

pub mod camera;
pub mod color;
mod cstr_cache;
pub mod gl_buffer;
pub mod gl_context;
pub mod queue;
pub mod shader;
pub mod texture;
pub mod vertex;
