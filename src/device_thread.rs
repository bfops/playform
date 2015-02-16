use nanomsg::{Socket, Protocol};

pub fn device_thread(client_device_url: &str, server_device_url: &str) {
  let mut send_socket = Socket::new_for_device(Protocol::Rep).unwrap();
  let mut send_endpoint = send_socket.bind(client_device_url).unwrap();
  let mut recv_socket = Socket::new_for_device(Protocol::Req).unwrap();
  let mut recv_endpoint = recv_socket.bind(server_device_url).unwrap();

  Socket::device(&send_socket, &recv_socket).unwrap();

  send_endpoint.shutdown().unwrap();
  recv_endpoint.shutdown().unwrap();
}
