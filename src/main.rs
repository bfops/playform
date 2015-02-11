use client_thread::client_thread;
use id_allocator::IdAllocator;
use log;
use logger::Logger;
use server_thread::server_thread;
use std::sync::mpsc::channel;
use std::thread::Thread;
use view_thread::view_thread;

pub fn main() {
  log::set_logger(|max_log_level| {
    max_log_level.set(log::LogLevelFilter::Debug);
    Box::new(Logger)
  }).unwrap();

  debug!("starting");

  // Create all the thread communication channels.
  let (server_to_client_send, server_to_client_recv) = channel();
  let (client_to_server_send, client_to_server_recv) = channel();
  let (view_to_client_send, view_to_client_recv) = channel();
  let (client_to_view_send, client_to_view_recv) = channel();

  let mut owner_alloc = IdAllocator::new();

  let client_id = owner_alloc.allocate();

  let _server_thread =
    Thread::spawn(move ||
      server_thread(
        &mut owner_alloc,
        &client_to_server_recv,
        &server_to_client_send,
      ));

  let _client_thread =
    Thread::spawn(move ||
      client_thread(
        client_id,
        &server_to_client_recv,
        &client_to_server_send,
        &view_to_client_recv,
        &client_to_view_send,
      )
    );

  view_thread(
    &client_to_view_recv,
    &view_to_client_send,
  );

  debug!("finished");
}
