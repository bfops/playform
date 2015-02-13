#![feature(std_misc)]

#[macro_use]
extern crate log;

extern crate client;
extern crate server;

mod logger;

use logger::Logger;
use std::sync::mpsc::channel;
use std::thread::Thread;

pub fn main() {
  log::set_logger(|max_log_level| {
    max_log_level.set(log::LogLevelFilter::Debug);
    Box::new(Logger)
  }).unwrap();

  debug!("starting");

  // Create all the thread communication channels.
  let (server_to_client_send, server_to_client_recv) = channel();
  let (client_to_server_send, client_to_server_recv) = channel();

  let _server_thread =
    Thread::spawn(||
      server::main(
        client_to_server_recv,
        server_to_client_send,
      )
    );

  client::main(
    server_to_client_recv,
    client_to_server_send,
  );

  debug!("finished");
}
