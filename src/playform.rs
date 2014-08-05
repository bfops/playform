//! The entry point, bitch.
#![crate_type = "bin"]
#![deny(warnings)]
#![deny(missing_doc)]
#![feature(globs)]
#![feature(macro_rules)]
#![feature(unsafe_destructor)]

// TODO(cgaebel): This is just to make the `time` macro work. I'm not sure how
// to disable warnings only in a macro.
#![allow(unused_unsafe)]

extern crate gl;
extern crate libc;
extern crate nalgebra;
extern crate ncollide3df32;
extern crate piston;
extern crate sdl2;
extern crate sdl2_game_window;

mod color;
mod cstr_cache;
mod fontloader;
mod glw;
mod main;
mod octree;
mod stopwatch;
mod ttf;
mod vertex;

#[allow(dead_code)]
fn main() {
  return main::main();
}
