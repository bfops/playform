use color::Color3;
use opencl;
use opencl::hl::{Program, Kernel};
use opencl::mem::CLBuffer;
use opencl_context::CL;

pub const TEXTURE_WIDTH: usize = 16;
pub const TEXTURE_LEN: usize = TEXTURE_WIDTH * TEXTURE_WIDTH;

const INCLUDE_PERLIN: &'static str = "
  double perlin(
    double amplitude,
    double frequency,
    const double persistence,
    const double lacunarity,
    const uint octaves,
    
    const double x,
    const double y)
  {{
    double r = 0.0;
    double max_crest = 0.0;

    for (int o = 0; o < octaves; ++o) {{
      double f = frequency * 2 * 3.14;
      r += amplitude * (sin((o+x) * f) + sin((o+y) * f)) / 2;

      max_crest += amplitude;
      amplitude *= persistence;
      frequency *= lacunarity;
    }}

    // Scale to [-1, 1]. N.B. There is no clamping.
    r /= max_crest;

    return r;
  }}
";

pub struct TerrainTextureGenerator {
  output: CLBuffer<Color3<f32>>,
  _program: Program,
  kernel: Kernel,
}

impl TerrainTextureGenerator {
  pub fn new(cl: &CL, width: u32) -> TerrainTextureGenerator {
    let output = cl.context.create_buffer(TEXTURE_LEN, opencl::cl::CL_MEM_WRITE_ONLY);

    let program = {
      let ker =
        format!("{} // includes
          __kernel void color(
            const float low_x,
            const float low_z,

            __global float* output)
          {{
            int W = {};
            int w = {};
            int i = get_global_id(0);

            double c_x = i % W;
            double c_y = i / W;
            c_x = c_x*w/W + low_x;
            c_y = c_y*w/W + low_z;

            double r = perlin({}, {}, {}, {}, {}, c_x, c_y);

            // shift, scale, clamp to [0, 1]
            r = (r + 1) / 2;
            r = fmin(fmax(r, 0), 1);

            i = i * 3;
            output[i+0] = (1 - r) / 2;
            output[i+1] = (3/2 + r) / 5;
            output[i+2] = (1 - r) / 5;
          }}
        ",
          INCLUDE_PERLIN,
          TEXTURE_WIDTH,
          width,
          1.0, 1.0 / 32.0, 0.8, 2.4, 2,
        );
      cl.context.create_program_from_source(ker.as_slice())
    };
    program.build(&cl.device).unwrap();

    let kernel = program.create_kernel("color");

    TerrainTextureGenerator {
      output: output,
      _program: program,
      kernel: kernel,
    }
  }

  pub fn generate(&self, cl: &CL, low_x: f32, low_z: f32) -> Vec<Color3<f32>> {
    self.kernel.set_arg(0, &low_x);
    self.kernel.set_arg(1, &low_z);

    // This is sketchy; we "implicitly cast" output from a
    // `CLBuffer<Color3<f32>>` to a `CLBuffer<f32>`.
    self.kernel.set_arg(2, &self.output);

    let event =
      cl.queue.enqueue_async_kernel(&self.kernel, TEXTURE_LEN, None, ());
    cl.queue.get(&self.output, &event)
  }
}
