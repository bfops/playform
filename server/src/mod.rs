//! This crate contains server-only components of Playform.

#![allow(let_and_return)]
#![allow(match_ref_pats)]
#![allow(type_complexity)]
#![deny(missing_docs)]
#![deny(warnings)]

#![feature(io)]
#![feature(main)]
#![feature(plugin)]
#![feature(test)]
#![feature(unboxed_closures)]

#![plugin(clippy)]

extern crate cgmath;
extern crate common;
extern crate env_logger;
#[macro_use]
extern crate log;
extern crate nanomsg;
extern crate noise;
extern crate num;
extern crate rand;
extern crate stopwatch;
extern crate terrain;
extern crate test;
extern crate thread_scoped;
extern crate time;
extern crate voxel;

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
mod terrain_loader;
mod update_gaia;
mod update_world;
