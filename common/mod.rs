#![deny(missing_docs)]
#![deny(warnings)]

//! Data structures and functions shared between server and client.

#![feature(test)]
#![feature(unboxed_closures)]
#![feature(range_inclusive)]
#![feature(iter_cmp)]

extern crate cgmath;
#[macro_use]
extern crate log;
extern crate nanomsg;
extern crate num;
extern crate stopwatch;
extern crate test;
extern crate time;

#[macro_use]
extern crate serialize as _serialize;

pub mod block_position;
pub mod color;
pub mod communicate;
pub mod cube_shell;
pub mod entity;
pub mod id_allocator;
pub mod interval_timer;
pub mod lod;
pub mod range_abs;
pub mod socket;
pub mod surroundings_iter;
pub mod surroundings_loader;
pub mod terrain_block;

pub use _serialize as serialize;
