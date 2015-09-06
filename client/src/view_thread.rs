//! This module defines the main function for the view/render/event thread.

use cgmath::Vector2;
use gl;
use sdl2;
use sdl2::event::Event;
use sdl2::video;
use std::mem;
use std::thread;
use stopwatch;
use time;
use yaglw::gl_context::GLContext;

use common::communicate::ClientToServer;
use common::entity::EntityId;
use common::interval_timer::IntervalTimer;

use hud::make_hud;
use process_event::process_event;
use render::render;
use view::View;
use view_update::{ClientToView, apply_client_to_view};

pub const FRAMES_PER_SECOND: u64 = 30;

pub const GL_MAJOR_VERSION: u8 = 3;
pub const GL_MINOR_VERSION: u8 = 3;

#[allow(missing_docs)]
pub fn view_thread<Recv, UpdateServer>(
  player_id: EntityId,
  recv: &mut Recv,
  update_server: &mut UpdateServer,
) where
  Recv: FnMut() -> Option<ClientToView>,
  UpdateServer: FnMut(ClientToServer),
{
  let sdl = sdl2::init().unwrap();
  let _event = sdl.event().unwrap();
  let video = sdl.video().unwrap();
  let gl_attr = video.gl_attr();

  gl_attr.set_context_profile(video::GLProfile::Core);
  gl_attr.set_context_version(GL_MAJOR_VERSION, GL_MINOR_VERSION);

  // Open the window as fullscreen at the current resolution.
  let mut window =
    video.window(
      "Playform",
      1200, 1024,
    );

  let window = window.position(0, 0);
  let window = window.opengl();

  let mut window = window.build().unwrap();

  assert_eq!(gl_attr.context_profile(), video::GLProfile::Core);
  assert_eq!(gl_attr.context_version(), (GL_MAJOR_VERSION, GL_MINOR_VERSION));

  let mut event_pump = sdl.event_pump().unwrap();

  let _sdl_gl_context = window.gl_create_context().unwrap();

  // Load the OpenGL function pointers.
  gl::load_with(|s| unsafe {
    mem::transmute(video.gl_get_proc_address(s))
  });

  let gl = unsafe {
    GLContext::new()
  };

  gl.print_stats();

  let window_size = {
    let (w, h) = window.size();
    Vector2::new(w as i32, h as i32)
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
      match event_pump.poll_event() {
        None => {
          break 'event_loop;
        },
        Some(Event::Quit{..}) => {
          break 'game_loop;
        }
        Some(Event::AppTerminating{..}) => {
          break 'game_loop;
        }
        Some(Event::Window{win_event_id: event_id, ..}) => {
          // Manage has_focus so that we don't capture the cursor when the
          // window is in the background
          match event_id {
            sdl2::event::WindowEventId::FocusGained => {
              has_focus = true;
              sdl.mouse().show_cursor(false);
            }
            sdl2::event::WindowEventId::FocusLost => {
              has_focus = false;
              sdl.mouse().show_cursor(true);
            }
            _ => {}
          }
        }
        Some(event) => {
          if has_focus {
            process_event(
              &sdl,
              player_id,
              update_server,
              &mut view,
              &mut window,
              event,
            );
          }
        },
      }
    }

    while let Some(update) = recv() {
      apply_client_to_view(update, &mut view);
    }

    let renders = render_timer.update(time::precise_time_ns());
    if renders > 0 {
      render(&mut view);
      // swap buffers
      window.gl_swap_window();
    }

    thread::yield_now();
  }

  stopwatch::clone().print();

  debug!("view exiting.");
}
