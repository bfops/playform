extern crate time;

pub struct Stopwatch {
  pub total_time: u64,
  // number of time windows we've clocked
  pub number_of_windows: u64,
}

impl Stopwatch {
  #[inline]
  pub fn new() -> Stopwatch {
    Stopwatch {
      total_time: 0,
      number_of_windows: 0,
    }
  }

  #[inline]
  pub fn timed<T>(&mut self, event: | | -> T) -> T {
    let then = time::precise_time_ns();
    let ret = event();
    self.total_time += time::precise_time_ns() - then;
    self.number_of_windows += 1;
    ret
  }
}
