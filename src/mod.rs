//! Standalone Playform binary to run the server and client.

#![deny(missing_docs)]
#![deny(warnings)]
#![feature(stmt_expr_attributes)]
#![feature(global_allocator)]
#![feature(allocator_api)]

extern crate env_logger;
extern crate nanomsg;
extern crate log;
extern crate thread_scoped;

extern crate client_lib;
#[cfg(feature = "dummy-client")]
extern crate dummy_client_lib;
extern crate server_lib;

use std::borrow::Borrow;
use std::sync::Mutex;

#[global_allocator]
static ALLOCATOR: std::heap::System = std::heap::System;

fn main() {
  env_logger::init().unwrap();

  let listen_url = String::from("ipc:///tmp/client.ipc");
  let server_url = String::from("ipc:///tmp/server.ipc");

  let quit_signal = Mutex::new(false);

  unsafe {
    let server_thread =
      thread_scoped::scoped(|| {
        server_lib::run(server_url.borrow(), &quit_signal);
      });

    #[cfg(feature = "dummy-client")]
    dummy_client_lib::run(listen_url.borrow(), server_url.borrow());
    #[cfg(not(feature = "dummy-client"))]
    client_lib::run(listen_url.borrow(), server_url.borrow());
    *quit_signal.lock().unwrap() = true;
    server_thread.join();

    nanomsg::Socket::terminate();
  }
}
