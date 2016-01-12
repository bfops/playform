use cgmath::{Point3, Vector3};

use common::entity::EntityId;
use common::lod::OwnerId;
use common::surroundings_loader::SurroundingsLoader;

use server::Server;

pub type Behavior = fn(&Server, &mut Mob);

pub struct Mob {
  pub position: Point3<f32>,
  pub speed: Vector3<f32>,
  pub behavior: Behavior,

  pub entity_id: EntityId,
  pub owner_id: OwnerId,
  pub surroundings_loader: SurroundingsLoader,
}
