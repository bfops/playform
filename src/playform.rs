//! The entry point.
#![crate_type = "bin"]
#![deny(warnings)]
#![deny(missing_doc)]
#![feature(globs)]
#![feature(macro_rules)]
#![feature(unsafe_destructor)]
#![feature(phase)]

// TODO(cgaebel): This is just to make the `time` macro work. I'm not sure how
// to disable warnings only in a macro.
#![allow(unused_unsafe)]

extern crate gl;
extern crate glw;
extern crate input;
extern crate libc;
extern crate nalgebra;
extern crate ncollide3df32;
extern crate piston;
extern crate png;
extern crate sdl2;
extern crate sdl2_game_window;
extern crate shader_version;

mod common;
mod fontloader;
// so the time! macro is defined in main
mod stopwatch;
mod main;
mod octree;
mod physics;
mod ttf;

#[allow(dead_code)]
fn main() {
  return main::main();
}
