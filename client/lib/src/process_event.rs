//! SDL input event processing code.

use cgmath::{Vector2, Vector3};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mouse::Mouse;
use std::f32::consts::PI;
use stopwatch;

use common::entity_id;
use common::protocol;

use client;
use view;

#[allow(missing_docs)]
pub fn process_event<UpdateServer>(
  update_server: &mut UpdateServer,
  view: &mut view::T,
  client: &client::T,
  event: Event,
) where UpdateServer: FnMut(protocol::ClientToServer)
{
  match event {
    Event::KeyDown{keycode, repeat, ..} => {
      keycode.map(|keycode| {
        if !repeat {
          key_press(update_server, view, client, keycode);
        }
      });
    },
    Event::KeyUp{keycode, repeat, ..} => {
      keycode.map(|keycode| {
        if !repeat {
          key_release(client.player_id, update_server, keycode);
        }
      });
    },
    Event::MouseMotion{xrel, yrel, ..} => {
      mouse_move(client.player_id, update_server, view, xrel, yrel);
    },
    Event::MouseButtonDown{mouse_btn, ..} => {
      mouse_press(client.player_id, update_server, mouse_btn);
    },
    _ => {},
  }
}

fn key_press<UpdateServer>(
  update_server: &mut UpdateServer,
  view: &mut view::T,
  client: &client::T,
  key: Keycode,
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
      Keycode::A => {
        update_server(Walk(client.player_id, Vector3::new(-1.0, 0.0, 0.0)));
      },
      Keycode::D => {
        update_server(Walk(client.player_id, Vector3::new(1.0, 0.0, 0.0)));
      },
      Keycode::Space => {
        update_server(StartJump(client.player_id));
      },
      Keycode::W => {
        update_server(Walk(client.player_id, Vector3::new(0.0, 0.0, -1.0)));
      },
      Keycode::S => {
        update_server(Walk(client.player_id, Vector3::new(0.0, 0.0, 1.0)));
      },
      Keycode::Left => {
        lr(update_server, view, 1.0);
      },
      Keycode::Right => {
        lr(update_server, view, -1.0);
      },
      Keycode::Up => {
        ud(update_server, view, 1.0);
      },
      Keycode::Down => {
        ud(update_server, view, -1.0);
      },
      Keycode::H => {
        view.show_hud = !view.show_hud;
      },
      Keycode::M => {
        view.input_mode =
          match view.input_mode {
            view::InputMode::Camera => view::InputMode::Sun,
            view::InputMode::Sun => view::InputMode::Camera,
          };
      },
      Keycode::P => {
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
  player_id: entity_id::T,
  update_server: &mut UpdateServer,
  mouse_btn: Mouse,
) where UpdateServer: FnMut(protocol::ClientToServer)
{
  stopwatch::time("event.mouse_press", || {
    match mouse_btn {
      Mouse::Left => {
        update_server(
          protocol::ClientToServer::Add(player_id)
        );
      },
      Mouse::Right => {
        update_server(
          protocol::ClientToServer::Remove(player_id)
        );
      },
      _ => {},
    }
  })
}

fn key_release<UpdateServer>(
  player_id: entity_id::T,
  update_server: &mut UpdateServer,
  key: Keycode,
) where UpdateServer: FnMut(protocol::ClientToServer)
{
  stopwatch::time("event.key_release", || {
    match key {
      // accelerations are negated from those in key_press.
      Keycode::A => {
        update_server(protocol::ClientToServer::Walk(player_id, Vector3::new(1.0, 0.0, 0.0)));
      },
      Keycode::D => {
        update_server(protocol::ClientToServer::Walk(player_id, Vector3::new(-1.0, 0.0, 0.0)));
      },
      Keycode::Space => {
        update_server(protocol::ClientToServer::StopJump(player_id));
      },
      Keycode::W => {
        update_server(protocol::ClientToServer::Walk(player_id, Vector3::new(0.0, 0.0, 1.0)));
      },
      Keycode::S => {
        update_server(protocol::ClientToServer::Walk(player_id, Vector3::new(0.0, 0.0, -1.0)));
      },
      _ => {}
    }
  })
}

// x and y are relative to last position.
fn mouse_move<UpdateServer>(
  player_id: entity_id::T,
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
