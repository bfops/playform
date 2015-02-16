//! This module defines the main function for the view/render/event thread.

use client_update::ViewToClient;
use common::interval_timer::IntervalTimer;
use common::process_events::process_channel;
use common::stopwatch::TimerSet;
use gl;
use hud::make_hud;
use nalgebra::Vec2;
use process_event::process_event;
use render::render;
use sdl2;
use sdl2::event::Event;
use sdl2::video;
use std::mem;
use std::old_io::timer;
use std::sync::mpsc::{Sender, Receiver};
use std::time::duration::Duration;
use time;
use view::View;
use view_update::{ClientToView, apply_client_to_view};
use yaglw::gl_context::GLContext;

pub const FRAMES_PER_SECOND: u64 = 30;

#[allow(missing_docs)]
pub fn view_thread(
  ups_from_client: Receiver<ClientToView>,
  ups_to_client: Sender<ViewToClient>,
) {
  let ups_from_client = &ups_from_client;
  let ups_to_client = &ups_to_client;

  let timers = TimerSet::new();

  sdl2::init(sdl2::INIT_EVERYTHING);

  video::gl_set_attribute(video::GLAttr::GLContextMajorVersion, 3);
  video::gl_set_attribute(video::GLAttr::GLContextMinorVersion, 3);
  video::gl_set_attribute(
    video::GLAttr::GLContextProfileMask,
    video::GLProfile::GLCoreProfile as i32,
  );

  let display_mode = video::get_desktop_display_mode(0).unwrap();

  // Open the window as fullscreen at the current resolution.
  let mut window =
    video::Window::new(
      "Playform",
      video::WindowPos::PosUndefined,
      video::WindowPos::PosUndefined,
      display_mode.w,
      display_mode.h,
      video::OPENGL | video::FULLSCREEN_DESKTOP,
    ).unwrap();

  // Send text input events.
  sdl2::keyboard::start_text_input();

  let _sdl_gl_context = window.gl_create_context().unwrap();

  // Load the OpenGL function pointers.
  gl::load_with(|s| unsafe {
    mem::transmute(video::gl_get_proc_address(s))
  });

  let gl = unsafe {
    GLContext::new()
  };

  gl.print_stats();

  let window_size = {
    let (w, h) = window.get_size();
    Vec2::new(w, h)
  };
  let mut view = View::new(gl, window_size);

  make_hud(&mut view);

  let mut render_timer;
  {
    let now = time::precise_time_ns();
    let nanoseconds_per_second = 1000000000;
    render_timer = IntervalTimer::new(nanoseconds_per_second / FRAMES_PER_SECOND, now);
  }

  let mut has_focus = true;

  'game_loop:loop {
    'event_loop:loop {
      match sdl2::event::poll_event() {
        Event::None => {
          break 'event_loop;
        },
        Event::Quit{..} => {
          ups_to_client.send(ViewToClient::Quit).unwrap();
          break 'game_loop;
        }
        Event::AppTerminating{..} => {
          ups_to_client.send(ViewToClient::Quit).unwrap();
          break 'game_loop;
        }
        Event::Window{win_event_id: event_id, ..} => {
          // Manage has_focus so that we don't capture the cursor when the
          // window is in the background
          match event_id {
            sdl2::event::WindowEventId::FocusGained => {
              has_focus = true;
              sdl2::mouse::show_cursor(false);
            }
            sdl2::event::WindowEventId::FocusLost => {
              has_focus = false;
              sdl2::mouse::show_cursor(true);
            }
            _ => {}
          }
        }
        event => {
          if has_focus {
            process_event(
              &timers,
              &ups_to_client,
              &mut view,
              &mut window,
              event,
            );
          }
        },
      }
    }

    process_channel(
      ups_from_client,
      |update| {
        apply_client_to_view(update, &mut view);
        true
      },
    );

    let renders = render_timer.update(time::precise_time_ns());
    if renders > 0 {
      render(&timers, &mut view);
      // swap buffers
      window.gl_swap_window();
    }

    timer::sleep(Duration::milliseconds(0));
  }

  timers.print();

  debug!("view exiting.");
}
