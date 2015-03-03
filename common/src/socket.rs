//! One-way socket wrapper data structures.

use nanomsg::{Endpoint, Socket, Protocol};
use std::io::{Read, Write};
use std::time::duration::Duration;

/// A send-only socket.
pub struct SendSocket {
  socket: Socket,
  endpoint: Endpoint,
}

impl SendSocket {
  #[allow(missing_docs)]
  pub fn new(url: &str) -> SendSocket {
    let mut socket = Socket::new(Protocol::Push).unwrap();
    socket.set_send_timeout(&Duration::seconds(30)).unwrap();
    let endpoint = socket.connect(url).unwrap();

    SendSocket {
      socket: socket,
      endpoint: endpoint,
    }
  }

  /// Block until we can send this socket a message.
  pub fn write(&mut self, msg: &[u8]) {
    self.socket.write(msg).unwrap();
  }

  /// Terminate this connection.
  pub fn close(self) {
    // The `drop` takes care of everything.
  }
}

#[unsafe_destructor]
impl Drop for SendSocket {
  fn drop(&mut self) {
    self.endpoint.shutdown().unwrap();
  }
}

/// A receive-only socket.
pub struct ReceiveSocket {
  socket: Socket,
  endpoint: Endpoint,
}

impl ReceiveSocket {
  #[allow(missing_docs)]
  pub fn new(url: &str) -> ReceiveSocket {
    let mut socket = Socket::new(Protocol::Pull).unwrap();
    socket.set_receive_timeout(&Duration::seconds(30)).unwrap();
    let endpoint = socket.bind(url.as_slice()).unwrap();

    ReceiveSocket {
      socket: socket,
      endpoint: endpoint,
    }
  }

  /// Block until a message can be fetched from this socket.
  pub fn read(&mut self) -> String {
    let mut msg = String::new();
    self.socket.read_to_string(&mut msg).unwrap();
    msg
  }

  /// Terminate this connection.
  pub fn close(self) {
    // The `drop` takes care of everything.
  }
}

#[unsafe_destructor]
impl Drop for ReceiveSocket {
  fn drop(&mut self) {
    self.endpoint.shutdown().unwrap();
  }
}
