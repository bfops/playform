#![feature(collections)]
#![feature(io)]
#![feature(std_misc)]

#[macro_use]
extern crate log;
extern crate nanomsg;

extern crate client;
extern crate server;

mod device_thread;
mod logger;

use device_thread::device_thread;
use logger::Logger;
use std::old_io::timer;
use std::thread::Thread;
use std::time::duration::Duration;

const CLIENT_TO_SERVER_URL: &'static str = "ipc:///tmp/client_to_server.ipc";
const SERVER_FROM_CLIENT_URL: &'static str = "ipc:///tmp/server_from_client.ipc";
const SERVER_TO_CLIENT_URL: &'static str = "ipc:///tmp/server_to_client.ipc";
const CLIENT_FROM_SERVER_URL: &'static str = "ipc:///tmp/client_from_server.ipc";

pub fn main() {
  log::set_logger(|max_log_level| {
    max_log_level.set(log::LogLevelFilter::Debug);
    Box::new(Logger)
  }).unwrap();

  debug!("starting");

  // Set up the loopback devices for local client-server communication.
  let _client_to_server_device_thread =
    Thread::spawn(||
      device_thread(CLIENT_TO_SERVER_URL, SERVER_FROM_CLIENT_URL),
    );
  let _server_to_client_device_thread =
    Thread::spawn(||
      device_thread(SERVER_TO_CLIENT_URL, CLIENT_FROM_SERVER_URL),
    );

  // TODO: Is this necessary? How long do devices need to start up?
  // Remove this.
  timer::sleep(Duration::milliseconds(1000));

  let _server_thread =
    Thread::spawn(||
      server::main(
        String::from_str(SERVER_FROM_CLIENT_URL),
        String::from_str(SERVER_TO_CLIENT_URL),
      )
    );

  client::main(
    String::from_str(CLIENT_FROM_SERVER_URL),
    String::from_str(CLIENT_TO_SERVER_URL),
  );

  debug!("finished");
}
