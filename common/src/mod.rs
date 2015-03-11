#![deny(missing_docs)]
#![deny(warnings)]

//! Data structures and functions shared between server and client.

#![feature(collections)]
#![feature(core)]
#![feature(io)]
#![feature(std_misc)]
#![feature(test)]
#![feature(unboxed_closures)]
#![feature(unsafe_destructor)]

extern crate cgmath;
#[macro_use]
extern crate log;
extern crate nanomsg;
extern crate "rustc-serialize" as rustc_serialize;
extern crate test;
extern crate time;

pub mod block_position;
pub mod color;
pub mod communicate;
pub mod cube_shell;
pub mod entity;
pub mod id_allocator;
pub mod interval_timer;
pub mod lod;
pub mod process_events;
pub mod range_abs;
pub mod socket;
pub mod stopwatch;
pub mod surroundings_iter;
pub mod surroundings_loader;
pub mod terrain_block;
