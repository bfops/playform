use common::*;
use gl;
use init::init;
use interval_timer::IntervalTimer;
use log;
use logger::Logger;
use opencl_context::CL;
use process_event::process_event;
use render::render;
use renderer::Renderer;
use sdl2;
use sdl2::event::Event;
use std::mem;
use std::time::duration::Duration;
use std::old_io::timer;
use stopwatch::TimerSet;
use time;
use update::update;
use yaglw::gl_context::GLContext;

pub const FRAMES_PER_SECOND: u64 = 30;
pub const UPDATES_PER_SECOND: u64 = 30;

pub fn main() {
  log::set_logger(|max_log_level| {
    max_log_level.set(log::LogLevelFilter::Debug);
    Box::new(Logger)
  }).unwrap();

  debug!("starting");

  let timers = TimerSet::new();

  let cl = unsafe {
    CL::new()
  };

  sdl2::init(sdl2::INIT_EVERYTHING);

  sdl2::video::gl_set_attribute(sdl2::video::GLAttr::GLContextMajorVersion, 3);
  sdl2::video::gl_set_attribute(sdl2::video::GLAttr::GLContextMinorVersion, 3);
  sdl2::video::gl_set_attribute(
    sdl2::video::GLAttr::GLContextProfileMask,
    sdl2::video::GLProfile::GLCoreProfile as i32,
  );

  let mut window =
    sdl2::video::Window::new(
      "playform",
      sdl2::video::WindowPos::PosCentered,
      sdl2::video::WindowPos::PosCentered,
      WINDOW_WIDTH as i32,
      WINDOW_HEIGHT as i32,
      sdl2::video::OPENGL,
    ).unwrap();

  // Send text input events.
  sdl2::keyboard::start_text_input();

  {
    let _sdl_gl_context = window.gl_create_context().unwrap();

    // Load the OpenGL function pointers.
    gl::load_with(|s| unsafe {
      mem::transmute(sdl2::video::gl_get_proc_address(s))
    });

    let gl = unsafe {
      GLContext::new()
    };

    gl.print_stats();

    let mut renderer = Renderer::new(gl);

    let mut app = init(&mut renderer, &cl, &timers);

    let mut render_timer;
    let mut update_timer;
    {
      let now = time::precise_time_ns();
      let nanoseconds_per_second = 1000000000;
      render_timer = IntervalTimer::new(nanoseconds_per_second / FRAMES_PER_SECOND, now);
      update_timer = IntervalTimer::new(nanoseconds_per_second / UPDATES_PER_SECOND, now);
    }

    let mut has_focus = true;

    'game_loop:loop {
      let updates = update_timer.update(time::precise_time_ns());
      if updates > 0 {
        update(&timers, &mut app, &mut renderer, &cl);
      }

      let renders = render_timer.update(time::precise_time_ns());
      if renders > 0 {
        render(&timers, &mut app, &mut renderer);
        // swap buffers
        window.gl_swap_window();
      }

      'event_loop:loop {
        match sdl2::event::poll_event() {
          Event::None => {
            break 'event_loop;
          },
          Event::Quit{..} => {
            break 'game_loop;
          }
          Event::AppTerminating{..} => {
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
              process_event(&timers, &mut app, &mut window, event);
            }
          },
        }
      }
      timer::sleep(Duration::microseconds(10));
    }
  }

  info!("Update Stats");
  info!("====================");
  timers.print();

  debug!("finished");
}
