//! SDL input event processing code.

use cgmath::{Vector2, Vector3};
use sdl2;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mouse::Mouse;
use sdl2::video;
use std::f32::consts::PI;
use stopwatch;

use common::communicate::ClientToServer;
use common::entity_id;
use common::communicate::ClientToServer::*;

use view;

#[allow(missing_docs)]
pub fn process_event<UpdateServer>(
  sdl: &sdl2::Sdl,
  player_id: entity_id::T,
  update_server: &mut UpdateServer,
  view: &mut view::T,
  window: &mut video::Window,
  event: Event,
) where UpdateServer: FnMut(ClientToServer)
{
  match event {
    Event::KeyDown{keycode, repeat, ..} => {
      keycode.map(|keycode| {
        if !repeat {
          key_press(player_id, update_server, view, keycode);
        }
      });
    },
    Event::KeyUp{keycode, repeat, ..} => {
      keycode.map(|keycode| {
        if !repeat {
          key_release(player_id, update_server, keycode);
        }
      });
    },
    Event::MouseMotion{x, y, ..} => {
      mouse_move(sdl, player_id, update_server, view, window, x, y);
    },
    Event::MouseButtonDown{mouse_btn, ..} => {
      mouse_press(player_id, update_server, mouse_btn);
    },
    _ => {},
  }
}

fn key_press<UpdateServer>(
  player_id: entity_id::T,
  update_server: &mut UpdateServer,
  view: &mut view::T,
  key: Keycode,
) where UpdateServer: FnMut(ClientToServer)
{
  stopwatch::time("event.key_press", || {
    match key {
      Keycode::A => {
        update_server(Walk(player_id, Vector3::new(-1.0, 0.0, 0.0)));
      },
      Keycode::D => {
        update_server(Walk(player_id, Vector3::new(1.0, 0.0, 0.0)));
      },
      Keycode::Space => {
        update_server(StartJump(player_id));
      },
      Keycode::W => {
        update_server(Walk(player_id, Vector3::new(0.0, 0.0, -1.0)));
      },
      Keycode::S => {
        update_server(Walk(player_id, Vector3::new(0.0, 0.0, 1.0)));
      },
      Keycode::Left => {
        update_server(RotatePlayer(player_id, Vector2::new(PI / 12.0, 0.0)));
        view.camera.rotate_lateral(PI / 12.0);
      },
      Keycode::Right => {
        update_server(RotatePlayer(player_id, Vector2::new(-PI / 12.0, 0.0)));
        view.camera.rotate_lateral(-PI / 12.0);
      },
      Keycode::Up => {
        update_server(RotatePlayer(player_id, Vector2::new(0.0, PI / 12.0)));
        view.camera.rotate_vertical(PI / 12.0);
      },
      Keycode::Down => {
        update_server(RotatePlayer(player_id, Vector2::new(0.0, -PI / 12.0)));
        view.camera.rotate_vertical(-PI / 12.0);
      },
      Keycode::H => {
        view.show_hud = !view.show_hud;
      },
      _ => {},
    }
  })
}

fn mouse_press<UpdateServer>(
  player_id: entity_id::T,
  update_server: &mut UpdateServer,
  mouse_btn: Mouse,
) where UpdateServer: FnMut(ClientToServer)
{
  stopwatch::time("event.mouse_press", || {
    match mouse_btn {
      Mouse::Left => {
        update_server(
          ClientToServer::Add(player_id)
        );
      },
      Mouse::Right => {
        update_server(
          ClientToServer::Remove(player_id)
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
) where UpdateServer: FnMut(ClientToServer)
{
  stopwatch::time("event.key_release", || {
    match key {
      // accelerations are negated from those in key_press.
      Keycode::A => {
        update_server(Walk(player_id, Vector3::new(1.0, 0.0, 0.0)));
      },
      Keycode::D => {
        update_server(Walk(player_id, Vector3::new(-1.0, 0.0, 0.0)));
      },
      Keycode::Space => {
        update_server(StopJump(player_id));
      },
      Keycode::W => {
        update_server(Walk(player_id, Vector3::new(0.0, 0.0, 1.0)));
      },
      Keycode::S => {
        update_server(Walk(player_id, Vector3::new(0.0, 0.0, -1.0)));
      },
      _ => {}
    }
  })
}

fn mouse_move<UpdateServer>(
  sdl: &sdl2::Sdl,
  player_id: entity_id::T,
  update_server: &mut UpdateServer,
  view: &mut view::T,
  window: &mut video::Window,
  x: i32, y: i32,
) where UpdateServer: FnMut(ClientToServer)
{
  // x and y are measured from the top-left corner.

  stopwatch::time("event.mouse_move", || {
    let (w, h) = window.size();
    let (cx, cy) = (w as i32 / 2, h as i32 / 2);
    let d = Vector2::new(x - cx, cy - y);
    // To-radians coefficient. Numbers closer to zero dull the mouse movement more.
    let to_radians = Vector2::new(-1.0 / 1000.0, 1.0 / 1600.0);
    let r = Vector2::new(d.x as f32 * to_radians.x, d.y as f32 * to_radians.y);

    update_server(RotatePlayer(player_id, r));
    view.camera.rotate_lateral(r.x);
    view.camera.rotate_vertical(r.y);

    sdl.mouse().warp_mouse_in_window(window, cx, cy);
  })
}
