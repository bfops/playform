use nalgebra::Pnt3;

pub struct Client {
  pub player_position: Pnt3<f32>,
}

impl Client {
  pub fn new() -> Client {
    Client {
      player_position: Pnt3::new(0.0, 0.0, 0.0),
    }
  }
}
