use common::*;
use nalgebra::Vec3;
use sdl2::event::Event;
use sdl2::keycode::KeyCode;
use sdl2::mouse;
use sdl2::video;
use state::App;
use stopwatch::TimerSet;
use std::f32::consts::PI;

pub fn process_event<'a>(
  timers: &TimerSet,
  app: &mut App<'a>,
  game_window: &mut video::Window,
  event: Event,
) {
  match event {
    Event::KeyDown{keycode, repeat, ..} => {
      if !repeat {
        key_press(timers, app, keycode);
      }
    },
    Event::KeyUp{keycode, repeat, ..} => {
      if !repeat {
        key_release(timers, app, keycode);
      }
    },
    Event::MouseMotion{x, y, ..} => {
      mouse_move(timers, app, game_window, x, y);
    },
    _ => {},
  }
}

fn key_press<'a>(
  timers: &TimerSet,
  app: &mut App<'a>,
  key: KeyCode,
) {
  timers.time("event.key_press", || {
    match key {
      KeyCode::A => {
        app.player.walk(Vec3::new(-1.0, 0.0, 0.0));
      },
      KeyCode::D => {
        app.player.walk(Vec3::new(1.0, 0.0, 0.0));
      },
      KeyCode::Space if !app.player.is_jumping => {
        app.player.is_jumping = true;
        // this 0.3 is duplicated in a few places
        app.player.accel.y = app.player.accel.y + 0.3;
      },
      KeyCode::W => {
        app.player.walk(Vec3::new(0.0, 0.0, -1.0));
      },
      KeyCode::S => {
        app.player.walk(Vec3::new(0.0, 0.0, 1.0));
      },
      KeyCode::Left =>
        app.player.rotate_lateral(PI / 12.0),
      KeyCode::Right =>
        app.player.rotate_lateral(-PI / 12.0),
      KeyCode::Up =>
        app.player.rotate_vertical(PI / 12.0),
      KeyCode::Down =>
        app.player.rotate_vertical(-PI / 12.0),
      KeyCode::L => {
        app.render_outlines = !app.render_outlines;
      }
      _ => {},
    }
  })
}

fn key_release<'a>(timers: &TimerSet, app: &mut App<'a>, key: KeyCode) {
  timers.time("event.key_release", || {
    match key {
      // accelerations are negated from those in key_press.
      KeyCode::A => {
        app.player.walk(Vec3::new(1.0, 0.0, 0.0));
      },
      KeyCode::D => {
        app.player.walk(Vec3::new(-1.0, 0.0, 0.0));
      },
      KeyCode::Space if app.player.is_jumping => {
        app.player.is_jumping = false;
        // this 0.3 is duplicated in a few places
        app.player.accel.y = app.player.accel.y - 0.3;
      },
      KeyCode::W => {
        app.player.walk(Vec3::new(0.0, 0.0, 1.0));
      },
      KeyCode::S => {
        app.player.walk(Vec3::new(0.0, 0.0, -1.0));
      },
      _ => {}
    }
  })
}

fn mouse_move<'a>(
  timers: &TimerSet,
  app: &mut App<'a>,
  window: &mut video::Window,
  x: i32, y: i32,
) {
  timers.time("event.mouse_move", || {
    let (cx, cy) = (WINDOW_WIDTH as i32 / 2, WINDOW_HEIGHT as i32 / 2);
    // y is measured from the top of the window.
    let (dx, dy) = (x - cx, cy - y);
    // magic numbers. Oh god why?
    let (rx, ry) = (dx as f32 * -3.14 / 2048.0, dy as f32 * 3.14 / 1600.0);
    app.player.rotate_lateral(rx);
    app.player.rotate_vertical(ry);

    mouse::warp_mouse_in_window(
      window,
      WINDOW_WIDTH as i32 / 2,
      WINDOW_HEIGHT as i32 / 2
    );
  })
}
