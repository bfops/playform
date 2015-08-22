use cgmath::{Aabb3, Point, Point3, Vector, Vector3};
use std::collections::hash_map;
use std::collections::HashMap;
use std::f32;
use std::sync::Mutex;
use stopwatch::TimerSet;

use common::block_position::BlockPosition;
use common::entity::EntityId;
use common::id_allocator::IdAllocator;
use common::lod::LODIndex;
use common::terrain_block;
use common::terrain_block::{TerrainBlock, tri};

use terrain::heightmap;
use terrain;
use voxel;

pub fn generate_voxel(
  timers: &TimerSet,
  heightmap: &heightmap::T,
  voxel: &::voxel::Bounds,
) -> terrain::voxel::T
{
  timers.time("generate_voxel", || {
    let (low, high) = voxel.corners();
    macro_rules! density(($x:expr, $y:expr, $z:expr) => {{
      voxel::field::T::density_at(heightmap, &Point3::new($x.x, $y.y, $z.z))
    }});
    // corners[x][y][z]
    let mut corners = [
      [
        [ density!( low,  low, low), density!( low,  low, high) ],
        [ density!( low, high, low), density!( low, high, high) ],
      ],
      [
        [ density!(high,  low, low), density!(high,  low, high) ],
        [ density!(high, high, low), density!(high, high, high) ],
      ],
    ];

    let corner;
    let mut any_inside = false;
    let mut all_inside = true;

    {
      let mut get_corner = |x1:usize, y1:usize, z1:usize| {
        let corner = corners[x1][y1][z1] >= 0.0;
        corners[x1][y1][z1] = 1.0 / (corners[x1][y1][z1].abs() + f32::EPSILON);
        any_inside = any_inside || corner;
        all_inside = all_inside && corner;
        corner
      };

      corner = get_corner(0,0,0);
      let _ = get_corner(0,0,1);
      let _ = get_corner(0,1,0);
      let _ = get_corner(0,1,1);
      let _ = get_corner(1,0,0);
      let _ = get_corner(1,0,1);
      let _ = get_corner(1,1,0);
      let _ = get_corner(1,1,1);
    }

    let all_corners_same = any_inside == all_inside;
    if all_corners_same {
      return terrain::voxel::T::Volume(all_inside)
    }

    let corner_coords = [0.0, 256.0];
    let mut total_weight = 0.0;
    macro_rules! weighted(($x:expr, $y:expr, $z:expr) => {{
      total_weight += corners[$x][$y][$z];
      Vector3::new(corner_coords[$x], corner_coords[$y], corner_coords[$z])
        .mul_s(corners[$x][$y][$z])
    }});

    let vertex =
      weighted!(0, 0, 0) +
      weighted!(0, 0, 1) +
      weighted!(0, 1, 0) +
      weighted!(0, 1, 1) +
      weighted!(1, 0, 0) +
      weighted!(1, 0, 1) +
      weighted!(1, 1, 0) +
      weighted!(1, 1, 1) +
      Vector3::new(0.0, 0.0, 0.0);
    let vertex = Point3::from_vec(&vertex.div_s(total_weight));
    let vertex =
      terrain::voxel::Vertex {
        x: terrain::voxel::Fracu8::of(f32::min(vertex.x, 255.0) as u8),
        y: terrain::voxel::Fracu8::of(f32::min(vertex.y, 255.0) as u8),
        z: terrain::voxel::Fracu8::of(f32::min(vertex.z, 255.0) as u8),
      };

    let normal;
    {
      // Okay, this is silly to have right after we construct the vertex.
      let vertex = vertex.to_world_vertex(voxel);
      normal = terrain::voxel::Normal::of_float_normal(&voxel::field::T::normal_at(heightmap, 0.01, &vertex));
    }

    terrain::voxel::T::Surface(terrain::voxel::SurfaceStruct {
      inner_vertex: vertex,
      normal: normal,
      corner_inside_surface: corner,
    })
  })
}

#[inline]
fn partial_min<I, It>(mut it: It) -> Option<I>
where
  I: PartialOrd,
  It: Iterator<Item=I>,
{
  match it.next() {
    None => None,
    Some(mut best) => {
      for thing in it {
        if thing < best {
          best = thing;
        }
      }
      Some(best)
    },
  }
}

#[inline]
fn partial_max<I, It>(mut it: It) -> Option<I>
where
  I: PartialOrd,
  It: Iterator<Item=I>,
{
  match it.next() {
    None => None,
    Some(mut best) => {
      for thing in it {
        if thing > best {
          best = thing;
        }
      }
      Some(best)
    },
  }
}

fn make_bounds(
  v1: &Point3<f32>,
  v2: &Point3<f32>,
  v3: &Point3<f32>,
) -> Aabb3<f32> {
  let minx = *partial_min([v1.x, v2.x, v3.x].iter()).unwrap();
  let maxx = *partial_max([v1.x, v2.x, v3.x].iter()).unwrap();

  let miny = *partial_min([v1.y, v2.y, v3.y].iter()).unwrap();
  let maxy = *partial_max([v1.y, v2.y, v3.y].iter()).unwrap();

  let minz = *partial_min([v1.z, v2.z, v3.z].iter()).unwrap();
  let maxz = *partial_max([v1.z, v2.z, v3.z].iter()).unwrap();

  Aabb3::new(
    // TODO: Remove this - 1.0. It's a temporary hack until voxel collisions work,
    // to avoid zero-height Aabb3s.
    Point3::new(minx, miny - 1.0, minz),
    Point3::new(maxx, maxy, maxz),
  )
}

/// Generate a `TerrainBlock` based on a given position in a `terrain::voxel::tree::T`.
/// Any necessary voxels will be generated.
pub fn generate_block(
  timers: &TimerSet,
  id_allocator: &Mutex<IdAllocator<EntityId>>,
  heightmap: &heightmap::T,
  voxels: &mut terrain::voxel::tree::T,
  position: &BlockPosition,
  lod_index: LODIndex,
) -> TerrainBlock {
  timers.time("update.generate_block", || {
    let mut block = TerrainBlock::empty();

    let lateral_samples = terrain_block::EDGE_SAMPLES[lod_index.0 as usize] as i32;
    let lg_size = terrain_block::LG_SAMPLE_SIZE[lod_index.0 as usize] as i16;

    let bounds_at = |v: &Point3<i32>| {
      voxel::Bounds::new(v.x, v.y, v.z, lg_size)
    };

    let mut get_voxel = |bounds: &voxel::Bounds| {
      let branch = voxels.get_mut_or_create(bounds);
      let r;
      match branch {
        &mut terrain::voxel::tree::Empty => {
          r = generate_voxel(timers, heightmap, bounds);
          *branch =
            terrain::voxel::tree::Branch {
              data: Some(r),
              branches: Box::new(terrain::voxel::tree::Branches::empty()),
            };
        },
        &mut terrain::voxel::tree::Branch { ref mut data, branches: _ }  => {
          match data {
            &mut None => {
              r = generate_voxel(timers, heightmap, bounds);
              *data = Some(r);
            },
            &mut Some(ref data) => {
              r = *data;
            },
          }
        },
      };
      r
    };

    let lg_ratio = terrain_block::LG_WIDTH - lg_size;
    let block_position = position.as_pnt();
    let voxel_position =
      Point3::new(
        block_position.x << lg_ratio,
        block_position.y << lg_ratio,
        block_position.z << lg_ratio,
      );

    let mut coords = Vec::new();
    let mut normals = Vec::new();
    let mut indices = HashMap::new();
    let mut polys = Vec::new();

    for dx in 0..lateral_samples {
    for dy in 0..lateral_samples {
    for dz in 0..lateral_samples {
      let voxel_position = voxel_position.add_v(&Vector3::new(dx, dy, dz));
      let voxel;
      let bounds = bounds_at(&voxel_position);
      match get_voxel(&bounds) {
        terrain::voxel::T::Surface(v) => voxel = v,
        _ => continue,
      }
      let index = coords.len();
      let vertex = voxel.inner_vertex.to_world_vertex(&bounds);
      let normal = voxel.normal.to_float_normal();
      coords.push(vertex);
      normals.push(normal);
      indices.insert(voxel_position, index);

      let mut edge = |
        d_neighbor, // Vector to the neighbor to make an edge toward.
        d1, d2,     // Vector to the voxels adjacent to the edge.
      | {
        let neighbor_position = voxel_position.add_v(&d_neighbor);
        debug!("edge from {:?} to {:?}", voxel_position, neighbor_position);
        let neighbor_inside_surface;
        match get_voxel(&bounds_at(&neighbor_position)) {
          terrain::voxel::T::Surface(v) => neighbor_inside_surface = v.corner_inside_surface,
          terrain::voxel::T::Volume(inside) => neighbor_inside_surface = inside,
        }
        if voxel.corner_inside_surface == neighbor_inside_surface {
          // This edge doesn't cross the surface, and doesn't generate polys.

          return
        }

        let v1; let v2; let v3; let v4;
        let n1; let n2; let n3; let n4;
        let i1; let i2; let i3; let i4;

        {
          let mut voxel_index = |position: &Point3<i32>| {
            match indices.entry(*position) {
              hash_map::Entry::Occupied(entry) => {
                let i = *entry.get();
                (coords[i], normals[i], i)
              },
              hash_map::Entry::Vacant(entry) => {
                let bounds = bounds_at(position);
                match get_voxel(&bounds) {
                  terrain::voxel::T::Surface(voxel) => {
                    let i = coords.len();
                    let vertex = voxel.inner_vertex.to_world_vertex(&bounds);
                    let normal = voxel.normal.to_float_normal();
                    coords.push(vertex);
                    normals.push(normal);
                    entry.insert(i);
                    (vertex, normal, i)
                  },
                  voxel => {
                    panic!("Unexpected neighbor {:?} at {:?}", voxel, bounds)
                  }
                }
              },
            }
          };

          let (tv1, tn1, ti1) = voxel_index(&voxel_position.add_v(&d1).add_v(&d2));
          let (tv2, tn2, ti2) = voxel_index(&voxel_position.add_v(&d1));
          let (tv3, tn3, ti3) = (vertex, normal, index);
          let (tv4, tn4, ti4) = voxel_index(&voxel_position.add_v(&d2));

          v1 = tv1; v2 = tv2; v3 = tv3; v4 = tv4;
          n1 = tn1; n2 = tn2; n3 = tn3; n4 = tn4;
          i1 = ti1; i2 = ti2; i3 = ti3; i4 = ti4;
        }

        // Put a vertex at the average of the vertices.
        let v_center =
          v1.add_v(&v2.to_vec()).add_v(&v3.to_vec()).add_v(&v4.to_vec()).div_s(4.0);
        let n_center =
          n1.add_v(&n2).add_v(&n3).add_v(&n4).div_s(4.0);

        let i_center = coords.len();
        coords.push(v_center);
        normals.push(n_center);

        if voxel.corner_inside_surface {
          // The polys are visible from positive infinity.
          polys.push([i2, i1, i_center]);
          polys.push([i3, i2, i_center]);
          polys.push([i4, i3, i_center]);
          polys.push([i1, i4, i_center]);
        } else {
          // The polys are visible from negative infinity.
          polys.push([i1, i2, i_center]);
          polys.push([i2, i3, i_center]);
          polys.push([i3, i4, i_center]);
          polys.push([i4, i1, i_center]);
        }
      };

      edge(
        Vector3::new(1, 0, 0),
        Vector3::new(0, -1, 0),
        Vector3::new(0, 0, -1),
      );

      edge(
        Vector3::new(0, 1, 0),
        Vector3::new(0, 0, -1),
        Vector3::new(-1, 0, 0),
      );

      edge(
        Vector3::new(0, 0, 1),
        Vector3::new(-1, 0, 0),
        Vector3::new(0, -1, 0),
      );
    }}}

    for poly in polys.iter() {
      block.vertex_coordinates.push(tri(coords[poly[0]], coords[poly[1]], coords[poly[2]]));
      block.normals.push(tri(normals[poly[0]], normals[poly[1]], normals[poly[2]]));

      let id = id_allocator.lock().unwrap().allocate();
      block.ids.push(id);
      block.bounds.push((id, make_bounds(&coords[poly[0]], &coords[poly[1]], &coords[poly[2]])));
    }

    block
  })
}
