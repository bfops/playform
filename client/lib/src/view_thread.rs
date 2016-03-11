//! This module defines the main function for the view/render/event thread.

use cgmath::{Vector2};
use gl;
use sdl2;
use sdl2::event::Event;
use sdl2::video;
use sdl2_sys;
use stopwatch;
use time;
use yaglw::gl_context::GLContext;

use common::interval_timer::IntervalTimer;
use common::protocol;

use client;
use hud::make_hud;
use process_event::process_event;
use render::render;
use view;
use view_update::{ClientToView, apply_client_to_view};

pub const FRAMES_PER_SECOND: u64 = 30;

pub const GL_MAJOR_VERSION: u8 = 3;
pub const GL_MINOR_VERSION: u8 = 3;

pub const WINDOW_WIDTH: u32 = 800;
pub const WINDOW_HEIGHT: u32 = 600;

enum ViewIteration {
  Quit,
  Continue,
}

#[allow(missing_docs)]
pub fn view_thread<Recv0, Recv1, UpdateServer>(
  client: &client::T,
  recv0: &mut Recv0,
  recv1: &mut Recv1,
  update_server: &mut UpdateServer,
) where
  Recv0: FnMut() -> Option<ClientToView>,
  Recv1: FnMut() -> Option<ClientToView>,
  UpdateServer: FnMut(protocol::ClientToServer),
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
      WINDOW_WIDTH, WINDOW_HEIGHT,
    );

  let window = window.position(0, 0);
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

  let mut view = view::T::new(gl, window_size);

  sdl.mouse().set_relative_mouse_mode(true);

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

  loop {
    let view_iteration =
      stopwatch::time("view_iteration", || {
        let now = time::precise_time_ns();
        if now - last_update >= render_interval {
          warn!("{:?}ms since last view update", (now - last_update) / 1000000);
        }
        last_update = now;

        for event in event_pump.poll_iter() {
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

        if window.window_flags() & (sdl2_sys::video::SDL_WindowFlags::SDL_WINDOW_MOUSE_FOCUS as u32) != 0 {
          sdl.mouse().warp_mouse_in_window(&window, WINDOW_WIDTH as i32 / 2, WINDOW_HEIGHT as i32 / 2);
        }

        stopwatch::time("apply_view_updates", || {
          let start = time::precise_time_ns();
          loop {
            if let Some(update) = recv0() {
              apply_client_to_view(&mut view, update);
            } else if let Some(update) = recv1() {
              apply_client_to_view(&mut view, update);
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
            render(&mut view);
            // swap buffers
            window.gl_swap_window();
          });
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
