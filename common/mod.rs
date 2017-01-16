//! Data structures and functions shared between server and client.

#![deny(missing_docs)]
#![deny(warnings)]

#![feature(box_syntax)]
#![feature(fn_traits)]
#![feature(test)]
#![feature(unboxed_closures)]

extern crate cgmath;
extern crate collision;
extern crate fnv;
extern crate isosurface_extraction;
#[macro_use]
extern crate log;
extern crate nanomsg;
extern crate num;
extern crate rustc_serialize;
extern crate stopwatch;
extern crate test;
extern crate time;
extern crate voxel_data;

pub mod chunk;
pub mod closure_series;
pub mod color;
pub mod cube_shell;
pub mod entity_id;
pub mod fnv_map;
pub mod fnv_set;
pub mod id_allocator;
pub mod interval_timer;
pub mod protocol;
pub mod range_abs;
pub mod socket;
pub mod surroundings_loader;
pub mod voxel;
