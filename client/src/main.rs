use env_logger;
use std::env;
use std::sync::mpsc::{channel, TryRecvError};
use std::sync::Mutex;
use std::thread;

use common::communicate::ClientToServer;
use common::socket::{SendSocket, ReceiveSocket};

use client::Client;
use server_recv_thread::server_recv_thread;
use server_send_thread::server_send_thread;
use surroundings_thread::surroundings_thread;
use terrain_load_thread::terrain_load_thread;
use view_thread::view_thread;

#[main]
fn main() {
  env_logger::init().unwrap();

  let mut args = env::args();
  args.next().unwrap();
  let listen_url = args.next().unwrap_or(String::from_str("ipc:///tmp/client.ipc"));
  let server_url = args.next().unwrap_or(String::from_str("ipc:///tmp/server.ipc"));
  assert!(args.next().is_none());

  info!("Sending to {}.", server_url);
  info!("Listening on {}.", listen_url);

  let (server_send_thread_send, mut server_send_thread_recv) = channel();
  let (surroundings_thread_send, mut surroundings_thread_recv) = channel();
  let (terrain_load_thread_send, mut terrain_load_thread_recv) = channel();
  let (view_thread_send, mut view_thread_recv) = channel();

  let server_send_thread_send = Mutex::new(server_send_thread_send);
  let surroundings_thread_send = Mutex::new(surroundings_thread_send);
  let terrain_load_thread_send = Mutex::new(terrain_load_thread_send);
  let view_thread_send = Mutex::new(view_thread_send);

  let mut listen_socket = ReceiveSocket::new(listen_url.as_slice());
  let mut talk_socket = SendSocket::new(server_url.as_slice());

  let client = Client::new();

  {
    let _server_recv_thread = {
      let client = &client;
      let listen_socket = &mut listen_socket;
      let view_thread_send = &view_thread_send;
      let surroundings_thread_send = &surroundings_thread_send;
      let terrain_load_thread_send = &terrain_load_thread_send;
      thread::scoped(move || {
        server_recv_thread(
          client,
          &mut || { listen_socket.read() },
          &mut |view_update| {
            view_thread_send.lock().unwrap().send(Some(view_update)).unwrap();
          },
          &mut |player_position| {
            surroundings_thread_send.lock().unwrap().send(Some(player_position)).unwrap();
          },
          &mut |block| {
            terrain_load_thread_send.lock().unwrap().send(Some(block)).unwrap();
          },
        );
      })
    };

    // TODO: This can get lost if the server is not started.
    // Maybe do this in a loop until we get a response?
    talk_socket.write(ClientToServer::Init(listen_url.clone()));

    let _server_send_thread = {
      let server_send_thread_recv = &mut server_send_thread_recv;
      let talk_socket = &mut talk_socket;
      thread::scoped(move || {
        server_send_thread(
          &mut move || { server_send_thread_recv.recv().unwrap() },
          &mut |msg| { talk_socket.write(msg) },
        )
      })
    };

    let _surroundings_thread = {
      let client = &client;
      let surroundings_thread_recv = &mut surroundings_thread_recv;
      let server_send_thread_send = &server_send_thread_send;
      let view_thread_send = &view_thread_send;
      thread::scoped(move || {
        surroundings_thread(
          client,
          &mut move || { surroundings_thread_recv.recv().unwrap() },
          &mut |view_update| {
            view_thread_send.lock().unwrap().send(Some(view_update)).unwrap();
          },
          &mut |server_update| {
            server_send_thread_send.lock().unwrap().send(Some(server_update)).unwrap();
          },
        )
      })
    };

    let _terrain_load_thread = {
      let client = &client;
      let terrain_load_thread_recv = &mut terrain_load_thread_recv;
      let view_thread_send = &view_thread_send;
      thread::scoped(move || {
        terrain_load_thread(
          &client,
          &mut move || { terrain_load_thread_recv.recv().unwrap() },
          &mut |view_update| {
            view_thread_send.lock().unwrap().send(Some(view_update)).unwrap();
          },
        )
      })
    };

    let view_thread = {
      let view_thread_recv = &mut view_thread_recv;
      let server_send_thread_send = &server_send_thread_send;
      thread::scoped(move || {
        view_thread(
          &mut || {
            match view_thread_recv.try_recv() {
              Ok(Some(msg)) => Some(msg),
              Ok(None) => {
                panic!(
                  "{} {}",
                  "The view thread initiates quits.",
                  "It should not receive a Quit signal."
                );
              },
              Err(TryRecvError::Empty) => None,
              Err(TryRecvError::Disconnected) =>
                panic!("view_thread_send should not be closed."),
            }
          },
          &mut |server_update| {
            server_send_thread_send.lock().unwrap().send(Some(server_update)).unwrap();
          },
        )
      })
    };

    view_thread.join();

    // System events go to the view_thread, so it handles quit signals.
    // Once it quits, we should close everything and die.

    server_send_thread_send.lock().unwrap().send(None).unwrap();
    surroundings_thread_send.lock().unwrap().send(None).unwrap();
    terrain_load_thread_send.lock().unwrap().send(None).unwrap();
    view_thread_send.lock().unwrap().send(None).unwrap();
  }

  drop(talk_socket);
  drop(listen_socket);
}
