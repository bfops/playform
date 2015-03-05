use env_logger;
use rustc_serialize::json;
use std::env;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Mutex;
use std::thread;

use common::communicate::ClientToServer;
use common::socket::{SendSocket, ReceiveSocket};

use client::Client;
use update_thread::update_thread;
use view_thread::view_thread;

// TODO: This is duplicated in the server. Fix that.
#[inline(always)]
fn try_recv<T>(recv: &Receiver<T>) -> Option<T>
  where T: Send,
{
  match recv.try_recv() {
    Ok(msg) => Some(msg),
    Err(TryRecvError::Empty) => None,
    e => Some(e.unwrap()),
  }
}

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

  let (server_send_thread_send, server_send_thread_recv) = channel();
  let (server_recv_thread_send, mut server_recv_thread_recv) = channel();
  let (terrain_blocks_send, mut terrain_blocks_recv) = channel();
  let (view_thread_send, mut view_thread_recv) = channel();

  let server_recv_thread_send = &server_recv_thread_send;
  let server_recv_thread_recv = &mut server_recv_thread_recv;
  let terrain_blocks_send = &terrain_blocks_send;
  let terrain_blocks_recv = &mut terrain_blocks_recv;
  let view_thread_send = &view_thread_send;
  let view_thread_recv = &mut view_thread_recv;

  let client = Client::new();
  let client = &client;

  let quit = Mutex::new(false);
  let quit = &quit;

  let _server_recv_thread = {
    let listen_url = listen_url.clone();
    let server_recv_thread_send = server_recv_thread_send.clone();
    thread::spawn(move || {
      let mut listen_socket = ReceiveSocket::new(listen_url.as_slice());
      loop {
        let msg = listen_socket.read();
        server_recv_thread_send.send(msg).unwrap();
      }
    })
  };

  let _server_send_thread = {
    thread::spawn(move || {
      let mut talk_socket = SendSocket::new(server_url.as_slice());
      loop {
        let msg = server_send_thread_recv.recv().unwrap();
        let msg = json::encode(&msg).unwrap();
        talk_socket.write(msg.as_bytes());
      }
    })
  };

  // TODO: This can get lost if the server is not started.
  // Maybe do this in a loop until we get a response?
  server_send_thread_send.send(Some(ClientToServer::Init(listen_url))).unwrap();

  {
    let _update_thread = {
      let client = &client;
      let view_thread_send = view_thread_send.clone();
      let server_send_thread_send = server_send_thread_send.clone();
      let terrain_blocks_send = terrain_blocks_send.clone();
      thread::scoped(move || {
        update_thread(
          quit,
          client,
          &mut || {
            try_recv(server_recv_thread_recv)
              .map(|msg| json::decode(&msg).unwrap())
          },
          &mut || { try_recv(terrain_blocks_recv) },
          &mut |up| { view_thread_send.send(up).unwrap() },
	        &mut |up| { server_send_thread_send.send(Some(up)).unwrap() },
          &mut |block| { terrain_blocks_send.send(block).unwrap() },
        )
      })
    };

    let view_thread = {
      let server_send_thread_send = server_send_thread_send.clone();
      thread::scoped(move || {
        view_thread(
          &mut || {
            match view_thread_recv.try_recv() {
              Ok(msg) => Some(msg),
              Err(TryRecvError::Empty) => None,
              Err(TryRecvError::Disconnected) =>
                panic!("view_thread_send should not be closed."),
            }
          },
          &mut |server_update| {
            server_send_thread_send.send(Some(server_update)).unwrap();
          },
        )
      })
    };

    view_thread.join();

    // View thread returned, so we got a quit event.
    *quit.lock().unwrap() = true;
  }
}

#[test]
fn keep_bin_code_live() {
  let i = 0;
  if i == 1 {
    main();
  }
}
