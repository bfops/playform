//! This crate contains client-only components of Playform.

#![deny(missing_docs)]
#![deny(warnings)]

#![feature(core)]
#![feature(collections)]
#![feature(env)]
#![feature(io)]
#![feature(path)]
#![feature(slicing_syntax)]
#![feature(std_misc)]
#![feature(test)]
#![feature(unboxed_closures)]
#![feature(unsafe_destructor)]

extern crate common;
extern crate env_logger;
extern crate gl;
#[macro_use]
extern crate log;
extern crate libc;
extern crate nalgebra;
extern crate nanomsg;
extern crate "rustc-serialize" as rustc_serialize;
extern crate sdl2;
extern crate "sdl2-sys" as sdl2_sys;
extern crate test;
extern crate time;
extern crate yaglw;

mod camera;
mod client;
mod client_thread;
mod client_update;
mod fontloader;
mod hud;
mod light;
mod mob_buffers;
mod process_event;
mod render;
mod shaders;
mod terrain_buffers;
mod ttf;
mod view;
mod view_thread;
mod view_update;

use client_thread::client_thread;
use common::communicate::{spark_socket_sender, spark_socket_receiver};
use std::sync::mpsc::channel;
use std::sync::Future;
use view_thread::view_thread;

/// Entry point.
pub fn main() {
  env_logger::init().unwrap();

  debug!("starting");

  let mut args = std::env::args();
  args.next().unwrap();
  let listen_url = args.next().unwrap_or(String::from_str("ipc:///tmp/client.ipc"));
  let server_listen_url = args.next().unwrap_or(String::from_str("ipc:///tmp/server.ipc"));
  assert!(args.next().is_none());

  let (view_to_client_send, view_to_client_recv) = channel();
  let (client_to_view_send, client_to_view_recv) = channel();

  let (ups_from_server, mut listen_endpoint) = spark_socket_receiver(listen_url.clone());
  let (ups_to_server, mut talk_endpoint) = spark_socket_sender(server_listen_url);

  let client_thread =
    Future::spawn(move || {
      client_thread(
        listen_url,
        &ups_from_server,
        &ups_to_server,
        &view_to_client_recv,
        &client_to_view_send,
      );

      (ups_from_server, ups_to_server, view_to_client_recv, client_to_view_send)
    });

  view_thread(
    &client_to_view_recv,
    &view_to_client_send,
  );

  let (_ups_from_server, _ups_to_server, _view_to_client_recv, _client_to_view_send) =
    client_thread.into_inner();

  listen_endpoint.shutdown().unwrap();
  talk_endpoint.shutdown().unwrap();

  debug!("Done.");
}
