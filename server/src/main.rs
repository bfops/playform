use env_logger;
use rustc_serialize::json;
use std::env;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::thread;

use common::socket::ReceiveSocket;

use server::Server;
use update_thread::update_thread;

// TODO: This is duplicated in the client. Fix that.
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
  let listen_url
    = args.next().unwrap_or(String::from_str("ipc:///tmp/server.ipc"));
  assert!(args.next().is_none());

  info!("Listening on {}.", listen_url);

  let (listen_thread_send, mut listen_thread_recv) = channel();
  let (gaia_thread_send, mut gaia_thread_recv) = channel();

  let _listen_thread = {
    let listen_thread_send = listen_thread_send.clone();
    thread::scoped(move || {
      let mut socket = ReceiveSocket::new(listen_url.as_slice());
      loop {
        let msg = socket.read();
        listen_thread_send.send(msg).unwrap();
      }
    })
  };

  let server = Server::new();

  let _update_thread = {
    let server = &server;
    let listen_thread_recv = &mut listen_thread_recv;
    let gaia_thread_recv = &mut gaia_thread_recv;
    let gaia_thread_send = gaia_thread_send.clone();
    thread::scoped(move || {
      update_thread(
        server,
        &mut || {
          try_recv(listen_thread_recv)
            .map(|msg| json::decode(&msg).unwrap())
        },
        &mut || { try_recv(gaia_thread_recv) },
        &mut |up| { gaia_thread_send.send(up).unwrap() },
      );
    })
  };
}
