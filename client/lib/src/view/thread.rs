//! This module defines the main function for the view/render/event thread.

use cgmath::{Vector2};
use gl;
use stopwatch;
use time;
use yaglw::gl_context::GLContext;
use glutin::{self, GlContext, Event, WindowEvent, KeyboardInput, VirtualKeyCode};

use common::interval_timer::IntervalTimer;
use common::protocol;

use client;
use hud::make_hud;
use process_event::process_event;
use view;

use super::update;

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

fn change_cursor_state(window: &glutin::Window, focused: bool) {
  if focused {
    let size = window.get_inner_size().expect("Window closed");
    window.set_cursor_position(size.0 as i32 / 2, size.1 as i32 / 2).unwrap();
  }
  window.set_cursor_state(if focused {
    glutin::CursorState::Grab
  } else {
    glutin::CursorState::Normal
  }).unwrap();
}

#[allow(missing_docs)]
pub fn view_thread<Recv0, Recv1, UpdateServer>(
  client: &client::T,
  recv0: &mut Recv0,
  recv1: &mut Recv1,
  update_server: &mut UpdateServer,
) where
  Recv0: FnMut() -> Option<update::T>,
  Recv1: FnMut() -> Option<update::T>,
  UpdateServer: FnMut(protocol::ClientToServer),
{
  let mut event_loop = glutin::EventsLoop::new();
  let window = glutin::WindowBuilder::new()
    .with_title("Playform")
    .with_dimensions(800, 600);
  let context = glutin::ContextBuilder::new()
    .with_gl(glutin::GlRequest::Specific(glutin::Api::OpenGl, (GL_MAJOR_VERSION, GL_MINOR_VERSION)))
    .with_gl_profile(glutin::GlProfile::Core);
  let window = glutin::GlWindow::new(window, context, &event_loop).unwrap();
  //window.set_maximized(true);
  window.show();
  change_cursor_state(&*window, true);

  // Initialize OpenGL
  unsafe { window.make_current().unwrap(); }
  gl::load_with(|s| window.get_proc_address(s) as *const _ );
  let gl = unsafe { GLContext::new() };

  gl.print_stats();

  let window_size = {
    let (w, h) = window.get_inner_size().expect("Window closed");
    Vector2::new(w as i32, h as i32)
  };

  let mut view = view::new(gl, window_size);

  make_hud(&mut view);

  let render_interval = {
    let nanoseconds_per_second = 1000000000;
    nanoseconds_per_second / FRAMES_PER_SECOND
  };
  let mut render_timer;
  {
    let now = time::precise_time_ns();
    render_timer = IntervalTimer::new(render_interval, now);
  }

  let mut last_update = time::precise_time_ns();
  let mut key_repeated = false;

  loop {
    let view_iteration =
      stopwatch::time("view_iteration", || {
        let mut will_quit = false;
        let now = time::precise_time_ns();
        if now - last_update >= render_interval {
          warn!("{:?}ms since last view update", (now - last_update) / 1000000);
        }
        last_update = now;

        event_loop.poll_events(|event| {
          match event {
            Event::WindowEvent { event: WindowEvent::Focused(focused), .. } => {
              change_cursor_state(&*window, focused)
            }
            Event::WindowEvent { event: WindowEvent::Closed, .. }  => will_quit = true,
            Event::WindowEvent {
              event: WindowEvent::KeyboardInput {
                input: KeyboardInput {
                  virtual_keycode: Some(key_code),
                  modifiers,
                  ..
                },
                ..
              },
              ..
            } => match key_code {
              VirtualKeyCode::Q => will_quit = true,
              VirtualKeyCode::F4 => {
                if modifiers.alt {
                  will_quit = true;
                }
              }
              VirtualKeyCode::Escape => {
                change_cursor_state(&*window, false);
              }
              _ => {}
            }
            _ => {}
          }
          process_event(
            update_server,
            &mut view,
            &client,
            event,
            &mut key_repeated,
          );
        });

        if will_quit {
          return ViewIteration::Quit;
        }

        stopwatch::time("apply_updates", || {
          let start = time::precise_time_ns();
          loop {
            if let Some(update) = recv0() {
              update::apply_client_to_view(&mut view, update);
            } else if let Some(update) = recv1() {
              update::apply_client_to_view(&mut view, update);
            } else {
              info!("Out of view updates");
              break
            }

            if time::precise_time_ns() - start >= 1_000_000 {
              break
            }
          }
        });

        let renders = render_timer.update(time::precise_time_ns());
        if renders > 0 {
          stopwatch::time("render", || {
            view::render::render(&mut view);
            // swap buffers
            window.swap_buffers().unwrap();
          });
        }

        ViewIteration::Continue
      });

    match view_iteration {
      ViewIteration::Quit => break,
      ViewIteration::Continue => {},
    }
  }
  change_cursor_state(&*window, false);

  debug!("view exiting.");
}
