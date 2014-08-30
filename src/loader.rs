use glw::queue::Queue;

#[deriving(Clone)]
pub enum Operation<L, U> {
  Load(L),
  Unload(U),
}

pub type Loader<L, U> = Queue<Operation<L, U>>;
