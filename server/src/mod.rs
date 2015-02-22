//! This crate contains server-only components of Playform.

#![deny(missing_docs)]
#![deny(warnings)]

#![feature(core)]
#![feature(collections)]
#![feature(env)]
#![feature(old_io)]
#![feature(std_misc)]
#![feature(test)]
#![feature(unboxed_closures)]
#![feature(unsafe_destructor)]

extern crate cgmath;
extern crate common;
extern crate env_logger;
#[macro_use]
extern crate log;
extern crate nanomsg;
extern crate noise;
extern crate opencl;
extern crate rand;
extern crate "rustc-serialize" as rustc_serialize;
extern crate test;
extern crate time;

mod client_thread;
mod gaia_thread;
mod in_progress_terrain;
mod init_mobs;
mod mob;
mod octree;
mod opencl_context;
mod physics;
mod player;
mod server;
mod sun;
mod terrain;
mod update_thread;

use common::communicate::spark_socket_receiver;
use common::stopwatch::TimerSet;
use gaia_thread::gaia_thread;
use server::Server;
use std::sync::mpsc::channel;
use std::sync::{Arc, Future, Mutex};

#[allow(missing_docs)]
pub fn main() {
  env_logger::init().unwrap();

  debug!("starting");

  let mut args = std::env::args();
  args.next().unwrap();
  let listen_url = args.next().unwrap_or(String::from_str("ipc:///tmp/server.ipc"));
  assert!(args.next().is_none());

  info!("Listening on {}.", listen_url);

  let (incoming, mut listen_endpoint) = spark_socket_receiver(listen_url);

  let server = Server::new();
  let server = Arc::new(server);

  let (ups_to_gaia_send, ups_to_gaia_recv) = channel();

  let ups_to_gaia_send = Arc::new(Mutex::new(ups_to_gaia_send));

  let gaia_thread = {
    let server = server.clone();
    Future::spawn(move || {
      gaia_thread(
        &ups_to_gaia_recv,
        &server,
      );

      ups_to_gaia_recv
    })
  };

  let _update_thread = {
    let server = server.clone();
    let ups_to_gaia_send = ups_to_gaia_send.clone();
    Future::spawn(move || {
      update_thread::update_thread(
        &TimerSet::new(),
        &server,
        &ups_to_gaia_send,
      );
    })
  };

  let mut client_endpoints = Vec::new();

  client_thread::client_thread(
    &mut client_endpoints,
    &server,
    &incoming,
    &ups_to_gaia_send,
  );

  let _ups_to_gaia_recv = gaia_thread.into_inner();

  listen_endpoint.shutdown().unwrap();
  for mut endpoint in client_endpoints.into_iter() {
    endpoint.shutdown().unwrap();
  }

  debug!("finished");
}
