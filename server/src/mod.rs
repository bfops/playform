//! This crate contains server-only components of Playform.

#![deny(missing_docs)]
#![deny(warnings)]

#![feature(collections)]
#![feature(core)]
#![feature(duration)]
#![feature(main)]
#![feature(scoped)]
#![feature(test)]
#![feature(unboxed_closures)]

extern crate bit_svo;
extern crate cgmath;
extern crate common;
extern crate env_logger;
#[macro_use]
extern crate log;
extern crate noise;
extern crate num;
extern crate rand;
extern crate test;
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
mod update_gaia;
mod update_world;
