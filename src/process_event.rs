use common::*;
use nalgebra::{Vec2, Vec3};
use view::View;
use sdl2::event::Event;
use sdl2::keycode::KeyCode;
use sdl2::mouse;
use sdl2::video;
use stopwatch::TimerSet;
use std::f32::consts::PI;
use std::sync::mpsc::Sender;
use world_thread::WorldUpdate;
use world_thread::WorldUpdate::*;

pub fn process_event (
  timers: &TimerSet,
  world: &Sender<WorldUpdate>,
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
  world: &Sender<WorldUpdate>,
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
        view.rotate_lateral(PI / 12.0);
      },
      KeyCode::Right => {
        world.send(RotatePlayer(Vec2::new(-PI / 12.0, 0.0))).unwrap();
        view.rotate_lateral(-PI / 12.0);
      },
      KeyCode::Up => {
        world.send(RotatePlayer(Vec2::new(0.0, PI / 12.0))).unwrap();
        view.rotate_vertical(PI / 12.0);
      },
      KeyCode::Down => {
        world.send(RotatePlayer(Vec2::new(0.0, -PI / 12.0))).unwrap();
        view.rotate_vertical(-PI / 12.0);
      },
      _ => {},
    }
  })
}

fn key_release<'a>(
  timers: &TimerSet,
  world: &Sender<WorldUpdate>,
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
  world: &Sender<WorldUpdate>,
  view: &mut View,
  window: &mut video::Window,
  x: i32, y: i32,
) {
  timers.time("event.mouse_move", || {
    let (cx, cy) = (WINDOW_WIDTH as i32 / 2, WINDOW_HEIGHT as i32 / 2);
    // y is measured from the top of the window.
    let (dx, dy) = (x - cx, cy - y);
    // magic numbers. Oh god why?
    let (rx, ry) = (dx as f32 * -3.14 / 2048.0, dy as f32 * 3.14 / 1600.0);

    world.send(RotatePlayer(Vec2::new(rx, ry))).unwrap();
    view.rotate_lateral(rx);
    view.rotate_vertical(ry);

    mouse::warp_mouse_in_window(
      window,
      WINDOW_WIDTH as i32 / 2,
      WINDOW_HEIGHT as i32 / 2
    );
  })
}
