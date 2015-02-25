//! One-way socket wrapper data structures.

use nanomsg::{Endpoint, Socket, Protocol};
use rustc_serialize::{Encodable, Decodable, json};
use std::marker::PhantomData;
use std::time::duration::Duration;

/// A send-only strongly typed socket with sends running in a separate thread.
pub struct SendSocket<T> {
  socket: Socket,
  endpoint: Endpoint,
  phantom: PhantomData<T>,
}

impl<T> SendSocket<T>
  where T: Encodable,
{
  /// Create a new `SendSocket` with a new thread to send its messages.
  pub fn new(url: &str) -> SendSocket<T> {
    let mut socket = Socket::new(Protocol::Push).unwrap();
    socket.set_send_timeout(&Duration::seconds(30)).unwrap();
    let endpoint = socket.connect(url).unwrap();

    SendSocket {
      socket: socket,
      endpoint: endpoint,
      phantom: PhantomData,
    }
  }

  /// Block until we can send this socket a message.
  pub fn write(&mut self, msg: T) {
    let msg = json::encode(&msg).unwrap();
    self.socket.write_all(msg.as_bytes()).unwrap();
  }

  /// Terminate this connection.
  pub fn close(self) {
    // The `drop` takes care of everything.
  }
}

#[unsafe_destructor]
impl<T> Drop for SendSocket<T> {
  fn drop(&mut self) {
    self.endpoint.shutdown().unwrap();
  }
}

/// A receive-only strongly typed socket with receives running in a separate thread.
pub struct ReceiveSocket<T> {
  socket: Socket,
  endpoint: Endpoint,
  phantom: PhantomData<T>,
}

impl<'a, T> ReceiveSocket<T>
  where T: Decodable,
{
  /// Create a new `ReceiveSocket` with a spawned thread polling the socket.
  pub fn new(url: &str) -> ReceiveSocket<T> {
    let mut socket = Socket::new(Protocol::Pull).unwrap();
    socket.set_receive_timeout(&Duration::seconds(30)).unwrap();
    let endpoint = socket.bind(url.as_slice()).unwrap();

    ReceiveSocket {
      socket: socket,
      endpoint: endpoint,
      phantom: PhantomData,
    }
  }

  /// Block until a message can be fetched from this socket.
  pub fn read(&mut self) -> T {
    let msg = self.socket.read_to_end().unwrap();
    let msg = String::from_utf8(msg).unwrap();
    let msg = json::decode(msg.as_slice()).unwrap();
    msg
  }

  /// Terminate this connection.
  pub fn close(self) {
    // The `drop` takes care of everything.
  }
}

#[unsafe_destructor]
impl<T> Drop for ReceiveSocket<T> {
  fn drop(&mut self) {
    self.endpoint.shutdown().unwrap();
  }
}
