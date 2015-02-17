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
use nanomsg::{Socket, Protocol};
use server::Server;
use std::sync::mpsc::channel;
use std::thread::Thread;

#[allow(missing_docs)]
pub fn main(
  from_client_url: String,
) {
  let mut ups_from_client = Socket::new(Protocol::Rep).unwrap();

  let mut endpoints = Vec::new();
  endpoints.push(ups_from_client.connect(from_client_url.as_slice()).unwrap());

  let ups_from_client = spark_socket_receiver(ups_from_client);

  let timers = TimerSet::new();
  let world = Server::new(&timers);

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

  server_thread::server_thread(
    timers,
    world,
    &mut endpoints,
    ups_from_client,
    ups_from_gaia_recv,
    ups_to_gaia_send,
  );

  for mut endpoint in endpoints.into_iter() {
    endpoint.shutdown().unwrap();
  }
}
