use cgmath::{Point, Point3, Vector, Vector3};
use cgmath::Aabb3;
use std::cmp::{min, max, partial_min, partial_max};
use std::num::Int;
use std::sync::Mutex;

use common::block_position::BlockPosition;
use common::entity::EntityId;
use common::id_allocator::IdAllocator;
use common::lod::LODIndex;
use common::stopwatch::TimerSet;
use common::terrain_block::{TerrainBlock, BLOCK_WIDTH, LOD_QUALITY, tri};

use terrain::heightmap::HeightMap;
use terrain::terrain::{Fracu8, Fraci8, Voxel, VoxelVertex, VoxelNormal, Edge};
use voxel_tree;
use voxel_tree::{VoxelBounds, VoxelTree};

fn generate_voxel<FieldContains, GetNormal>(
  timers: &TimerSet,
  field_contains: &mut FieldContains,
  get_normal: &mut GetNormal,
  voxel: VoxelBounds,
) -> Option<Voxel>
  where
    FieldContains: FnMut(f32, f32, f32) -> bool,
    GetNormal: FnMut(f32, f32, f32) -> Vector3<f32>,
{
  timers.time("generate_voxel", || {
    let size = voxel.size();
    let (x1, y1, z1) = (voxel.x as f32 * size, voxel.y as f32 * size, voxel.z as f32 * size);
    let delta = size;
    let (x2, y2, z2) = (x1 + delta, y1 + delta, z1 + delta);
    // corners[x][y][z]
    let corners = [
      [
        [ field_contains(x1, y1, z1), field_contains(x1, y1, z2) ],
        [ field_contains(x1, y2, z1), field_contains(x1, y2, z2) ],
      ],
      [
        [ field_contains(x2, y1, z1), field_contains(x2, y1, z2) ],
        [ field_contains(x2, y2, z1), field_contains(x2, y2, z2) ],
      ],
    ];

    let x_edges;
    let y_edges;
    let z_edges;
    let mut any_crossed = false;

    {
      let mut edge = |x1:usize, y1:usize, z1:usize, x2:usize, y2:usize, z2:usize| {
        let is_crossed = corners[x1][y1][z1] != corners[x2][y2][z2];
        any_crossed = any_crossed || is_crossed;
        Edge {
          is_crossed: is_crossed,
          direction: corners[x2][y2][z2],
        }
      };

      x_edges = [
        [ edge(0,0,0, 1,0,0), edge(0,0,1, 1,0,1) ],
        [ edge(0,1,0, 1,1,0), edge(0,1,1, 1,1,1) ],
      ];
      y_edges = [
        [ edge(0,0,0, 0,1,0), edge(1,0,0, 1,1,0) ],
        [ edge(0,0,1, 0,1,1), edge(1,0,1, 1,1,1) ],
      ];
      z_edges = [
        [ edge(0,0,0, 0,0,1), edge(1,0,0, 1,0,1) ],
        [ edge(0,1,0, 0,1,1), edge(1,1,0, 1,1,1) ],
      ];
    }

    if !any_crossed {
      return None
    }

    let mut vertex: Vector3<u32> = Vector3::new(0, 0, 0);
    let mut n = 0;
    for (&x, corners) in [0, 0xFF].iter().zip(corners.iter()) {
    for (&y, corners) in [0, 0xFF].iter().zip(corners.iter()) {
    for (&z, &corner) in [0, 0xFF].iter().zip(corners.iter()) {
      if corner {
        vertex.add_self_v(&Vector3::new(x, y, z));
        n += 1;
      }
    }}}

    {
      // Sample in extra areas to help weight the vertex to the appropriate place.
      let mut sample_extra = |lg_s: u8| {
        let fs = 1.0 / ((1 << lg_s) as f32);
        let mfs = 1.0 - fs;
        let s = 0x100 >> lg_s;
        let ms = 0x100 - s;
        for &(wx, x) in
          [((voxel.x as f32 + fs) * size, s), ((voxel.x as f32 + mfs) * size, ms)].iter() {
        for &(wy, y) in
          [((voxel.y as f32 + fs) * size, s), ((voxel.y as f32 + mfs) * size, ms)].iter() {
        for &(wz, z) in
          [((voxel.z as f32 + fs) * size, s), ((voxel.z as f32 + mfs) * size, ms)].iter() {
          if field_contains(wx, wy, wz) {
            vertex.add_self_v(&Vector3::new(x, y, z).mul_s(lg_s as u32));
            n += lg_s as u32;
          }
        }}}
      };

      sample_extra(2);
    }

    let vertex = vertex.div_s(n);
    let vertex =
      VoxelVertex {
        x: Fracu8::of(vertex.x as u8),
        y: Fracu8::of(vertex.y as u8),
        z: Fracu8::of(vertex.z as u8),
      };

    let normal;
    {
      // Okay, this is silly to have right after we construct the vertex.
      let vertex = vertex.to_world_vertex(voxel);
      normal = get_normal(vertex.x, vertex.y, vertex.z).mul_s(127.0);
    }

    // Okay, so we scale the normal by 127, and use 127 to represent 1.0.
    // Then we store it in a `Fraci8`, which scales by 128 and represents a
    // fraction in [0,1). That seems wrong, but this is normal data, so scaling
    // doesn't matter. Sketch factor is over 9000, but it's not wrong.

    let normal = Vector3::new(normal.x as i32, normal.y as i32, normal.z as i32);
    let normal =
      Vector3::new(
        max(-127, min(127, normal.x)) as i8,
        max(-127, min(127, normal.y)) as i8,
        max(-127, min(127, normal.z)) as i8,
      );

    let normal =
      VoxelNormal {
        x: Fraci8::of(normal.x),
        y: Fraci8::of(normal.y),
        z: Fraci8::of(normal.z),
      };

    Some(Voxel {
      vertex: vertex,
      normal: normal,
      x_edges: x_edges,
      y_edges: y_edges,
      z_edges: z_edges,
    })
  })
}

pub fn generate_block(
  timers: &TimerSet,
  id_allocator: &Mutex<IdAllocator<EntityId>>,
  heightmap: &HeightMap,
  voxels: &mut VoxelTree<Voxel>,
  position: &BlockPosition,
  lod_index: LODIndex,
) -> TerrainBlock {
  timers.time("update.generate_block", || {
    let mut block = TerrainBlock::empty();

    let position = position.to_world_position();
    let iposition = Point3::new(position.x as i32, position.y as i32, position.z as i32);

    let lateral_samples = LOD_QUALITY[lod_index.0 as usize] as i32;
    assert!(
      BLOCK_WIDTH % lateral_samples == 0 || lateral_samples % BLOCK_WIDTH == 0,
      "Block width doesn't sample cleanly."
    );

    let lg_block_width = Int::trailing_zeros(BLOCK_WIDTH);
    assert!(1 << lg_block_width == BLOCK_WIDTH, "BLOCK_WIDTH should be an exponent of 2");

    let lg_lateral_samples = Int::trailing_zeros(lateral_samples);
    assert!(
      1 << lg_lateral_samples == lateral_samples,
      "lateral_samples should be an exponent of 2");

    let lg_size: i16 = lg_block_width as i16 - lg_lateral_samples as i16;
    assert!(lg_size.abs() < 16, "2^{} is too a huge voxel.", lg_size);
    // TODO: This is duplicated in src/voxel_tree.src. Fix that.
    let voxel_size;
    if lg_size >= 0 {
      voxel_size = (1 << lg_size) as f32;
    } else {
      voxel_size =  1.0 / (1 << -lg_size) as f32;
    }

    {
      let mut add_poly =
        |v1: Point3<f32>, n1, v2: Point3<f32>, n2, center: Point3<f32>, center_normal| {
          let id = id_allocator.lock().unwrap().allocate();

          let minx = partial_min(v1.x, v2.x);
          let minx = minx.and_then(|m| partial_min(m, center.x));
          let minx = minx.unwrap();

          let maxx = partial_max(v1.x, v2.x);
          let maxx = maxx.and_then(|m| partial_max(m, center.x));
          let maxx = maxx.unwrap();

          let miny = partial_min(v1.y, v2.y);
          let miny = miny.and_then(|m| partial_min(m, center.y));
          let miny = miny.unwrap();

          let maxy = partial_max(v1.y, v2.y);
          let maxy = maxy.and_then(|m| partial_max(m, center.y));
          let maxy = maxy.unwrap();

          let minz = partial_min(v1.z, v2.z);
          let minz = minz.and_then(|m| partial_min(m, center.z));
          let minz = minz.unwrap();

          let maxz = partial_max(v1.z, v2.z);
          let maxz = maxz.and_then(|m| partial_max(m, center.z));
          let maxz = maxz.unwrap();

          block.vertex_coordinates.push(tri(v1, v2, center));
          block.normals.push(tri(n1, n2, center_normal));
          block.ids.push(id);

          block.bounds.push((
            id,
            Aabb3::new(
              // TODO: Remove this - 1.0. It's a temporary hack until voxel collisions work.
              Point3::new(minx, miny - 1.0, minz),
              Point3::new(maxx, maxy, maxz),
            ),
          ));
        };

      let mut field_contains = |x, y, z| {
        heightmap.height_at(x, z) >= y
      };

      let mut get_normal = |x, _, z| {
        heightmap.normal_at(x, z, voxel_size as f32)
      };

      macro_rules! get_voxel(($bounds:expr) => {{
        let branch = voxels.get_mut($bounds);
        match branch {
          &mut voxel_tree::TreeBody::Leaf(v) => Some(v),
          &mut voxel_tree::TreeBody::Empty => {
            // TODO: Add a "yes this is empty I checked" flag so we don't regen every time.
            generate_voxel(timers, &mut field_contains, &mut get_normal, $bounds).map(|v| {
              *branch = voxel_tree::TreeBody::Leaf(v);
              v
            })
          },
          &mut voxel_tree::TreeBody::Branch(_) => {
            // Overwrite existing for now.
            // TODO: Don't do ^that.
            generate_voxel(timers, &mut field_contains, &mut get_normal, $bounds).map(|v| {
              *branch = voxel_tree::TreeBody::Leaf(v);
              v
            })
          },
        }
      }});

      macro_rules! get_vertex(($v:expr) => {{
        let bounds = VoxelBounds::new($v.x, $v.y, $v.z, lg_size);
        let voxel =
          get_voxel!(bounds)
          .unwrap_or_else(|| panic!("Couldn't find edge neighbor voxel"));
        (voxel.vertex.to_world_vertex(bounds), voxel.normal.to_world_normal())
      }});

      macro_rules! extract((
        $edges:ident,
        $facing:expr,
        $d1:expr,
        $d2:expr,
      ) => (
          for x in 0..lateral_samples {
          for y in 0..lateral_samples {
          for z in 0..lateral_samples {
            let w;
            {
              let iposition =
                if lg_size >= 0 {
                  let mask = (1 << lg_size) - 1;
                  assert!(
                    (iposition.x|iposition.y|iposition.z) & mask == 0,
                    "Block position should be a multiple of voxel sizes."
                  );
                  Point3::new(
                    iposition.x >> lg_size,
                    iposition.y >> lg_size,
                    iposition.z >> lg_size,
                  )
                } else {
                  let lg_size = -lg_size;
                  Point3::new(
                    iposition.x << lg_size,
                    iposition.y << lg_size,
                    iposition.z << lg_size,
                  )
                };
              w = iposition.add_v(&Vector3::new(x, y, z));
            }
            let bounds = VoxelBounds::new(w.x, w.y, w.z, lg_size);
            let voxel;
            match get_voxel!(bounds) {
              None => continue,
              Some(v) => voxel = v,
            }

            let edge = voxel.$edges[0][0];
            if !edge.is_crossed {
              continue
            }

            // Make a quad out of the vertices from the 4 voxels adjacent to this edge.
            // We know they have vertices in them because if the surface crosses an edge,
            // it must cross that edge's neighbors.

            let (v1, n1) = get_vertex!(w.add_v(&$d1).add_v(&$d2));
            let (v2, n2) = get_vertex!(w.add_v(&$d1));
            let v3 = voxel.vertex.to_world_vertex(bounds);
            let n3 = voxel.normal.to_world_normal();
            let (v4, n4) = get_vertex!(w.add_v(&$d2));

            // Put a vertex at the average of the vertices.
            let center =
              v1.add_v(&v2.to_vec()).add_v(&v3.to_vec()).add_v(&v4.to_vec()).div_s(4.0);
            let center_normal = n1.add_v(&n2).add_v(&n3).add_v(&n4).div_s(4.0);

            let mut add_poly = |v1, n1, v2, n2| add_poly(v1, n1, v2, n2, center, center_normal);

            if edge.direction {
              // The polys are visible from negative infinity.
              add_poly(v1, n1, v2, n2);
              add_poly(v2, n2, v3, n3);
              add_poly(v3, n3, v4, n4);
              add_poly(v4, n4, v1, n1);
            } else {
              // The polys are visible from positive infinity.
              add_poly(v1, n1, v4, n4);
              add_poly(v4, n4, v3, n3);
              add_poly(v3, n3, v2, n2);
              add_poly(v2, n2, v1, n1);
            }
          }}}
        )
      );

      extract!(
        x_edges, 0,
        Vector3::new(0, -1, 0),
        Vector3::new(0, 0, -1),
      );

      extract!(
        y_edges, 1,
        Vector3::new(0, 0, -1),
        Vector3::new(-1, 0, 0),
      );

      extract!(
        z_edges, 2,
        Vector3::new(-1, 0, 0),
        Vector3::new(0, -1, 0),
      );
    }

    block
  })
}
