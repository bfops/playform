//! Server binary

#![deny(missing_docs)]
#![deny(warnings)]

#![feature(plugin)]
#![plugin(clippy)]
#![allow(mutex_atomic)]
#![allow(or_fun_call)]

#![feature(alloc_system)]
extern crate alloc_system;

extern crate env_logger;
extern crate nanomsg;
#[macro_use]
extern crate log;
extern crate thread_scoped;

extern crate server_lib;

use std::borrow::Borrow;
use std::env;
use std::sync::Mutex;

fn main() {
  env_logger::init().unwrap();

  let mut args = env::args();
  args.next().unwrap();
  let listen_url = args.next().unwrap_or_else(|| String::from("ipc:///tmp/server.ipc"));
  assert!(args.next().is_none());

  info!("Listening on {}.", listen_url);

  let quit_signal = Mutex::new(false);

  server_lib::run(listen_url.borrow(), &quit_signal);
}
