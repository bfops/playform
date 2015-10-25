use std;

use common::socket::{SendSocket, ReceiveSocket};

pub mod send {
  use std::sync::mpsc::Sender;

  use common::communicate::ClientToServer;

  #[derive(Clone)]
  pub struct T (pub Sender<Vec<u8>>);

  impl T {
    pub fn tell(&self, msg: &ClientToServer) {
      use bincode::rustc_serialize::encode;
      use bincode::SizeLimit;
      let msg = encode(msg, SizeLimit::Infinite).unwrap();
      self.0.send(msg).unwrap();
    }
  }
}

pub mod recv {
  use bincode;
  use std;
  use std::sync::mpsc::Receiver;
  use std::sync::mpsc::TryRecvError;

  use common::communicate::ServerToClient;

  #[derive(Clone)]
  pub struct T (pub std::sync::Arc<Receiver<Vec<u8>>>);

  impl T {
    pub fn try(&self) -> Option<ServerToClient> {
      match self.0.try_recv() {
        Ok(msg) => Some(bincode::rustc_serialize::decode(&msg).unwrap()),
        Err(TryRecvError::Empty) => None,
        e => {
          e.unwrap();
          unreachable!();
        },
      }
    }

    pub fn wait(&self) -> ServerToClient {
      let msg = self.0.recv().unwrap();
      bincode::rustc_serialize::decode(msg.as_ref()).unwrap()
    }
  }
}

#[derive(Clone)]
pub struct T {
  pub talk: send::T,
  pub listen: recv::T,
}

unsafe impl Send for T {}

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
        let msg = listen_socket.read();
        recv_send.send(msg).unwrap();
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
        let msg: Vec<u8> = send_recv.recv().unwrap();
        talk_socket.write(msg.as_ref()).unwrap();
      }
    })
  };

  T {
    talk: send::T (send_send),
    listen: recv::T (std::sync::Arc::new(recv_recv)),
  }
}
