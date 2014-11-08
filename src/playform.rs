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
extern crate glw;
extern crate event;
extern crate input;
extern crate libc;
#[phase(plugin, link)]
extern crate log;
extern crate nalgebra;
extern crate ncollide;
extern crate noise;
extern crate png;
extern crate sdl2;
extern crate sdl2_window;
extern crate shader_version;

// so time! macro is defined in main
mod stopwatch;

mod common;
mod event_handler;
mod fontloader;
mod id_allocator;
mod player;
mod loader;
mod main;
mod mob;
mod octree;
mod physics;
mod render;
mod shader;
mod state;
mod terrain;
mod ttf;
mod update;

#[allow(dead_code)]
fn main() {
  return main::main();
}
