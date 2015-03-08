//! SDL input event processing code.

use cgmath::{Vector2, Vector3};
use sdl2::event::Event;
use sdl2::keycode::KeyCode;
use sdl2::mouse;
use sdl2::video;
use std::f32::consts::PI;

use common::communicate::ClientToServer;
use common::entity::EntityId;
use common::communicate::ClientToServer::*;
use common::stopwatch::TimerSet;

use view::View;

#[allow(missing_docs)]
pub fn process_event<UpdateServer>(
  timers: &TimerSet,
  player_id: EntityId,
  update_server: &mut UpdateServer,
  view: &mut View,
  game_window: &mut video::Window,
  event: Event,
) where UpdateServer: FnMut(ClientToServer)
{
  match event {
    Event::KeyDown{keycode, repeat, ..} => {
      if !repeat {
        key_press(timers, player_id, update_server, view, keycode);
      }
    },
    Event::KeyUp{keycode, repeat, ..} => {
      if !repeat {
        key_release(timers, player_id, update_server, keycode);
      }
    },
    Event::MouseMotion{x, y, ..} => {
      mouse_move(timers, player_id, update_server, view, game_window, x, y);
    },
    _ => {},
  }
}

fn key_press<UpdateServer>(
  timers: &TimerSet,
  player_id: EntityId,
  update_server: &mut UpdateServer,
  view: &mut View,
  key: KeyCode,
) where UpdateServer: FnMut(ClientToServer)
{
  timers.time("event.key_press", || {
    match key {
      KeyCode::A => {
        update_server(Walk(player_id, Vector3::new(-1.0, 0.0, 0.0)));
      },
      KeyCode::D => {
        update_server(Walk(player_id, Vector3::new(1.0, 0.0, 0.0)));
      },
      KeyCode::Space => {
        update_server(StartJump(player_id));
      },
      KeyCode::W => {
        update_server(Walk(player_id, Vector3::new(0.0, 0.0, -1.0)));
      },
      KeyCode::S => {
        update_server(Walk(player_id, Vector3::new(0.0, 0.0, 1.0)));
      },
      KeyCode::Left => {
        update_server(RotatePlayer(player_id, Vector2::new(PI / 12.0, 0.0)));
        view.camera.rotate_lateral(PI / 12.0);
      },
      KeyCode::Right => {
        update_server(RotatePlayer(player_id, Vector2::new(-PI / 12.0, 0.0)));
        view.camera.rotate_lateral(-PI / 12.0);
      },
      KeyCode::Up => {
        update_server(RotatePlayer(player_id, Vector2::new(0.0, PI / 12.0)));
        view.camera.rotate_vertical(PI / 12.0);
      },
      KeyCode::Down => {
        update_server(RotatePlayer(player_id, Vector2::new(0.0, -PI / 12.0)));
        view.camera.rotate_vertical(-PI / 12.0);
      },
      _ => {},
    }
  })
}

fn key_release<UpdateServer>(
  timers: &TimerSet,
  player_id: EntityId,
  update_server: &mut UpdateServer,
  key: KeyCode,
) where UpdateServer: FnMut(ClientToServer)
{
  timers.time("event.key_release", || {
    match key {
      // accelerations are negated from those in key_press.
      KeyCode::A => {
        update_server(Walk(player_id, Vector3::new(1.0, 0.0, 0.0)));
      },
      KeyCode::D => {
        update_server(Walk(player_id, Vector3::new(-1.0, 0.0, 0.0)));
      },
      KeyCode::Space => {
        update_server(StopJump(player_id));
      },
      KeyCode::W => {
        update_server(Walk(player_id, Vector3::new(0.0, 0.0, 1.0)));
      },
      KeyCode::S => {
        update_server(Walk(player_id, Vector3::new(0.0, 0.0, -1.0)));
      },
      _ => {}
    }
  })
}

fn mouse_move<UpdateServer>(
  timers: &TimerSet,
  player_id: EntityId,
  update_server: &mut UpdateServer,
  view: &mut View,
  window: &mut video::Window,
  x: i32, y: i32,
) where UpdateServer: FnMut(ClientToServer)
{
  // x and y are measured from the top-left corner.

  timers.time("event.mouse_move", || {
    let (w, h) = window.get_size();
    let (cx, cy) = (w as i32 / 2, h as i32 / 2);
    let d = Vector2::new(x - cx, cy - y);
    // To-radians coefficient. Numbers closer to zero dull the mouse movement more.
    let to_radians = Vector2::new(-1.0 / 1000.0, 1.0 / 1600.0);
    let r = Vector2::new(d.x as f32 * to_radians.x, d.y as f32 * to_radians.y);

    update_server(RotatePlayer(player_id, r));
    view.camera.rotate_lateral(r.x);
    view.camera.rotate_vertical(r.y);

    mouse::warp_mouse_in_window(window, cx, cy);
  })
}
