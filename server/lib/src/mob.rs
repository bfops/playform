use cgmath::{Point3, Vector3};

use common::entity;
use common::surroundings_loader;

use lod;
use server;

pub type Behavior = fn(&server::T, &mut Mob);

pub struct Mob {
  pub position            : Point3<f32>,
  pub speed               : Vector3<f32>,
  pub behavior            : Behavior,

  pub entity_id           : entity::id::T,
  pub owner_id            : lod::OwnerId,
  pub surroundings_loader : surroundings_loader::T,
}
