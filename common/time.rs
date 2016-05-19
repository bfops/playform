use time;

static mut conversion_ratio: Option<u64> = None;

#[derive(Debug)]
pub struct AlreadyInitialized;

pub fn init() -> Result<(), AlreadyInitialized> {

}

#[derive(Debug)]
pub struct NotInitialized;

pub fn now() -> Result<u64, NotInitialized> {

}
