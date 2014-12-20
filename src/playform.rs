//! The entry point.
#![crate_type = "bin"]
#![deny(warnings)]
#![deny(missing_docs)]
#![feature(globs)]
#![feature(macro_rules)]
#![feature(unsafe_destructor)]
#![feature(phase)]

extern crate current;
extern crate gl;
extern crate event;
extern crate input;
extern crate libc;
#[phase(plugin, link)]
extern crate log;
extern crate nalgebra;
extern crate ncollide;
extern crate noise;
extern crate sdl2;
extern crate sdl2_window;
extern crate shader_version;
extern crate time;
extern crate yaglw;

mod camera;
mod color;
mod common;
mod event_handler;
mod fontloader;
mod id_allocator;
mod player;
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
