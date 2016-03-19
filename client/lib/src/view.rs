//! The state associated with perceiving the world state.

use cgmath;
use gl;
use gl::types::*;
use image;
use image::GenericImage;
use std;
use yaglw::gl_context::GLContext;
use yaglw;
use yaglw::vertex_buffer::{GLArray, GLBuffer, GLType, DrawMode, VertexAttribData};
use yaglw::texture::{TextureUnit};

use camera::Camera;
use common;
use common::id_allocator;
use light;
use grass_buffers;
use mob_buffers::MobBuffers;
use player_buffers::PlayerBuffers;
use shaders::Shaders;
use terrain_buffers::TerrainBuffers;
use vertex::{ColoredVertex};

pub const FOV: f32 = std::f32::consts::FRAC_PI_3;

const VERTICES_PER_TRIANGLE: usize = 3;

/// The state associated with perceiving the world state.
pub struct T<'a> {
  /// Current OpengL context.
  pub gl: GLContext,
  #[allow(missing_docs)]
  pub shaders: Shaders<'a>,

  #[allow(missing_docs)]
  pub terrain_buffers: TerrainBuffers<'a>,
  pub grass_buffers: grass_buffers::T<'a>,
  pub grass_texture: yaglw::texture::Texture2D<'a>,
  #[allow(missing_docs)]
  pub mob_buffers: MobBuffers<'a>,
  #[allow(missing_docs)]
  pub player_buffers: PlayerBuffers<'a>,
  /// Hud triangles for non-text.
  pub hud_triangles: GLArray<'a, ColoredVertex>,

  pub empty_gl_array: yaglw::vertex_buffer::ArrayHandle<'a>,

  /// A texture unit for misc use.
  pub misc_texture_unit: TextureUnit,

  #[allow(missing_docs)]
  pub sun: light::Sun,

  #[allow(missing_docs)]
  pub camera: Camera,

  #[allow(missing_docs)]
  pub show_hud: bool,

  pub window_size: cgmath::Vector2<i32>,
}

fn load_grass_texture<'a, 'b:'a>(
  gl: &'a mut GLContext,
) -> image::ImageResult<yaglw::texture::Texture2D<'b>> {
  let grass_texture = yaglw::texture::Texture2D::new(&gl);
  let fd = std::fs::File::open("Assets/Free_Vector_Grass.png").unwrap();
  let image = try!(image::load(fd, image::ImageFormat::PNG));
  let (w, h) = image.dimensions();
  let colortype = image.color();
  assert!(colortype == image::ColorType::RGBA(8));

  let mut pixels: Vec<u8> = Vec::with_capacity((w * h) as usize);
  for y in 0 .. h {
  for x in 0 .. w {
    let y = h - y - 1;
    let pixel = image.get_pixel(x, y);
    pixels.extend_from_slice(&pixel.data);
  }}

  unsafe {
    gl::BindTexture(gl::TEXTURE_2D, grass_texture.handle.gl_id);
    let data = std::mem::transmute(pixels.as_ptr());
    gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGBA as i32, w as i32, h as i32, 0, gl::RGBA, gl::UNSIGNED_BYTE, data);
    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
  }

  Ok(grass_texture)
}

impl<'a> T<'a> {
  #[allow(missing_docs)]
  pub fn new(mut gl: GLContext, window_size: cgmath::Vector2<i32>) -> T<'a> {
    let mut texture_unit_alloc = id_allocator::new();

    let mut shaders = Shaders::new(&mut gl, window_size);

    let terrain_buffers = TerrainBuffers::new(&mut gl);
    terrain_buffers.bind_glsl_uniforms(
      &mut gl,
      &mut texture_unit_alloc,
      &mut shaders.terrain_shader,
    );

    let mob_buffers = MobBuffers::new(&mut gl, &shaders.mob_shader);
    let player_buffers = PlayerBuffers::new(&mut gl, &shaders.mob_shader);

    let buffer = GLBuffer::new(&mut gl, 16 * VERTICES_PER_TRIANGLE);
    let hud_triangles = {
      GLArray::new(
        &mut gl,
        &shaders.hud_color_shader.shader,
        &[
          VertexAttribData { name: "position", size: 3, unit: GLType::Float, divisor: 0 },
          VertexAttribData { name: "in_color", size: 4, unit: GLType::Float, divisor: 0 },
        ],
        DrawMode::Triangles,
        buffer,
      )
    };

    let misc_texture_unit = texture_unit_alloc.allocate();

    unsafe {
      gl::FrontFace(gl::CCW);
      gl::CullFace(gl::BACK);
      gl::Enable(gl::CULL_FACE);

      gl::Enable(gl::BLEND);
      gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

      gl::Enable(gl::LINE_SMOOTH);
      gl::LineWidth(2.5);

      gl::Enable(gl::DEPTH_TEST);
      gl::DepthFunc(gl::LESS);
      gl::ClearDepth(1.0);
    }

    unsafe {
      gl::ActiveTexture(misc_texture_unit.gl_id());
    }

    let texture_in =
      shaders.texture_shader.shader.get_uniform_location("texture_in");
    shaders.texture_shader.shader.use_shader(&mut gl);
    unsafe {
      gl::Uniform1i(texture_in, misc_texture_unit.glsl_id as GLint);
    }

    let texture_in =
      shaders.grass_billboard.shader.get_uniform_location("texture_in");
    shaders.grass_billboard.shader.use_shader(&mut gl);
    unsafe {
      gl::Uniform1i(texture_in, misc_texture_unit.glsl_id as GLint);
    }

    let grass_buffers = grass_buffers::new(&mut gl, &shaders.grass_billboard.shader);
    let grass_texture = load_grass_texture(&mut gl).unwrap();

    let empty_gl_array = yaglw::vertex_buffer::ArrayHandle::new(&gl);

    T {
      gl: gl,
      shaders: shaders,

      terrain_buffers: terrain_buffers,
      grass_buffers: grass_buffers,
      grass_texture: grass_texture,
      mob_buffers: mob_buffers,
      player_buffers: player_buffers,
      hud_triangles: hud_triangles,

      empty_gl_array: empty_gl_array,
      misc_texture_unit: misc_texture_unit,

      window_size: window_size,

      camera: {
        let fovy = cgmath::rad(FOV);
        let aspect = window_size.x as f32 / window_size.y as f32;
        let mut camera = Camera::unit();
        // Initialize the projection matrix.
        camera.fov = cgmath::perspective(fovy, aspect, 0.1, 2048.0);
        // TODO: This should use player rotation from the server.
        camera.rotate_lateral(std::f32::consts::PI / 2.0);
        camera
      },

      sun:
        light::Sun {
          direction: cgmath::Vector3::new(0.0, 0.0, 0.0),
          intensity: common::color::Color3 { r: 0.0, g: 0.0, b: 0.0 },
        },

      show_hud: true,
    }
  }
}
