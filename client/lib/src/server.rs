//! A low-level interface to send and receive server-client protocol messages

use std;

use common::socket::{SendSocket, ReceiveSocket};

#[allow(missing_docs)]
pub mod send {
  use std;
  use std::sync::mpsc::Sender;

  use common::protocol;

  #[derive(Clone)]
  pub struct T {
    pub sender     : Sender<Vec<u8>>,
    pub bytes_sent : std::sync::Arc<std::sync::Mutex<u64>>,
  }

  pub fn new(sender: Sender<Vec<u8>>) -> T {
    T {
      sender     : sender,
      bytes_sent : std::sync::Arc::new(std::sync::Mutex::new(0)),
    }
  }

  impl T {
    pub fn tell(&self, msg: &protocol::ClientToServer) {
      use bincode::rustc_serialize::encode;
      use bincode::SizeLimit;
      let msg = encode(msg, SizeLimit::Infinite).unwrap();
      *self.bytes_sent.lock().unwrap() += msg.len() as u64;
      self.sender.send(msg).unwrap();
    }
  }
}

#[allow(missing_docs)]
pub mod recv {
  use bincode;
  use std;
  use std::sync::mpsc::Receiver;
  use std::sync::mpsc::TryRecvError;

  use common::protocol;

  #[derive(Clone)]
  pub struct T (pub std::sync::Arc<Receiver<Vec<u8>>>);

  impl T {
    pub fn try(&self) -> Option<protocol::ServerToClient> {
      match self.0.try_recv() {
        Ok(msg) => Some(bincode::rustc_serialize::decode(&msg).unwrap()),
        Err(TryRecvError::Empty) => None,
        e => {
          e.unwrap();
          unreachable!();
        },
      }
    }

    pub fn wait(&self) -> protocol::ServerToClient {
      let msg = self.0.recv().unwrap();
      bincode::rustc_serialize::decode(msg.as_ref()).unwrap()
    }
  }
}

#[allow(missing_docs)]
#[derive(Clone)]
pub struct T {
  pub talk   : send::T,
  pub listen : recv::T,
}

unsafe impl Send for T {}

#[allow(missing_docs)]
pub fn new(
  server_url: &str,
  listen_url: &str,
) -> T {
  let (send_send, send_recv) = std::sync::mpsc::channel();
  let (recv_send, recv_recv) = std::sync::mpsc::channel();

  let _recv_thread ={
    let listen_url = listen_url.to_owned();
    let recv_send = recv_send.clone();
    std::thread::spawn(move || {
      let mut listen_socket =
        ReceiveSocket::new(
          listen_url.clone().as_ref(),
          Some(std::time::Duration::from_secs(30)),
        );
      loop {
        match listen_socket.read() {
          None => break,
          Some(msg) => {
            recv_send.send(msg).unwrap()
          },
        }
      }
    })
  };

  let _send_thread = {
    let server_url = server_url.to_owned();
    std::thread::spawn(move || {
      let mut talk_socket =
        SendSocket::new(
          server_url.as_ref(),
          Some(std::time::Duration::from_secs(30)),
        );
      loop {
        match send_recv.recv() {
          Err(_) => break,
          Ok(msg) => {
            let msg: Vec<u8> = msg;
            talk_socket.write(msg.as_ref()).unwrap();
          },
        }
      }
    })
  };

  T {
    talk: send::new(send_send),
    listen: recv::T (std::sync::Arc::new(recv_recv)),
  }
}
