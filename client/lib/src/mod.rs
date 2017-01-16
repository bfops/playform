//! This crate contains client-only components of Playform.

#![deny(missing_docs)]
#![deny(warnings)]

#![feature(test)]
#![feature(type_ascription)]
#![feature(unboxed_closures)]

extern crate bincode;
extern crate cgmath;
extern crate collision;
extern crate common;
extern crate fnv;
extern crate gl;
extern crate hound;
extern crate image;
extern crate isosurface_extraction;
#[macro_use]
extern crate log;
extern crate libc;
extern crate num;
extern crate portaudio;
extern crate rand;
extern crate sdl2;
extern crate sdl2_sys;
extern crate stopwatch;
extern crate rustc_serialize;
extern crate test;
extern crate thread_scoped;
extern crate time;
extern crate voxel_data;
extern crate yaglw;

#[allow(missing_docs)]
pub mod audio;
#[allow(missing_docs)]
pub mod audio_loader;
#[allow(missing_docs)]
pub mod audio_thread;
#[allow(missing_docs)]
pub mod chunk;
#[allow(missing_docs)]
pub mod client;
#[allow(missing_docs)]
pub mod hud;
#[allow(missing_docs)]
pub mod lod;
#[allow(missing_docs)]
pub mod process_event;
#[allow(missing_docs)]
pub mod record_book;
#[allow(missing_docs)]
pub mod run;
#[allow(missing_docs)]
pub mod server;
#[allow(missing_docs)]
pub mod server_update;
#[allow(missing_docs)]
pub mod terrain;
#[allow(missing_docs)]
pub mod terrain_mesh;
#[allow(missing_docs)]
pub mod update_thread;
#[allow(missing_docs)]
pub mod vertex;
#[allow(missing_docs)]
pub mod view;

pub use run::run;
