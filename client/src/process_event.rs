//! SDL input event processing code.

use client_update::ViewToClient;
use client_update::ViewToClient::*;
use common::stopwatch::TimerSet;
use nalgebra::{Vec2, Vec3};
use view::View;
use sdl2::event::Event;
use sdl2::keycode::KeyCode;
use sdl2::mouse;
use sdl2::video;
use std::f32::consts::PI;
use std::sync::mpsc::Sender;

#[allow(missing_docs)]
pub fn process_event (
  timers: &TimerSet,
  world: &Sender<ViewToClient>,
  view: &mut View,
  game_window: &mut video::Window,
  event: Event,
) {
  match event {
    Event::KeyDown{keycode, repeat, ..} => {
      if !repeat {
        key_press(timers, world, view, keycode);
      }
    },
    Event::KeyUp{keycode, repeat, ..} => {
      if !repeat {
        key_release(timers, world, keycode);
      }
    },
    Event::MouseMotion{x, y, ..} => {
      mouse_move(timers, world, view, game_window, x, y);
    },
    _ => {},
  }
}

fn key_press<'a>(
  timers: &TimerSet,
  world: &Sender<ViewToClient>,
  view: &mut View,
  key: KeyCode,
) {
  timers.time("event.key_press", || {
    match key {
      KeyCode::A => {
        world.send(Walk(Vec3::new(-1.0, 0.0, 0.0))).unwrap();
      },
      KeyCode::D => {
        world.send(Walk(Vec3::new(1.0, 0.0, 0.0))).unwrap();
      },
      KeyCode::Space => {
        world.send(StartJump).unwrap();
      },
      KeyCode::W => {
        world.send(Walk(Vec3::new(0.0, 0.0, -1.0))).unwrap();
      },
      KeyCode::S => {
        world.send(Walk(Vec3::new(0.0, 0.0, 1.0))).unwrap();
      },
      KeyCode::Left => {
        world.send(RotatePlayer(Vec2::new(PI / 12.0, 0.0))).unwrap();
        view.camera.rotate_lateral(PI / 12.0);
      },
      KeyCode::Right => {
        world.send(RotatePlayer(Vec2::new(-PI / 12.0, 0.0))).unwrap();
        view.camera.rotate_lateral(-PI / 12.0);
      },
      KeyCode::Up => {
        world.send(RotatePlayer(Vec2::new(0.0, PI / 12.0))).unwrap();
        view.camera.rotate_vertical(PI / 12.0);
      },
      KeyCode::Down => {
        world.send(RotatePlayer(Vec2::new(0.0, -PI / 12.0))).unwrap();
        view.camera.rotate_vertical(-PI / 12.0);
      },
      _ => {},
    }
  })
}

fn key_release<'a>(
  timers: &TimerSet,
  world: &Sender<ViewToClient>,
  key: KeyCode,
) {
  timers.time("event.key_release", || {
    match key {
      // accelerations are negated from those in key_press.
      KeyCode::A => {
        world.send(Walk(Vec3::new(1.0, 0.0, 0.0))).unwrap();
      },
      KeyCode::D => {
        world.send(Walk(Vec3::new(-1.0, 0.0, 0.0))).unwrap();
      },
      KeyCode::Space => {
        world.send(StopJump).unwrap();
      },
      KeyCode::W => {
        world.send(Walk(Vec3::new(0.0, 0.0, 1.0))).unwrap();
      },
      KeyCode::S => {
        world.send(Walk(Vec3::new(0.0, 0.0, -1.0))).unwrap();
      },
      _ => {}
    }
  })
}

fn mouse_move<'a>(
  timers: &TimerSet,
  world: &Sender<ViewToClient>,
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

    world.send(RotatePlayer(r)).unwrap();
    view.camera.rotate_lateral(r.x);
    view.camera.rotate_vertical(r.y);

    mouse::warp_mouse_in_window(window, cx, cy);
  })
}
