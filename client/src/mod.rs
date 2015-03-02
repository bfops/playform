//! This crate contains client-only components of Playform.

#![deny(missing_docs)]
#![deny(warnings)]

#![feature(core)]
#![feature(collections)]
#![feature(main)]
#![feature(old_io)]
#![feature(old_path)]
#![feature(std_misc)]
#![feature(test)]
#![feature(unboxed_closures)]
#![feature(unsafe_destructor)]

extern crate cgmath;
extern crate common;
extern crate env_logger;
extern crate gl;
#[macro_use]
extern crate log;
extern crate libc;
extern crate nanomsg;
extern crate "rustc-serialize" as rustc_serialize;
extern crate sdl2;
extern crate "sdl2-sys" as sdl2_sys;
extern crate test;
extern crate time;
extern crate yaglw;

mod camera;
mod client;
mod client_update;
mod fontloader;
mod hud;
mod light;
mod load_terrain;
mod main;
mod mob_buffers;
mod process_event;
mod render;
mod server_recv_thread;
mod server_send_thread;
mod server_update;
mod shaders;
mod surroundings_thread;
mod terrain_buffers;
mod terrain_load_thread;
mod ttf;
mod update_surroundings;
mod view;
mod view_thread;
mod view_update;
