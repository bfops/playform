#![macro_escape]

extern crate time;

use std::cell::{RefCell, Ref};
use std::rc::Rc;
use std::collections::HashMap;

/// A simple stopwatch taht can time events and print stats about them.
#[deriving(Send)]
pub struct Stopwatch {
  pub total_time: u64,
  // number of time windows we've clocked
  pub number_of_windows: u64,
}

impl Stopwatch {
  #[inline]
  /// Creates a new stopwatch.
  pub fn new() -> Stopwatch {
    Stopwatch {
      total_time: 0,
      number_of_windows: 0,
    }
  }

  #[inline]
  /// Times a function, updating stats as necessary.
  pub fn timed<T>(&mut self, event: | | -> T) -> T {
    let then = time::precise_time_ns();
    let ret = event();
    self.total_time += time::precise_time_ns() - then;
    self.number_of_windows += 1;
    ret
  }

  /// Prints out timing statistics of this stopwatch.
  fn print(&self, name: &str) {
    if self.number_of_windows == 0 {
      info!("{} never ran", name);
    } else {
      info!(
        "{}: {}ms over {} samples (avg {}us)",
        name,
        self.total_time / 1000000,
        self.number_of_windows,
        (self.total_time / self.number_of_windows / 1000),
      );
    }
  }

}

/// A set of stopwatches for multiple, named events.
pub struct TimerSet {
  timers: RefCell<HashMap<String, Rc<RefCell<Stopwatch>>>>,
}

impl TimerSet {
  /// Creates a new set of timers.
  pub fn new() -> TimerSet {
    TimerSet { timers: RefCell::new(HashMap::new()) }
  }

  /// Times the execution of a function, and logs it under a timer with
  /// the given name.
  ///
  /// You probably want to use the `time!` macro instead of using this
  /// directly.
  ///
  /// This function is not marked `mut` because borrow checking is done
  /// dynamically.
  pub fn time<T>(&self, name: &str, f: || -> T) -> T {
    if !self.timers.borrow().contains_key_equiv(&name) {
      self.timers
          .borrow_mut()
          .insert(String::from_str(name), Rc::new(RefCell::new(Stopwatch::new())));
    };

    let timer : Rc<RefCell<Stopwatch>> =
      self.timers.borrow().find_equiv(&name).unwrap().clone();

    match timer.try_borrow_mut() {
      None => fail!("timer \"{}\" used recursively", name),
      Some(mut timer) => timer.timed(f),
    }
  }

  /// Prints all the timer statistics to stdout, each tagged with their name.
  pub fn print(&self) {
    let timers = self.timers.borrow();

    let mut timer_vec : Vec<(&str, Ref<Stopwatch>)> =
      timers
        .iter()
        .map(|(name, sw)| (name.as_slice(), sw.borrow()))
        .collect();

    timer_vec.sort_by(|&(k1, _), &(k2, _)| k1.cmp(&k2));

    for &(name, ref timer) in timer_vec.iter() {
      timer.print(name);
    }
  }
}

#[test]
fn test_simple() {
  let ts = TimerSet::new();
  ts.time("hello", || {});
}

#[test]
fn test_nested() {
  let ts = TimerSet::new();
  ts.time("hello", || {
    ts.time("world", || {});
  });
}

#[macro_export]
macro_rules! time(
  ($timers:expr, $name:expr, $f:expr) => (
    unsafe { ($timers as *const stopwatch::TimerSet).as_ref().unwrap() }.time($name, $f)
  );
)
