//! The entry point.
#![crate_type = "bin"]
#![deny(missing_docs)]
#![deny(warnings)]
#![feature(old_orphan_check)]
#![feature(slicing_syntax)]
#![feature(unboxed_closures)]
#![feature(unsafe_destructor)]

extern crate gl;
extern crate libc;
#[macro_use]
extern crate log;
extern crate nalgebra;
extern crate ncollide;
extern crate noise;
extern crate sdl2;
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
mod main;
mod mob;
mod octree;
mod physics;
mod player;
mod process_event;
mod range_abs;
mod render;
mod shader;
mod state;
mod stopwatch;
mod surroundings_iter;
mod surroundings_loader;
mod terrain;
mod terrain_block;
mod terrain_game_loader;
mod terrain_vram_buffers;
mod ttf;
mod update;
mod vertex;

#[allow(dead_code)]
fn main() {
  return main::main();
}
