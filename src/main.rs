use common::*;
use gl;
use interval_timer::IntervalTimer;
use log;
use logger::Logger;
use process_event::process_event;
use render::render;
use view::View;
use sdl2;
use sdl2::event::Event;
use std::mem;
use std::old_io::timer;
use std::sync::mpsc::{channel, TryRecvError};
use std::thread::Thread;
use std::time::duration::Duration;
use stopwatch::TimerSet;
use time;
use world::{WorldUpdate, world_thread};
use yaglw::gl_context::GLContext;

pub const FRAMES_PER_SECOND: u64 = 30;

pub fn main() {
  log::set_logger(|max_log_level| {
    max_log_level.set(log::LogLevelFilter::Debug);
    Box::new(Logger)
  }).unwrap();

  debug!("starting");

  let timers = TimerSet::new();

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

    let mut view = View::new(gl);

    let mut render_timer;
    {
      let now = time::precise_time_ns();
      let nanoseconds_per_second = 1000000000;
      render_timer = IntervalTimer::new(nanoseconds_per_second / FRAMES_PER_SECOND, now);
    }

    let mut has_focus = true;

    let (world_updates_send, world_updates_recv) = channel();
    let (view_send, view_recv) = channel();

    let _thread = Thread::spawn(|| world_thread(world_updates_recv, view_send));
    let world_updates = world_updates_send;

    'game_loop:loop {
      'event_loop:loop {
        match sdl2::event::poll_event() {
          Event::None => {
            break 'event_loop;
          },
          Event::Quit{..} => {
            world_updates.send(WorldUpdate::Quit).unwrap();
            break 'game_loop;
          }
          Event::AppTerminating{..} => {
            world_updates.send(WorldUpdate::Quit).unwrap();
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
                &world_updates,
                &mut view,
                &mut window,
                event,
              );
            }
          },
        }
      }

      'event_loop:loop {
        let event;
        match view_recv.try_recv() {
          Err(TryRecvError::Empty) => break 'event_loop,
          Err(e) => panic!("Error getting view updates: {:?}", e),
          Ok(e) => event = e,
        };
        event.apply(&mut view);
      }

      let renders = render_timer.update(time::precise_time_ns());
      if renders > 0 {
        render(&timers, &mut view);
        // swap buffers
        window.gl_swap_window();
      }

      timer::sleep(Duration::milliseconds(0));
    }
  }

  info!("Update Stats");
  info!("====================");
  timers.print();

  debug!("finished");
}
