//! Run a collection of closures in series.

/// Signals returned from constituent closures.
pub enum Return {
  /// Exit the closure series.
  Quit,
  /// Move to the next closure.
  Continue,
  /// Start the series from the beginning.
  Restart,
}

pub use self::Return::*;

/// The closure type used.
pub type Closure<'a> = Box<FnMut() -> Return + 'a>;

#[allow(missing_docs)]
pub struct T<'a> {
  closures: Vec<Closure<'a>>,
}

#[allow(missing_docs)]
pub fn new(closures: Vec<Closure>) -> T {
  assert!(!closures.is_empty());

  T {
    closures: closures,
  }
}

impl<'a> T<'a> {
  /// Keep running this closure series until a quit signal is received.
  pub fn until_quit(&mut self) {
    loop {
      for closure in &mut self.closures {
        match closure() {
          Return::Quit => return,
          Return::Restart => break,
          Return::Continue => {},
        }
      }
    }
  }
}
