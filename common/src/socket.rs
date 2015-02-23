//! One-way socket wrapper data structures.

use nanomsg::{Endpoint, Socket, Protocol};
use rustc_serialize::{Encodable, Decodable, json};
use std::marker::PhantomData;
use std::sync::mpsc::{channel, Sender};
use std::thread;

/// A send-only strongly typed socket with sends running in a separate thread.
pub struct SendSocket<'a, T> {
  endpoint: Endpoint,
  messages: Sender<T>,
  _thread: thread::JoinGuard<'a, ()>,
}

impl<'a, T> SendSocket<'a, T>
  where T: Send + Encodable + 'static,
{
  /// Create a new `SendSocket` with a new thread to send its messages.
  pub fn spawn(url: &str) -> SendSocket<'a, T> {
    let (send, recv) = channel();

    let mut socket = Socket::new(Protocol::Push).unwrap();
    let endpoint = socket.connect(url).unwrap();

    let thread =
      thread::scoped(move || {
        // There's no real need to support a nice clean "kill" signal;
        // we're doing blocking IO, so it's a very real possibility that
        // we'll have to kill the thread anyway.
        // The nicest way to do it is probably to cause a panic, e.g. by
        // closing the send half of the channel.
        loop {
          match recv.recv() {
            Err(e) => panic!("Error receiving from channel: {:?}", e),
            Ok(msg) => {
              let msg = json::encode(&msg).unwrap();
              if let Err(e) = socket.write_all(msg.as_bytes()) {
                panic!("Error sending message: {:?}", e);
              }
            }
          };
        }
      });

    SendSocket {
      endpoint: endpoint,
      messages: send,
      _thread: thread,
    }
  }

  /// Send this socket a message.
  pub fn send(&self, msg: T) {
    self.messages.send(msg).unwrap();
  }

  /// Terminate this connection and wait for the thread to exit.
  pub fn close(mut self) {
    self.endpoint.shutdown().unwrap();
  }
}

unsafe impl<'a, T> Sync for SendSocket<'a, T> {}

#[unsafe_destructor]
impl<'a, T> Drop for SendSocket<'a, T> {
  fn drop(&mut self) {
    self.endpoint.shutdown().unwrap();
  }
}

/// A receive-only strongly typed socket with receives running in a separate thread.
pub struct ReceiveSocket<'a, T> {
  endpoint: Endpoint,
  _thread: thread::JoinGuard<'a, ()>,
  phantom: PhantomData<T>,
}

impl<'a, T> ReceiveSocket<'a, T>
  where T: Decodable,
{
  /// Create a new `ReceiveSocket` with a spawned thread polling the socket.
  pub fn spawn<F>(url: &str, mut act: F) -> ReceiveSocket<'a, T>
    where F: FnMut(T) + Send + 'a,
  {
    let mut socket = Socket::new(Protocol::Pull).unwrap();
    let endpoint = socket.bind(url.as_slice()).unwrap();

    let thread =
      thread::scoped(move || {
        loop {
          match socket.read_to_end() {
            Err(e) => panic!("Error reading from socket: {:?}", e),
            Ok(s) => {
              let s = String::from_utf8(s).unwrap();
              let msg = json::decode(s.as_slice()).unwrap();
              act(msg);
            },
          }
        }
      });
    
    ReceiveSocket {
      endpoint: endpoint,
      _thread: thread,
      phantom: PhantomData,
    }
  }

  /// Terminate this connection and wait for the thread to exit.
  pub fn close(mut self) {
    self.endpoint.shutdown().unwrap();
  }
}

unsafe impl<'a, T> Sync for ReceiveSocket<'a, T> {}

#[unsafe_destructor]
impl<'a, T> Drop for ReceiveSocket<'a, T> {
  fn drop(&mut self) {
    self.endpoint.shutdown().unwrap();
  }
}
