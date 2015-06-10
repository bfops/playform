use env_logger;
use std::env;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use common::communicate::{ClientToServer, ServerToClient};
use common::serialize;
use common::serialize::Copyable;
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
  let listen_url = args.next().unwrap_or(String::from("ipc:///tmp/client.ipc"));
  let server_url = args.next().unwrap_or(String::from("ipc:///tmp/server.ipc"));
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

  let quit = Mutex::new(false);
  let quit = &quit;

  let _server_recv_thread = {
    let listen_url = listen_url.clone();
    let server_recv_thread_send = server_recv_thread_send.clone();
    thread::spawn(move || {
      let mut listen_socket =
        ReceiveSocket::new(listen_url.clone().as_ref(), Some(Duration::from_secs(30)));
      loop {
        let msg = listen_socket.read();
        server_recv_thread_send.send(msg).unwrap();
      }
    })
  };

  let _server_send_thread = {
    thread::spawn(move || {
      let mut talk_socket =
        SendSocket::new(server_url.clone().as_ref(), Some(Duration::from_secs(30)));
      loop {
        let msg: Option<ClientToServer> = server_send_thread_recv.recv().unwrap();
        let msg = serialize::encode(&msg.unwrap()).unwrap();
        talk_socket.write(msg.as_ref());
      }
    })
  };

  // TODO: Consider using RPCs to solidify the request-response patterns.
  server_send_thread_send.send(Some(ClientToServer::Init(listen_url.clone()))).unwrap();
  let client;
  'init_loop:loop {
    match server_recv_thread_recv.recv().map(|s| serialize::decode(s.as_ref()).unwrap()) {
      Ok(ServerToClient::LeaseId(client_id)) => {
        server_send_thread_send.send(Some(ClientToServer::AddPlayer(client_id))).unwrap();
        let client_id = client_id.0;
        loop {
          match server_recv_thread_recv.recv().map(|s| serialize::decode(s.as_ref()).unwrap()) {
            Ok(ServerToClient::PlayerAdded(Copyable(player_id), Copyable(position))) => {
              client = Client::new(client_id, player_id, position);
              break 'init_loop;
            },
            Ok(msg) => {
              // Ignore other messages in the meantime.
              info!("Ignoring: {:?}", msg);
            },
            Err(e) => {
              panic!("Received error: {:?}", e);
            },
          }
        }
      },
      Ok(msg) => {
        // Ignore other messages in the meantime.
        info!("Ignoring: {:?}", msg);
      },
      Err(e) => {
        panic!("Received error: {:?}", e);
      },
    }
  }
  let client = &client;

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
              .map(|msg| serialize::decode(msg.as_ref()).unwrap())
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
          client.player_id,
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
