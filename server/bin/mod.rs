//! Server binary

#![deny(missing_docs)]
#![deny(warnings)]

#![feature(plugin)]
#![plugin(clippy)]
#![allow(mutex_atomic)]
#![allow(or_fun_call)]

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

  let _quit_thread =
    unsafe {
      let quit_signal = &quit_signal;
      thread_scoped::scoped(move || {
        wait_for_quit();
        *quit_signal.lock().unwrap() = true;
        // Close all sockets.
        nanomsg::Socket::terminate();
      })
    };

  server_lib::run(listen_url.borrow(), &quit_signal);
}

fn wait_for_quit() {
  loop {
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).unwrap();

    if line == "quit\n" {
      println!("Quitting");
      return
    } else {
      println!("Unrecognized command: {:?}", line);
    }
  }
}
