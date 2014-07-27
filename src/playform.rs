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

extern crate cgmath;
extern crate gl;
extern crate libc;
extern crate piston;
extern crate sdl2;
extern crate sdl2_game_window;

mod bounding_box;
mod color;
mod cstr_cache;
mod glbuffer;
mod main;
mod fontloader;
mod stopwatch;
mod ttf;
mod vertex;

#[allow(dead_code)]
fn main() {
  return main::main();
}
