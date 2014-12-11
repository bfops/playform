use color::Color4;
use common::*;
use event::Event;
use input;
use input::{InputEvent,Button,Motion};
use input::keyboard::Key;
use nalgebra::Vec3;
use render::render;
use sdl2_window::*;
use sdl2::mouse;
use state::App;
use std::f32::consts::PI;
use update::update;
use vertex::ColoredVertex;

#[inline]
fn swap_remove_first<T: PartialEq + Copy>(v: &mut Vec<T>, t: T) {
  match v.iter().position(|x| *x == t) {
    None => { },
    Some(i) => { v.swap_remove(i); },
  }
}

pub fn handle_event<'a>(app: &mut App<'a>, game_window: &mut Sdl2Window, event: Event) {
  match event {
    Event::Render(_) => render(app),
    Event::Update(_) => update(app),
    Event::Input(ref i) => match *i {
      InputEvent::Press(Button::Keyboard(key)) => key_press(app, key),
      InputEvent::Release(Button::Keyboard(key)) => key_release(app, key),
      InputEvent::Press(Button::Mouse(button)) => mouse_press(app, button),
      InputEvent::Release(Button::Mouse(button)) => mouse_release(app, button),
      InputEvent::Move(Motion::MouseCursor(x, y)) => mouse_move(app, game_window, x, y),
      _ => {},
    },
  }
}

fn key_press<'a>(app: &mut App<'a>, key: input::keyboard::Key) {
  app.timers.time("event.key_press", || {
    match key {
      Key::A => {
        app.player.walk(Vec3::new(-1.0, 0.0, 0.0));
      },
      Key::D => {
        app.player.walk(Vec3::new(1.0, 0.0, 0.0));
      },
      Key::Space => {
        if !app.player.is_jumping {
          app.player.is_jumping = true;
          // this 0.3 is duplicated in a few places
          app.player.accel.y = app.player.accel.y + 0.3;
        }
      },
      Key::W => {
        app.player.walk(Vec3::new(0.0, 0.0, -1.0));
      },
      Key::S => {
        app.player.walk(Vec3::new(0.0, 0.0, 1.0));
      },
      Key::Left =>
        app.player.rotate_lateral(PI / 12.0),
      Key::Right =>
        app.player.rotate_lateral(-PI / 12.0),
      Key::Up =>
        app.player.rotate_vertical(PI / 12.0),
      Key::Down =>
        app.player.rotate_vertical(-PI / 12.0),
      Key::M => {
        let updates = [
          ColoredVertex {
            position: app.player.camera.position,
            color: Color4::of_rgba(1.0, 0.0, 0.0, 1.0),
          },
          ColoredVertex {
            position: app.player.camera.position + app.player.forward() * (32.0 as f32),
            color: Color4::of_rgba(1.0, 0.0, 0.0, 1.0),
          },
        ];
        app.line_of_sight.buffer.update(app.gl_context, 0, &updates);
      },
      Key::O => {
        app.render_octree = !app.render_octree;
      }
      Key::L => {
        app.render_outlines = !app.render_outlines;
      }
      _ => {},
    }
  })
}

fn key_release<'a>(app: &mut App<'a>, key: Key) {
  app.timers.time("event.key_release", || {
    match key {
      // accelerations are negated from those in key_press.
      Key::A => {
        app.player.walk(Vec3::new(1.0, 0.0, 0.0));
      },
      Key::D => {
        app.player.walk(Vec3::new(-1.0, 0.0, 0.0));
      },
      Key::Space => {
        if app.player.is_jumping {
          app.player.is_jumping = false;
          // this 0.3 is duplicated in a few places
          app.player.accel.y = app.player.accel.y - 0.3;
        }
      },
      Key::W => {
        app.player.walk(Vec3::new(0.0, 0.0, 1.0));
      },
      Key::S => {
        app.player.walk(Vec3::new(0.0, 0.0, -1.0));
      },
      _ => { }
    }
  })
}

fn mouse_move<'a>(app: &mut App<'a>, w: &mut Sdl2Window, x: f64, y: f64) {
  app.timers.time("event.mouse_move", || {
    let (cx, cy) = (WINDOW_WIDTH as f32 / 2.0, WINDOW_HEIGHT as f32 / 2.0);
    // args.y = h - args.y;
    // dy = args.y - cy;
    //  => dy = cy - args.y;
    let (dx, dy) = (x as f32 - cx, cy - y as f32);
    let (rx, ry) = (dx * -3.14 / 2048.0, dy * 3.14 / 1600.0);
    app.player.rotate_lateral(rx);
    app.player.rotate_vertical(ry);

    mouse::warp_mouse_in_window(
      &w.window,
      WINDOW_WIDTH as i32 / 2,
      WINDOW_HEIGHT as i32 / 2
    );
  })
}

fn mouse_press<'a>(app: &mut App<'a>, button: input::mouse::Button) {
  app.timers.time("event.mouse_press", || {
    app.mouse_buttons_pressed.push(button);
  })
}

fn mouse_release<'a>(app: &mut App<'a>, button: input::mouse::Button) {
  swap_remove_first(&mut app.mouse_buttons_pressed, button)
}
