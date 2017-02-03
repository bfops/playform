//! own a set of playing audio tracks and combine them into a single buffer

use std;
use std::sync::atomic;

/// one independent audio track
pub struct Track {
  // TODO: Make this a ref.
  data: Vec<f32>,
  idx: usize,
  repeat: bool,
}

impl Track {
  #[allow(missing_docs)]
  pub fn new(data: Vec<f32>, repeat: bool) -> Self {
    Track {
      data: data,
      idx: 0,
      repeat: repeat,
    }
  }
}

impl Track {
  #[allow(missing_docs)]
  pub fn is_done(&self) -> bool {
    !self.repeat && (self.idx >= self.data.len())
  }
}

impl Iterator for Track {
  type Item = f32;

  fn next(&mut self) -> Option<f32> {
    if self.is_done() {
      None
    } else {
      let r = self.data[self.idx];
      self.idx = self.idx + 1;
      if self.repeat && self.idx >= self.data.len() {
        self.idx = 0;
      }
      Some(r)
    }
  }
}

/// all the playing sounds
pub struct TracksPlaying {
  tracks: Vec<Track>,
  ready: atomic::AtomicBool,
  buffer: Vec<f32>,
}

unsafe impl Sync for TracksPlaying {}

impl TracksPlaying {
  #[allow(missing_docs)]
  pub fn new(buffer_len: usize) -> Self {
    TracksPlaying {
      tracks: Vec::new(),
      ready: atomic::AtomicBool::new(false),
      buffer: std::iter::repeat(0.0).take(buffer_len).collect(),
    }
  }

  /// add a new independently playing audio track
  pub fn push(&mut self, t: Track) {
    self.tracks.push(t);
  }

  /// fill the buffer with the next sample of all the tracks, mixed together
  pub fn refresh_buffer(&mut self) {
    if self.ready.load(atomic::Ordering::Acquire) {
      return
    }

    for x in &mut self.buffer {
      *x = 0.0;
    }

    for track in &mut self.tracks {
      for buffer in &mut self.buffer {
        match track.next() {
          None => break,
          Some(x) => *buffer = *buffer + x,
        }
      }
    }

    let mut i = 0;
    while i < self.tracks.len() {
      if self.tracks[i].is_done() {
        self.tracks.swap_remove(i);
      } else {
        i += 1;
      }
    }

    self.ready.store(true, atomic::Ordering::Release);
  }

  /// invoke a callback with the most recent audio sample
  pub fn with_buffer<F>(&mut self, f: F)
    where F: FnOnce(&mut [f32])
  {
    if self.ready.load(atomic::Ordering::Acquire) {
      f(&mut self.buffer);
      self.ready.store(false, atomic::Ordering::Release);
    }
  }
}
