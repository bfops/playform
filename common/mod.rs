#![deny(missing_docs)]
#![deny(warnings)]

//! Data structures and functions shared between server and client.

#![feature(box_syntax)]
#![feature(core)]
#![feature(iter_cmp)]
#![feature(plugin)]
#![feature(test)]
#![feature(unboxed_closures)]

#![plugin(clippy)]
#![allow(type_complexity)]

extern crate cgmath;
#[macro_use]
extern crate log;
extern crate nanomsg;
extern crate num;
extern crate rustc_serialize;
extern crate stopwatch;
extern crate test;
extern crate time;

pub mod block_position;
pub mod closure_series;
pub mod color;
pub mod communicate;
pub mod cube_shell;
pub mod entity;
pub mod id_allocator;
pub mod interval_timer;
pub mod lod;
pub mod range_abs;
pub mod socket;
pub mod surroundings_loader;
pub mod terrain_block;
