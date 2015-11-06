use cgmath::{Point, Point3, Vector, Vector3, Aabb3};
use rand;
use rand::distributions::IndependentSample;
use std::convert::AsRef;
use std::f32::consts::PI;
use std::ops::DerefMut;
use std::time::Duration;
use stopwatch;

use common::entity_id;
use common::protocol;
use common::socket::SendSocket;
use common::voxel;

use player::Player;
use server::{Client, Server};
use terrain;
use voxel_data;
use update_gaia;
use update_gaia::LoadReason;

fn center(bounds: &Aabb3<f32>) -> Point3<f32> {
  bounds.min.add_v(&bounds.max.to_vec()).mul_s(1.0 / 2.0)
}

fn cast(
  server: &Server,
  player_id: entity_id::T,
) -> Option<voxel_data::bounds::T> {
  let ray;
  {
    let players = server.players.lock().unwrap();
    let player = players.get(&player_id).unwrap();
    ray = player.forward_ray();
  }

  server.terrain_loader.terrain.voxels.lock().unwrap().cast_ray(
    &ray,
    &mut |bounds, voxel| {
      match voxel {
        &voxel::Volume(voxel::Material::Empty) => None,
        _ => Some(bounds),
      }
    }
  )
}

pub fn apply_client_update<UpdateGaia>(
  server: &Server,
  update_gaia: &mut UpdateGaia,
  update: protocol::ClientToServer,
) where
  UpdateGaia: FnMut(update_gaia::Message),
{
  stopwatch::time("apply_client_update", move || {
    match update {
      protocol::ClientToServer::Init(client_url) => {
        info!("Sending to {}.", client_url);

        let mut client =
          Client {
            socket: SendSocket::new(client_url.as_ref(), Some(Duration::from_secs(30))),
          };

        let client_id = server.client_allocator.lock().unwrap().allocate();
        client.send(protocol::ServerToClient::LeaseId(client_id));

        server.clients.lock().unwrap().insert(client_id, client);
      },
      protocol::ClientToServer::Ping(client_id) => {
        server.clients.lock().unwrap()
          .get_mut(&client_id)
          .unwrap()
          .send(protocol::ServerToClient::Ping);
      },
      protocol::ClientToServer::AddPlayer(client_id) => {
        let mut player =
          Player::new(
            server.id_allocator.lock().unwrap().allocate(),
            &server.owner_allocator,
          );

        // TODO: shift upward until outside terrain
        let min = Point3::new(0.0, 64.0, 4.0);
        let max = min.add_v(&Vector3::new(1.0, 2.0, 1.0));
        let bounds = Aabb3::new(min, max);
        server.physics.lock().unwrap().insert_misc(player.entity_id, bounds.clone());

        player.position = center(&bounds);
        player.rotate_lateral(PI / 2.0);

        let id = player.entity_id;
        let pos = player.position;

        server.players.lock().unwrap().insert(id, player);

        let mut clients = server.clients.lock().unwrap();
        let client = clients.get_mut(&client_id).unwrap();
        client.send(
          protocol::ServerToClient::PlayerAdded(id, pos)
        );
      },
      protocol::ClientToServer::StartJump(player_id) => {
        let mut players = server.players.lock().unwrap();
        let player = players.get_mut(&player_id).unwrap();
        if !player.is_jumping {
          player.is_jumping = true;
          // this 0.3 is duplicated in a few places
          player.accel.y = player.accel.y + 0.3;
        }
      },
      protocol::ClientToServer::StopJump(player_id) => {
        let mut players = server.players.lock().unwrap();
        let player = players.get_mut(&player_id).unwrap();
        if player.is_jumping {
          player.is_jumping = false;
          // this 0.3 is duplicated in a few places
          player.accel.y = player.accel.y - 0.3;
        }
      },
      protocol::ClientToServer::Walk(player_id, v) => {
        let mut players = server.players.lock().unwrap();
        let mut player = players.get_mut(&player_id).unwrap();
        player.walk(v);
      },
      protocol::ClientToServer::RotatePlayer(player_id, v) => {
        let mut players = server.players.lock().unwrap();
        let mut player = players.get_mut(&player_id).unwrap();
        player.rotate_lateral(v.x);
        player.rotate_vertical(v.y);
      },
      protocol::ClientToServer::RequestVoxel(client_id, position) => {
        update_gaia(update_gaia::Message::Load(position, LoadReason::ForClient(client_id)));
      },
      protocol::ClientToServer::Add(player_id) => {
        let bounds = cast(server, player_id);

        bounds.map(|bounds| {
          let mut rng = server.rng.lock().unwrap();
          let rng = rng.deref_mut();

          let trunk_radius =
            rand::distributions::normal::Normal::new(2.0, 0.5)
            .ind_sample(rng);
          let trunk_radius =
            f64::max(1.0, f64::min(3.0, trunk_radius));

          let trunk_height =
            rand::distributions::normal::Normal::new(8.0 * trunk_radius, 2.0 * trunk_radius)
            .ind_sample(rng);
          let trunk_height =
            f64::max(4.0 * trunk_radius, f64::min(12.0 * trunk_radius, trunk_height));

          let leaf_radius =
            rand::distributions::normal::Normal::new(4.0 * trunk_radius, trunk_radius)
            .ind_sample(rng);
          let leaf_radius =
            f64::max(2.0 * trunk_radius, f64::min(6.0 * trunk_radius, leaf_radius));

          let (low, high) = bounds.corners();
          let mut bottom = low.add_v(&high.to_vec()).div_s(2.0);
          bottom.y = low.y;

          let trunk_height = trunk_height as f32;
          let trunk_radius = trunk_radius as f32;
          let leaf_radius = leaf_radius as f32;

          let tree =
            voxel_data::mosaic::translation::T {
              translation: bottom.to_vec(),
              mosaic: terrain::tree::new(rng, trunk_height, trunk_radius, leaf_radius),
            };

          let center =
            bottom.add_v(&Vector3::new(0.0, trunk_height / 2.0, 0.0));
          let r = trunk_height / 2.0 + leaf_radius + 20.0;
          let brush =
            voxel_data::brush::T {
              bounds:
                Aabb3::new(
                  {
                    let low = center.add_v(&-Vector3::new(r, r, r));
                    Point3::new(low.x.floor() as i32, low.y.floor() as i32, low.z.floor() as i32)
                  },
                  {
                    let high = center.add_v(&Vector3::new(r, r, r));
                    Point3::new(high.x.ceil() as i32, high.y.ceil() as i32, high.z.ceil() as i32)
                  },
                ),
              mosaic: Box::new(tree) as Box<voxel_data::mosaic::T<voxel::Material> + Send>,
              min_lg_size: 0,
            };

          update_gaia(update_gaia::Message::Brush(brush));
        });
      },
      protocol::ClientToServer::Remove(player_id) => {
        let bounds = cast(server, player_id);

        bounds.map(|bounds| {
          debug!("bounds {:?}", bounds);
          let center = bounds.center();
          let r = 8.0;
          let sphere =
            voxel_data::mosaic::solid::T {
              field: voxel_data::field::translation::T {
                translation: center.to_vec(),
                field: voxel_data::field::sphere::T {
                  radius: r,
                },
              },
              material: voxel::Material::Empty,
            };
          let r = sphere.field.field.radius + 1.0;
          let brush =
            voxel_data::brush::T {
              bounds:
                Aabb3::new(
                  {
                    let low = sphere.field.translation.add_v(&-Vector3::new(r, r, r));
                    Point3::new(low.x.floor() as i32, low.y.floor() as i32, low.z.floor() as i32)
                  },
                  {
                    let high = sphere.field.translation.add_v(&Vector3::new(r, r, r));
                    Point3::new(high.x.ceil() as i32, high.y.ceil() as i32, high.z.ceil() as i32)
                  },
                ),
              mosaic: Box::new(sphere) as Box<voxel_data::mosaic::T<voxel::Material> + Send>,
              min_lg_size: 0,
            };
          let brush: voxel_data::brush::T<Box<voxel_data::mosaic::T<voxel::Material> + Send>> = brush;
          update_gaia(update_gaia::Message::Brush(brush));
        });
      },
    };
  })
}
