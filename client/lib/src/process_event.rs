//! SDL input event processing code.

use cgmath::{Vector2, Vector3};
use glutin::{Event, DeviceEvent, WindowEvent, ElementState, VirtualKeyCode, MouseButton};
use std::f32::consts::PI;
use stopwatch;

use common::entity;
use common::protocol;

use client;
use view;

#[allow(missing_docs)]
pub fn process_event<UpdateServer>(
  update_server: &mut UpdateServer,
  view: &mut view::T,
  client: &client::T,
  event: Event,
  last_key: &mut Option<VirtualKeyCode>,
) where UpdateServer: FnMut(protocol::ClientToServer)
{
  match event {
    Event::WindowEvent { event: WindowEvent::CursorMoved { .. }, .. } => {}
    Event::WindowEvent { event: WindowEvent::MouseWheel { .. }, .. } => {}
    Event::WindowEvent { event: WindowEvent::AxisMotion { .. }, .. } => {}
    Event::DeviceEvent { event: DeviceEvent::Motion { .. }, .. } => {}
    Event::DeviceEvent { event: DeviceEvent::MouseMotion { .. }, .. } => {}
    _ => warn!("event: {:?}", event),
  }

  match event {
    Event::WindowEvent { event, .. } => {
      match event {
        WindowEvent::Focused(false) => {
          if let Some(last_key) = last_key.take() {
            key_release(client.player_id, update_server, last_key);
          }
        }
        WindowEvent::KeyboardInput { input, .. } => {
          if let Some(virt_keycode) = input.virtual_keycode {
            match input.state {
              ElementState::Pressed => {
                if last_key == &None {
                  key_press(update_server, view, client, virt_keycode);
                  *last_key = Some(virt_keycode);
                } else if last_key.is_some() && last_key != &Some(virt_keycode) {
                  key_release(client.player_id, update_server, last_key.unwrap());
                  key_press(update_server, view, client, virt_keycode);
                  *last_key = Some(virt_keycode);
                }
              },
              ElementState::Released => {
                key_release(client.player_id, update_server, virt_keycode);
                *last_key = None;
              }
            }
          }
        }
        WindowEvent::MouseInput { state, button, .. } => {
          match state {
            ElementState::Pressed => mouse_press(client.player_id, update_server, button),
            ElementState::Released => {}
          }
        }
        _ => {},
      }
    }
    Event::DeviceEvent { event, .. } => {
      match event {
        DeviceEvent::MouseMotion { delta: (xrel, yrel) } =>
          mouse_move(client.player_id, update_server, view, xrel as i32, yrel as i32),
        _ => {}
      }
    }
    _ => {}
  }
}

fn key_press<UpdateServer>(
  update_server: &mut UpdateServer,
  view: &mut view::T,
  client: &client::T,
  key: VirtualKeyCode,
) where UpdateServer: FnMut(protocol::ClientToServer)
{
  use common::protocol::ClientToServer::*;

  let lr = |update_server: &mut UpdateServer, view: &mut view::T, k| {
    match view.input_mode {
      view::InputMode::Camera => {
        let angle = k * PI / 12.0;
        update_server(RotatePlayer(client.player_id, Vector2::new(angle, 0.0)));
        view.camera.rotate_lateral(angle);
      },
      view::InputMode::Sun => {
        view.sun.rotation += k * PI / 512.0;
      },
    }
  };

  let ud = |update_server: &mut UpdateServer, view: &mut view::T, k| {
    match view.input_mode {
      view::InputMode::Camera => {
        let angle = k * PI / 12.0;
        update_server(RotatePlayer(client.player_id, Vector2::new(0.0, angle)));
        view.camera.rotate_vertical(angle);
      },
      view::InputMode::Sun => {
        view.sun.progression += k * PI / 512.0;
      },
    }
  };

  stopwatch::time("event.key_press", || {
    match key {
      VirtualKeyCode::A => {
        update_server(Walk(client.player_id, Vector3::new(-1.0, 0.0, 0.0)));
      },
      VirtualKeyCode::D => {
        update_server(Walk(client.player_id, Vector3::new(1.0, 0.0, 0.0)));
      },
      VirtualKeyCode::Space => {
        update_server(StartJump(client.player_id));
      },
      VirtualKeyCode::W => {
        update_server(Walk(client.player_id, Vector3::new(0.0, 0.0, -1.0)));
      },
      VirtualKeyCode::S => {
        update_server(Walk(client.player_id, Vector3::new(0.0, 0.0, 1.0)));
      },
      VirtualKeyCode::Left => {
        lr(update_server, view, 1.0);
      },
      VirtualKeyCode::Right => {
        lr(update_server, view, -1.0);
      },
      VirtualKeyCode::Up => {
        ud(update_server, view, 1.0);
      },
      VirtualKeyCode::Down => {
        ud(update_server, view, -1.0);
      },
      VirtualKeyCode::H => {
        view.show_hud = !view.show_hud;
      },
      VirtualKeyCode::M => {
        view.input_mode =
          match view.input_mode {
            view::InputMode::Camera => view::InputMode::Sun,
            view::InputMode::Sun => view::InputMode::Camera,
          };
      },
      VirtualKeyCode::P => {
        let mut load_position = client.load_position.lock().unwrap();
        match *load_position {
          None => *load_position = Some(*client.player_position.lock().unwrap()),
          Some(_) => *load_position = None,
        }
      },
      _ => {},
    }
  })
}

fn mouse_press<UpdateServer>(
  player_id: entity::id::Player,
  update_server: &mut UpdateServer,
  mouse_btn: MouseButton,
) where UpdateServer: FnMut(protocol::ClientToServer)
{
  stopwatch::time("event.mouse_press", || {
    match mouse_btn {
      MouseButton::Left => {
        update_server(
          protocol::ClientToServer::Add(player_id)
        );
      },
      MouseButton::Right => {
        update_server(
          protocol::ClientToServer::Remove(player_id)
        );
      },
      _ => {},
    }
  })
}

fn key_release<UpdateServer>(
  player_id: entity::id::Player,
  update_server: &mut UpdateServer,
  key: VirtualKeyCode,
) where UpdateServer: FnMut(protocol::ClientToServer)
{
  stopwatch::time("event.key_release", || {
    match key {
      // accelerations are negated from those in key_press.
      VirtualKeyCode::A => {
        update_server(protocol::ClientToServer::Walk(player_id, Vector3::new(1.0, 0.0, 0.0)));
      },
      VirtualKeyCode::D => {
        update_server(protocol::ClientToServer::Walk(player_id, Vector3::new(-1.0, 0.0, 0.0)));
      },
      VirtualKeyCode::Space => {
        update_server(protocol::ClientToServer::StopJump(player_id));
      },
      VirtualKeyCode::W => {
        update_server(protocol::ClientToServer::Walk(player_id, Vector3::new(0.0, 0.0, 1.0)));
      },
      VirtualKeyCode::S => {
        update_server(protocol::ClientToServer::Walk(player_id, Vector3::new(0.0, 0.0, -1.0)));
      },
      _ => {}
    }
  })
}

// x and y are relative to last position.
fn mouse_move<UpdateServer>(
  player_id: entity::id::Player,
  update_server: &mut UpdateServer,
  view: &mut view::T,
  dx: i32, dy: i32,
) where UpdateServer: FnMut(protocol::ClientToServer)
{
  stopwatch::time("event.mouse_move", || {
    let d = Vector2::new(dx, dy);
    // To-radians coefficient. Numbers closer to zero dull the mouse movement more.
    let to_radians = Vector2::new(-1.0 / 1000.0, -1.0 / 1600.0);
    let r = Vector2::new(d.x as f32 * to_radians.x, d.y as f32 * to_radians.y);

    update_server(protocol::ClientToServer::RotatePlayer(player_id, r));
    view.camera.rotate_lateral(r.x);
    view.camera.rotate_vertical(r.y);
  })
}
