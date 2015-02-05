//! The entry point.
#![crate_type = "bin"]
#![deny(missing_docs)]
#![deny(warnings)]
#![feature(slicing_syntax)]
#![feature(unboxed_closures)]
#![feature(unsafe_destructor)]

#![feature(core)]
#![feature(collections)]
#![feature(hash)]
#![feature(io)]
#![feature(libc)]
#![feature(path)]
#![feature(std_misc)]
#![feature(test)]

extern crate gl;
extern crate libc;
#[macro_use]
extern crate log;
extern crate nalgebra;
extern crate ncollide;
extern crate noise;
extern crate opencl;
extern crate rand;
extern crate sdl2;
extern crate "sdl2-sys" as sdl2_sys;
extern crate test;
extern crate time;
extern crate yaglw;

mod camera;
mod color;
mod common;
mod cube_shell;
mod fontloader;
mod id_allocator;
mod in_progress_terrain;
mod interval_timer;
mod light;
mod loader;
mod logger;
mod lod_map;
mod main;
mod mob;
mod octree;
mod opencl_context;
mod physics;
mod player;
mod process_event;
mod range_abs;
mod render;
mod shaders;
mod state;
mod stopwatch;
mod surroundings_iter;
mod surroundings_loader;
mod terrain;
mod ttf;
mod update;
mod vertex;

#[allow(dead_code)]
fn main() {
  return main::main();
}
