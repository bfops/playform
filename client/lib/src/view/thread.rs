//! This module defines the main function for the view/render/event thread.

use cgmath::{Vector2};
use gl;
use sdl2;
use sdl2::event::Event;
use sdl2::video;
use std;
use stopwatch;
use yaglw::gl_context::GLContext;

use common::protocol;

use client;
use process_event::process_event;
use view;

#[allow(missing_docs)]
pub const FRAMES_PER_SECOND: u64 = 30;

#[allow(missing_docs)]
pub const GL_MAJOR_VERSION: u8 = 3;
#[allow(missing_docs)]
pub const GL_MINOR_VERSION: u8 = 3;

enum ViewIteration {
  Quit,
  Continue,
}

#[allow(missing_docs)]
pub fn view_thread<UpdateServer>(
  client: &client::T,
  update_server: &mut UpdateServer,
) where
  UpdateServer: FnMut(protocol::ClientToServer),
{
  let sdl = sdl2::init().unwrap();
  let sdl_event = sdl.event().unwrap();
  let video = sdl.video().unwrap();
  let gl_attr = video.gl_attr();

  gl_attr.set_context_profile(video::GLProfile::Core);
  gl_attr.set_context_version(GL_MAJOR_VERSION, GL_MINOR_VERSION);

  // Open the window as fullscreen at the current resolution.
  let mut window =
    video.window(
      "Playform",
      800, 600,
    );
  let window = window.opengl();
  let window = window.build().unwrap();

  assert_eq!(gl_attr.context_profile(), video::GLProfile::Core);
  assert_eq!(gl_attr.context_version(), (GL_MAJOR_VERSION, GL_MINOR_VERSION));

  let mut event_pump = sdl.event_pump().unwrap();

  let _sdl_gl_context = window.gl_create_context().unwrap();

  // Load the OpenGL function pointers.
  gl::load_with(|s| video.gl_get_proc_address(s) as *const _ );

  let gl = unsafe {
    GLContext::new()
  };

  gl.print_stats();

  let window_size = {
    let (w, h) = window.size();
    Vector2::new(w as i32, h as i32)
  };

  let mut view = view::new(gl, window_size);

  loop {
    let view_iteration =
      stopwatch::time("view_iteration", || {
        event_pump.pump_events();
        let events: Vec<Event> = sdl_event.peek_events(1 << 6);
        sdl_event.flush_events(0, std::u32::MAX);
        for event in events {
          match event {
            Event::Quit{..} => {
              return ViewIteration::Quit
            }
            Event::AppTerminating{..} => {
              return ViewIteration::Quit
            }
            event => {
              process_event(
                update_server,
                &mut view,
                &client,
                event,
              );
            },
          }
        }

        ViewIteration::Continue
      });

    match view_iteration {
      ViewIteration::Quit => break,
      ViewIteration::Continue => {},
    }
  }

  debug!("view exiting.");
}
