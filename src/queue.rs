/// Circular bounded queue.
pub struct Queue<T> {
  pub contents: Vec<T>,
  /// index in `contents` the queue starts at
  pub head: uint,
  /// index in `contents` to place the next element at
  pub tail: uint,
  /// number of elements used
  pub length: uint,
}

impl<T: Clone> Queue<T> {
  pub fn new(capacity: uint) -> Queue<T> {
    Queue {
      contents: Vec::with_capacity(capacity),
      head: 0,
      tail: 0,
      length: 0,
    }
  }

  fn wrap(&self, i: uint) -> uint {
    let r =
      if i >= self.contents.capacity() {
        i - self.contents.capacity()
      } else {
        i
      };
    assert!(r < self.contents.capacity());
    r
  }

  pub fn push(&mut self, t: T) {
    assert!(
      self.length < self.contents.capacity(),
      "Bounded queue (capacity {}) exceeded",
      self.contents.len()
    );

    if self.contents.len() < self.contents.capacity() {
      self.contents.push(t);
    } else {
      *self.contents.get_mut(self.tail) = t;
    }

    self.tail = self.wrap(self.tail + 1);
    self.length += 1;
  }

  pub fn push_all(&mut self, ts: &[T]) {
    for t in ts.iter() {
      self.push(t.clone());
    }
  }

  pub fn pop(&mut self, count: uint) {
    assert!(count <= self.length);
    self.head = self.wrap(self.head + count);
    self.length -= count;
  }

  pub fn is_empty(&self) -> bool {
    self.length == 0
  }

  pub fn swap_remove(&mut self, idx: uint, count: uint) {
    assert!(count <= self.length);
    // TODO: do this in bigger chunks
    for i in range(0, count) {
      if self.tail > 0 {
        self.tail -= 1;
      } else {
        self.tail = self.contents.len() - 1;
      };

      let swapped = self.contents[self.tail].clone();
      let idx = self.wrap(self.head + idx + i);
      *self.contents.get_mut(idx) = swapped;
    }

    self.length -= count;
  }

  pub fn len(&self) -> uint {
    self.length
  }

  #[allow(dead_code)]
  pub fn clear(&mut self) {
    self.contents.clear();
    self.head = 0;
    self.tail = 0;
    self.length = 0;
  }

  pub fn slices<'a>(&'a self, low: uint, high: uint) -> (&'a [T], &'a [T]) {
    assert!(low <= self.length);
    assert!(high <= self.length);
    let low = self.wrap(self.head + low);
    let high = self.wrap(self.head + high);
    if low < high {
      (self.contents.slice(low, high), self.contents.slice(high, high))
    } else {
      (self.contents.slice(low, self.contents.len()), self.contents.slice(0, high))
    }
  }
}
