//! SDL input event processing code.

use common::communicate::ClientToServer;
use common::communicate::ClientToServer::*;
use common::stopwatch::TimerSet;
use nalgebra::{Vec2, Vec3};
use view::View;
use sdl2::event::Event;
use sdl2::keycode::KeyCode;
use sdl2::mouse;
use sdl2::video;
use std::f32::consts::PI;
use std::sync::mpsc::Sender;
use std::sync::Mutex;

#[allow(missing_docs)]
pub fn process_event (
  timers: &TimerSet,
  ups_to_server: &Mutex<Sender<ClientToServer>>,
  view: &mut View,
  game_window: &mut video::Window,
  event: Event,
) {
  match event {
    Event::KeyDown{keycode, repeat, ..} => {
      if !repeat {
        key_press(timers, ups_to_server, view, keycode);
      }
    },
    Event::KeyUp{keycode, repeat, ..} => {
      if !repeat {
        key_release(timers, ups_to_server, keycode);
      }
    },
    Event::MouseMotion{x, y, ..} => {
      mouse_move(timers, ups_to_server, view, game_window, x, y);
    },
    _ => {},
  }
}

fn key_press<'a>(
  timers: &TimerSet,
  ups_to_server: &Mutex<Sender<ClientToServer>>,
  view: &mut View,
  key: KeyCode,
) {
  timers.time("event.key_press", || {
    match key {
      KeyCode::A => {
        ups_to_server.lock().unwrap().send(Walk(Vec3::new(-1.0, 0.0, 0.0))).unwrap();
      },
      KeyCode::D => {
        ups_to_server.lock().unwrap().send(Walk(Vec3::new(1.0, 0.0, 0.0))).unwrap();
      },
      KeyCode::Space => {
        ups_to_server.lock().unwrap().send(StartJump).unwrap();
      },
      KeyCode::W => {
        ups_to_server.lock().unwrap().send(Walk(Vec3::new(0.0, 0.0, -1.0))).unwrap();
      },
      KeyCode::S => {
        ups_to_server.lock().unwrap().send(Walk(Vec3::new(0.0, 0.0, 1.0))).unwrap();
      },
      KeyCode::Left => {
        ups_to_server.lock().unwrap().send(RotatePlayer(Vec2::new(PI / 12.0, 0.0))).unwrap();
        view.camera.rotate_lateral(PI / 12.0);
      },
      KeyCode::Right => {
        ups_to_server.lock().unwrap().send(RotatePlayer(Vec2::new(-PI / 12.0, 0.0))).unwrap();
        view.camera.rotate_lateral(-PI / 12.0);
      },
      KeyCode::Up => {
        ups_to_server.lock().unwrap().send(RotatePlayer(Vec2::new(0.0, PI / 12.0))).unwrap();
        view.camera.rotate_vertical(PI / 12.0);
      },
      KeyCode::Down => {
        ups_to_server.lock().unwrap().send(RotatePlayer(Vec2::new(0.0, -PI / 12.0))).unwrap();
        view.camera.rotate_vertical(-PI / 12.0);
      },
      _ => {},
    }
  })
}

fn key_release<'a>(
  timers: &TimerSet,
  ups_to_server: &Mutex<Sender<ClientToServer>>,
  key: KeyCode,
) {
  timers.time("event.key_release", || {
    match key {
      // accelerations are negated from those in key_press.
      KeyCode::A => {
        ups_to_server.lock().unwrap().send(Walk(Vec3::new(1.0, 0.0, 0.0))).unwrap();
      },
      KeyCode::D => {
        ups_to_server.lock().unwrap().send(Walk(Vec3::new(-1.0, 0.0, 0.0))).unwrap();
      },
      KeyCode::Space => {
        ups_to_server.lock().unwrap().send(StopJump).unwrap();
      },
      KeyCode::W => {
        ups_to_server.lock().unwrap().send(Walk(Vec3::new(0.0, 0.0, 1.0))).unwrap();
      },
      KeyCode::S => {
        ups_to_server.lock().unwrap().send(Walk(Vec3::new(0.0, 0.0, -1.0))).unwrap();
      },
      _ => {}
    }
  })
}

fn mouse_move<'a>(
  timers: &TimerSet,
  ups_to_server: &Mutex<Sender<ClientToServer>>,
  view: &mut View,
  window: &mut video::Window,
  x: i32, y: i32,
) {
  // x and y are measured from the top-left corner.

  timers.time("event.mouse_move", || {
    let (w, h) = window.get_size();
    let (cx, cy) = (w as i32 / 2, h as i32 / 2);
    let d = Vec2::new(x - cx, cy - y);
    // To-radians coefficient. Numbers closer to zero dull the mouse movement more.
    let to_radians = Vec2::new(-1.0 / 1000.0, 1.0 / 1600.0);
    let r = Vec2::new(d.x as f32 * to_radians.x, d.y as f32 * to_radians.y);

    ups_to_server.lock().unwrap().send(RotatePlayer(r)).unwrap();
    view.camera.rotate_lateral(r.x);
    view.camera.rotate_vertical(r.y);

    mouse::warp_mouse_in_window(window, cx, cy);
  })
}
