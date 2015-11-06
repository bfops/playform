//! Standalone Playform binary to run the server and client.

#![deny(missing_docs)]
#![deny(warnings)]

#![feature(plugin)]
#![plugin(clippy)]
#![allow(mutex_atomic)]

extern crate env_logger;
extern crate nanomsg;
#[macro_use]
extern crate log;
extern crate thread_scoped;

extern crate client_lib;
extern crate server_lib;

use std::borrow::Borrow;
use std::sync::Mutex;

fn main() {
  env_logger::init().unwrap();

  let listen_url = String::from("ipc:///tmp/client.ipc");
  let server_url = String::from("ipc:///tmp/server.ipc");

  let quit_signal = Mutex::new(false);

  unsafe {
    let _server_thread =
      thread_scoped::scoped(|| {
        server_lib::run(server_url.borrow(), &quit_signal);
      });

    client_lib::run(listen_url.borrow(), server_url.borrow());

    *quit_signal.lock().unwrap() = true;
    // Close all sockets.
    nanomsg::Socket::terminate();
  }
}
