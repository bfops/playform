use cgmath::{Point3, Vector3};

use common::entity_id;
use common::surroundings_loader::SurroundingsLoader;

use lod;
use server::Server;

pub type Behavior = fn(&Server, &mut Mob);

pub struct Mob {
  pub position: Point3<f32>,
  pub speed: Vector3<f32>,
  pub behavior: Behavior,

  pub entity_id: entity_id::T,
  pub owner_id: lod::OwnerId,
  pub surroundings_loader: SurroundingsLoader,
}
