use opencl::hl::{Device, Context, CommandQueue};
use opencl::util::create_compute_context;

pub struct CL {
  pub device: Device,
  pub context: Context,
  pub queue: CommandQueue,
}

impl CL {
  pub unsafe fn new() -> CL {
    let (device, context, queue) = create_compute_context().unwrap();
    CL {
      device: device,
      context: context,
      queue: queue,
    }
  }
}
