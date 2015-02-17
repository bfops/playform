#![feature(std_misc)]

#[macro_use]
extern crate log;
extern crate nanomsg;

extern crate common;

use common::logger::Logger;
use nanomsg::{Socket, Protocol};
use std::thread::Thread;

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
  let client_to_server_device_thread =
    Thread::scoped(||
      device_thread(CLIENT_TO_SERVER_URL, SERVER_FROM_CLIENT_URL),
    );
  let server_to_client_device_thread =
    Thread::scoped(||
      device_thread(SERVER_TO_CLIENT_URL, CLIENT_FROM_SERVER_URL),
    );
  client_to_server_device_thread.join().unwrap_or_else(|_| panic!("join failed"));
  server_to_client_device_thread.join().unwrap_or_else(|_| panic!("join failed"));

  debug!("finished");
}

pub fn device_thread(client_device_url: &str, server_device_url: &str) {
  let mut send_socket = Socket::new_for_device(Protocol::Rep).unwrap();
  let mut send_endpoint = send_socket.bind(client_device_url).unwrap();
  let mut recv_socket = Socket::new_for_device(Protocol::Req).unwrap();
  let mut recv_endpoint = recv_socket.bind(server_device_url).unwrap();

  Socket::device(&send_socket, &recv_socket).unwrap();

  send_endpoint.shutdown().unwrap();
  recv_endpoint.shutdown().unwrap();
}
