//! The entry point.
#![crate_type = "bin"]
#![deny(warnings)]
#![deny(missing_docs)]
#![feature(globs)]
#![feature(macro_rules)]
#![feature(unsafe_destructor)]
#![feature(phase)]

extern crate gl;
extern crate libc;
#[phase(plugin, link)]
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
mod interval_timer;
mod player;
mod process_event;
mod light;
mod loader;
mod main;
mod mob;
mod octree;
mod physics;
mod range_abs;
mod render;
mod shader;
mod state;
mod stopwatch;
mod surroundings_loader;
mod terrain_vram_buffers;
mod terrain;
mod ttf;
mod update;
mod vertex;

#[allow(dead_code)]
fn main() {
  return main::main();
}
