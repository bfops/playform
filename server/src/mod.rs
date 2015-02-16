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
use common::id_allocator::IdAllocator;
use common::stopwatch::TimerSet;
use gaia_thread::gaia_thread;
use server::Server;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread::Thread;

#[allow(missing_docs)]
pub fn main(
  ups_from_client: Receiver<ClientToServer>,
  ups_to_client: Sender<ServerToClient>,
) {
  let timers = TimerSet::new();
  let mut owner_allocator = IdAllocator::new();
  let world = Server::new(&mut owner_allocator, &timers);

  let (ups_to_gaia_send, ups_to_gaia_recv) = channel();
  let (ups_from_gaia_send, ups_from_gaia_recv) = channel();
  let _gaia_thread = {
    let terrain = world.terrain_game_loader.terrain.clone();
    Thread::spawn(move || {
      gaia_thread(
        ups_to_gaia_recv,
        ups_from_gaia_send,
        terrain,
      );
    })
  };
  let ups_to_gaia = ups_to_gaia_send;
  let ups_from_gaia = ups_from_gaia_recv;

  server_thread::server_thread(
    timers,
    world,
    ups_from_client,
    ups_to_client,
    ups_from_gaia,
    ups_to_gaia,
  )
}
