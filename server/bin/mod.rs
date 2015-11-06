//! Server binary

#![deny(missing_docs)]
#![deny(warnings)]

#![feature(plugin)]
#![plugin(clippy)]

extern crate env_logger;
#[macro_use]
extern crate log;

extern crate server_lib;

use std::borrow::Borrow;
use std::env;

fn main() {
  env_logger::init().unwrap();

  let mut args = env::args();
  args.next().unwrap();
  let listen_url
    = args.next().unwrap_or(String::from("ipc:///tmp/server.ipc"));
  assert!(args.next().is_none());

  info!("Listening on {}.", listen_url);

  server_lib::run(listen_url.borrow());
}
