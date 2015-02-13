//! This crate contains server-only components of Playform.

#![deny(missing_docs)]
#![deny(warnings)]

#![feature(core)]
#![feature(collections)]
#![feature(hash)]
#![feature(io)]
#![feature(slicing_syntax)]
#![feature(std_misc)]
#![feature(test)]
#![feature(unboxed_closures)]
#![feature(unsafe_destructor)]

extern crate common;
#[macro_use]
extern crate log;
extern crate nalgebra;
extern crate ncollide_entities;
extern crate ncollide_queries;
extern crate noise;
extern crate opencl;
extern crate rand;
extern crate test;
extern crate time;

mod gaia_thread;
mod gaia_update;
mod in_progress_terrain;
mod init_mobs;
mod mob;
mod octree;
mod opencl_context;
mod physics;
mod player;
mod server;
mod server_thread;
mod server_update;
mod sun;
mod terrain;
mod update;

use common::communicate::{ClientToServer, ServerToClient};
use std::sync::mpsc::{Sender, Receiver};

#[allow(missing_docs)]
pub fn main(
  ups_from_client: Receiver<ClientToServer>,
  ups_to_client: Sender<ServerToClient>,
) {
  server_thread::server_thread(
    &ups_from_client,
    &ups_to_client,
  )
}
