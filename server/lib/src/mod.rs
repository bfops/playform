//! This crate contains server-only components of Playform.

#![deny(missing_docs)]
#![deny(warnings)]

extern crate bincode;
extern crate cgmath;
extern crate collision;
extern crate common;
#[macro_use]
extern crate log;
extern crate nanomsg;
extern crate num;
extern crate rand;
extern crate stopwatch;
extern crate terrain;
extern crate thread_scoped;
extern crate time;
extern crate voxel_data;

mod client_recv_thread;
mod entity;
mod in_progress_terrain;
mod init_mobs;
mod lod;
mod mob;
mod octree;
mod physics;
mod player;
mod run;
pub mod server;
mod sun;
mod terrain_loader;
pub mod update_gaia;
mod update_world;

pub use run::run;
