use common::*;
use event::{WindowSettings, EventIterator, EventSettings};
use event_handler::handle_event;
use sdl2_window::*;
use shader_version::opengl::*;
use state::App;

pub fn main() {
  debug!("starting");

  let mut window = Sdl2Window::new(
    OpenGL_3_3,
    WindowSettings {
      title: "playform".to_string(),
      size: [WINDOW_WIDTH as u32, WINDOW_HEIGHT as u32],
      fullscreen: false,
      exit_on_esc: false,
      samples: 0,
    }
  );

  let mut app = App::new();
  let mut game_iter =
    EventIterator::new(
      &mut window,
      &EventSettings {
        updates_per_second: 30,
        max_frames_per_second: 30,
      });

  loop {
    match game_iter.next() {
      None => break,
      Some(e) => handle_event(&mut app, game_iter.window, e)
    }
  }

  debug!("finished!");
}
