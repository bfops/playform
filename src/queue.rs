// for testing
#[allow(unused_imports)]
use std::iter::range_inclusive;

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

#[test]
fn push_then_slice() {
  let elems: Vec<int> = Vec::from_slice([1, 2, 3]);
  let mut q = Queue::new(32);
  q.push_all(elems.as_slice());
  assert!(q.len() == elems.len());
  let (l, h) = q.slices(0, q.len() - 1);
  assert!(l.len() + h.len() == elems.len() - 1);
  for (i, elem) in l.iter().enumerate() {
    assert!(*elem == elems[i]);
  }
  for (i, elem) in h.iter().enumerate() {
    assert!(*elem == elems[i + l.len()]);
  }
}

#[test]
fn push_then_pop() {
  let popped_pushes: Vec<int> = Vec::from_slice([1, 2, 3]);
  let more_pushes: Vec<int> = Vec::from_slice([4, 5]);
  let mut q = Queue::new(32);
  q.push_all(popped_pushes.as_slice());
  q.push_all(more_pushes.as_slice());
  assert!(q.len() == popped_pushes.len() + more_pushes.len());
  q.pop(popped_pushes.len());
  assert!(q.len() == more_pushes.len());
  let (l, h) = q.slices(0, q.len());
  assert!(l.len() + h.len() == more_pushes.len());
  for (i, elem) in l.iter().enumerate() {
    assert!(*elem == more_pushes[i]);
  }
  for (i, elem) in h.iter().enumerate() {
    assert!(*elem == more_pushes[i + l.len()]);
  }
}

#[test]
fn wrapped_pushes() {
  static capacity: uint = 32;
  let popped_pushes = Vec::from_elem(capacity / 2, 0);
  let more_pushes: Vec<int> = range_inclusive(1, capacity as int).collect();
  let mut q: Queue<int> = Queue::new(capacity);
  q.push_all(popped_pushes.as_slice());
  q.pop(popped_pushes.len());
  q.push_all(more_pushes.as_slice());
  assert!(q.len() == more_pushes.len());
  let (l, h) = q.slices(0, q.len());
  assert!(l.len() + h.len() == more_pushes.len());
  assert!(h.len() > 0);
  for (i, elem) in l.iter().enumerate() {
    assert!(*elem == more_pushes[i]);
  }
  for (i, elem) in h.iter().enumerate() {
    assert!(*elem == more_pushes[i + l.len()]);
  }
}
