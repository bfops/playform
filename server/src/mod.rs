//! This crate contains server-only components of Playform.

#![deny(missing_docs)]
#![deny(warnings)]

#![feature(iter_cmp)]
#![feature(io)]
#![feature(main)]
#![feature(range_inclusive)]
#![feature(test)]
#![feature(unboxed_closures)]
#![feature(vec_push_all)]

extern crate cgmath;
extern crate common;
extern crate env_logger;
#[macro_use]
extern crate log;
extern crate noise;
extern crate num;
extern crate rand;
extern crate stopwatch;
extern crate test;
extern crate thread_scoped;
extern crate time;

mod client_recv_thread;
mod in_progress_terrain;
mod init_mobs;
mod main;
mod mob;
mod octree;
mod physics;
mod player;
mod server;
mod sun;
mod terrain;
mod terrain_loader;
mod update_gaia;
mod update_world;
mod voxel;
