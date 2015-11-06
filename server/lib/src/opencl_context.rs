use opencl::hl;
use opencl::hl::{Device, Context, CommandQueue};

pub struct CL {
  pub device: Device,
  pub context: Context,
  pub queue: CommandQueue,
}

impl CL {
  pub unsafe fn new() -> CL {
    for platform in hl::get_platforms().iter() {
      debug!("Found OpenCL platform: {}", platform.name());
      debug!("Available devices:");
      let devices = platform.get_devices();
      for device in devices.into_iter() {
        debug!("  {}", device.name());
        let context = device.create_context();
        let queue = context.create_command_queue(&device);

        return CL {
          device: device,
          context: context,
          queue: queue,
        };
      }
    }

    panic!("Couldn't find an OpenCL device.");
  }
}
