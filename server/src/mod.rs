//! This crate contains server-only components of Playform.

#![deny(missing_docs)]
#![deny(warnings)]

#![feature(core)]
#![feature(collections)]
#![feature(env)]
#![feature(hash)]
#![feature(io)]
#![feature(slicing_syntax)]
#![feature(std_misc)]
#![feature(test)]
#![feature(unboxed_closures)]
#![feature(unsafe_destructor)]

extern crate common;
extern crate env_logger;
#[macro_use]
extern crate log;
extern crate nalgebra;
extern crate nanomsg;
extern crate ncollide_entities;
extern crate ncollide_queries;
extern crate noise;
extern crate opencl;
extern crate rand;
extern crate "rustc-serialize" as rustc_serialize;
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

use common::communicate::spark_socket_receiver;
use common::stopwatch::TimerSet;
use gaia_thread::gaia_thread;
use server::Server;
use std::sync::mpsc::channel;
use std::sync::Future;

#[allow(missing_docs)]
pub fn main() {
  env_logger::init().unwrap();

  debug!("starting");

  let mut args = std::env::args();
  args.next().unwrap();
  let listen_url = args.next().unwrap_or(String::from_str("ipc:///tmp/server.ipc"));
  assert!(args.next().is_none());

  let (incoming, mut listen_endpoint) = spark_socket_receiver(listen_url);

  let timers = TimerSet::new();
  let world = Server::new(&timers);

  let (ups_to_gaia_send, ups_to_gaia_recv) = channel();
  let (ups_from_gaia_send, ups_from_gaia_recv) = channel();

  let gaia_thread = {
    let id_allocator = world.id_allocator.clone();
    let terrain = world.terrain_game_loader.terrain.clone();
    Future::spawn(move || {
      gaia_thread(
        id_allocator,
        &ups_to_gaia_recv,
        &ups_from_gaia_send,
        terrain,
      );

      (ups_to_gaia_recv, ups_from_gaia_send)
    })
  };

  let mut client_endpoints = Vec::new();

  server_thread::server_thread(
    &timers,
    world,
    &mut client_endpoints,
    &incoming,
    &ups_from_gaia_recv,
    &ups_to_gaia_send,
  );

  let (_ups_to_gaia_recv, _ups_from_gaia_send) = gaia_thread.into_inner();

  listen_endpoint.shutdown().unwrap();
  for mut endpoint in client_endpoints.into_iter() {
    endpoint.shutdown().unwrap();
  }

  timers.print();

  debug!("finished");
}
