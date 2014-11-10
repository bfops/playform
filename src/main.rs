use common::*;
use current::Modifier;
use event::WindowSettings;
use event::{Events, Ups, MaxFps};
use event_handler::handle_event;
use glw::gl_context::GLContext;
use sdl2_window::*;
use shader_version::opengl::*;
use state::App;
use std::cell::RefCell;
use time;

pub fn main() {
  debug!("starting");

  let window = Sdl2Window::new(
    OpenGL_3_3,
    WindowSettings {
      title: "playform".to_string(),
      size: [WINDOW_WIDTH as u32, WINDOW_HEIGHT as u32],
      fullscreen: false,
      exit_on_esc: false,
      samples: 0,
    }
  );
  let window = RefCell::new(window);

  let (gl, mut gl_context) = unsafe {
    GLContext::new()
  };

  let then = time::precise_time_ns();
  let mut app = App::new(&gl, &mut gl_context);
  let time_elapsed = time::precise_time_ns() - then;
  println!("load finished after {}ms", time_elapsed / 1000000);

  let mut game_iter = Events::new(&window);
  Ups(30).modify(&mut game_iter);
  MaxFps(30).modify(&mut game_iter);

  loop {
    match game_iter.next() {
      None => break,
      Some(e) => handle_event(&mut app, game_iter.window.borrow_mut().deref_mut(), e)
    }
  }

  debug!("finished!");
}
